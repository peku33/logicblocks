use super::api::Api;
use crate::util::atomic_cell::AtomicCell;
use anyhow::{anyhow, bail, Context, Error};
use futures::{
    future::FutureExt,
    pin_mut, select,
    stream::{StreamExt, TryStreamExt},
};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Duration,
};
use tokio::sync::watch;
use xmltree::Element;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum Event {
    CameraFailure,
    VideoLoss,
    TamperingDetection,
    MotionDetection,
    LineDetection,
    FieldDetection,
}
#[derive(Debug)]
pub struct EventStateUpdate {
    event: Event,
    active: bool,
}
pub type Events = HashSet<Event>;

pub struct Manager<'a> {
    api: &'a Api,
    mixed_content_extractor: AtomicCell<MixedContentExtractor>,
    events_active: AtomicCell<HashMap<Event, usize>>, // Event -> Ticks left

    events_sender: watch::Sender<Events>,
    events_receiver: watch::Receiver<Events>,
}
impl<'a> Manager<'a> {
    const EVENT_STREAM_TIMEOUT: Duration = Duration::from_secs(1);
    const EVENT_DISABLE_TICK_INTERVAL: Duration = Duration::from_millis(250);
    const EVENT_DISABLE_TICKS: usize = 5; // 1250ms
    const ERROR_RESTART_DELAY: Duration = Duration::from_secs(1);

    pub fn new(api: &'a Api) -> Self {
        let (events_sender, events_receiver) = watch::channel(Events::new());

        let mixed_content_extractor = MixedContentExtractor::new();
        let mixed_content_extractor = AtomicCell::new(mixed_content_extractor);

        let events_active = HashMap::new();
        let events_active = AtomicCell::new(events_active);

        Self {
            api,
            mixed_content_extractor,
            events_active,

            events_sender,
            events_receiver,
        }
    }

    pub fn receiver(&self) -> watch::Receiver<Events> {
        self.events_receiver.clone()
    }

    fn parse_event_state_update(element: Element) -> Result<EventStateUpdate, Error> {
        let event_type = element
            .get_child("eventType")
            .ok_or_else(|| anyhow!("missing eventType"))?
            .get_text()
            .ok_or_else(|| anyhow!("missing eventType text"))?;

        let event_state = element
            .get_child("eventState")
            .ok_or_else(|| anyhow!("missing eventState"))?
            .get_text()
            .ok_or_else(|| anyhow!("missing eventState text"))?;

        let event = match event_type.as_ref() {
            "videoloss" => Event::VideoLoss,
            "shelteralarm" => Event::TamperingDetection,
            "VMD" => Event::MotionDetection,
            "linedetection" => Event::LineDetection,
            "fielddetection" => Event::FieldDetection,
            _ => bail!("unknown event type: {}", event_type),
        };
        let active = match event_state.as_ref() {
            "inactive" => false,
            "active" => true,
            _ => bail!("unknown event state: {}", event_state),
        };

        Ok(EventStateUpdate { event, active })
    }
    fn handle_event_state_update(
        &self,
        event_state_update: EventStateUpdate,
    ) -> bool {
        let mut events_active = self.events_active.lease();
        if event_state_update.active {
            events_active
                .insert(event_state_update.event, Self::EVENT_DISABLE_TICKS)
                .is_none()
        } else {
            events_active.remove(&event_state_update.event).is_some()
        }
    }
    fn handle_events_disabler(&self) -> bool {
        let mut events_active = self.events_active.lease();
        events_active
            .drain_filter(|_, ticks_left| {
                *ticks_left -= 1;
                *ticks_left == 0
            })
            .count()
            > 0
    }

    fn propagate_events(&self) {
        let events = self
            .events_active
            .lease()
            .keys()
            .cloned()
            .collect::<Events>();

        self.events_sender.send(events).unwrap();
    }

    pub async fn run_once(&self) -> Result<!, Error> {
        let data_stream = self
            .api
            .request_mixed_stream("/ISAPI/Event/notification/alertStream".parse().unwrap())
            .await
            .context("request_mixed_stream")?;

        // TODO: Add timeout
        let data_stream_runner = data_stream
            .err_into::<Error>()
            .try_for_each(async move |chunk| {
                let chunk = std::str::from_utf8(&chunk).context("from_utf8")?;
                let mut mixed_content_extractor = self.mixed_content_extractor.lease();
                mixed_content_extractor.push(chunk);

                let mut events_changed = false;
                for element in mixed_content_extractor.try_extract().into_vec().into_iter() {
                    let event_state_update = Self::parse_event_state_update(element)
                        .context("parse_event_state_update")?;
                    events_changed |= self.handle_event_state_update(event_state_update);
                }
                if events_changed {
                    self.propagate_events();
                }
                Ok(())
            })
            .map(|result| match result.context("data_stream_runner") {
                Ok(()) => anyhow!("data_stream completed"),
                Err(error) => error,
            });
        pin_mut!(data_stream_runner);
        let mut data_stream_runner = data_stream_runner.fuse();

        let events_disabler_runner = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(Self::EVENT_DISABLE_TICK_INTERVAL),
        )
        .for_each(async move |_time_point| {
            let mut events_changed = false;
            events_changed |= self.handle_events_disabler();
            if events_changed {
                self.propagate_events();
            }
        });
        pin_mut!(events_disabler_runner);
        let mut events_disabler_runner = events_disabler_runner.fuse();

        select! {
            data_stream_runner_error = data_stream_runner => bail!(data_stream_runner_error),
            _ = events_disabler_runner => bail!("events_disabler_runner"),
        }
    }
    pub async fn run(&self) -> ! {
        loop {
            let error = self.run_once().await.context("run_once");
            log::error!("device failed: {:?}", error);
            tokio::time::sleep(Self::ERROR_RESTART_DELAY).await;
        }
    }
}

struct MixedContentExtractor {
    buffer: VecDeque<u8>,
}
impl MixedContentExtractor {
    pub fn new() -> Self {
        let buffer = VecDeque::new();
        Self { buffer }
    }

    pub fn push(
        &mut self,
        chunk: &str,
    ) {
        self.buffer.extend(chunk.bytes());
    }

    pub fn try_extract(&mut self) -> Box<[Element]> {
        lazy_static! {
            static ref PATTERN: Regex = Regex::new("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: (\\d+)\r\n\r\n").unwrap();
        }

        let buffer = self.buffer.make_contiguous();
        let buffer = unsafe { std::str::from_utf8_unchecked(buffer) }; // SAFETY: buffer accepts &str only

        let mut elements = Vec::new();
        let mut start_index: usize = 0;
        while let Some(capture) = PATTERN.captures(&buffer[start_index..]) {
            let header = capture.get(0).unwrap();
            if header.start() != 0 {
                log::warn!("whole.start() != start_index, got some noise on input?");
            }

            let content_length = match capture
                .get(1)
                .unwrap()
                .as_str()
                .parse::<usize>()
                .context("content_length parse")
            {
                Ok(content_length) => content_length,
                Err(error) => {
                    log::warn!("failed to parse content_length: {:?}", error);

                    start_index += header.end(); // Skip header
                    continue;
                }
            };

            // Do we have whole message in buffer?
            if content_length - 1 > buffer.len() - start_index - header.end() {
                break;
            }

            let element = match Element::parse(
                (&buffer[start_index + header.end()..start_index + header.end() + content_length])
                    .as_bytes(),
            )
            .context("parse")
            {
                Ok(element) => element,
                Err(error) => {
                    log::warn!("failed to parse element: {:?}", error);

                    start_index += header.end() + content_length; // Skip payload
                    continue;
                }
            };

            elements.push(element);
            start_index += header.end() + content_length;
        }
        self.buffer.drain(0..start_index);

        elements.into_boxed_slice()
    }
}

#[cfg(test)]
pub mod mixed_content_extractor_tests {
    use super::MixedContentExtractor;

    #[test]
    fn test_1() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_2() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push(
            "--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478",
        );

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);

        extractor.push("\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_3() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);

        extractor.push("<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_4() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);

        extractor.push("\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_5() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 2);

        let result = extractor.try_extract();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_6() {
        let mut extractor = MixedContentExtractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);

        extractor.push("\r\n");

        let result = extractor.try_extract();
        assert_eq!(result.len(), 1);
    }
}
