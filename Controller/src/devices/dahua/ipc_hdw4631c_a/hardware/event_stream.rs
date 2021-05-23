use super::api::Api;
use crate::util::atomic_cell::AtomicCell;
use anyhow::{anyhow, bail, ensure, Context, Error};
use futures::{
    future::FutureExt,
    pin_mut, select,
    stream::{StreamExt, TryStreamExt},
};
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use tokio::sync::watch;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Event {
    VideoBlind,
    SceneChange,
    VideoMotion { region: String },
    AudioMutation,
}

pub type Events = HashSet<Event>;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct EventStateUpdate {
    event: Event,
    active: bool,
}

pub struct Manager<'a> {
    api: &'a Api,

    events_active: AtomicCell<HashMap<Event, Instant>>, // {event: started}

    events_sender: watch::Sender<Events>,
    events_receiver: watch::Receiver<Events>,
}
impl<'a> Manager<'a> {
    const EVENT_DURATION_THRESHOLD: Duration = Duration::from_secs(60 * 60);
    const EVENT_FIXER_INTERVAL: Duration = Duration::from_secs(60);
    const ERROR_RESTART_DELAY: Duration = Duration::from_secs(1);

    pub fn new(api: &'a Api) -> Self {
        let events_active = HashMap::new();
        let events_active = AtomicCell::new(events_active);

        let (events_sender, events_receiver) = watch::channel(Events::new());

        Self {
            api,

            events_active,

            events_sender,
            events_receiver,
        }
    }

    pub fn receiver(&self) -> watch::Receiver<Events> {
        self.events_receiver.clone()
    }

    fn event_parse(
        code: &str,
        data: Option<serde_json::Value>,
    ) -> Result<Option<Event>, Error> {
        match code {
            "VideoBlind" => Ok(Some(Event::VideoBlind)),
            "AudioMutation" => Ok(Some(Event::AudioMutation)),
            "SceneChange" => Ok(Some(Event::SceneChange)),
            "VideoMotion" => {
                let data = match data {
                    Some(data) => data,
                    None => return Ok(None),
                };

                let data_object = data.as_object().ok_or_else(|| anyhow!("expected object"))?;

                let regions = data_object
                    .get("RegionName")
                    .ok_or_else(|| anyhow!("missing RegionName"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("expected array"))?;

                if regions.len() != 1 {
                    return Err(anyhow!("regions array size must be 1"));
                }
                ensure!(regions.len() == 1);

                let region = regions.get(0).unwrap();

                let region = region
                    .as_str()
                    .ok_or_else(|| anyhow!("expected string"))?
                    .to_owned();

                Ok(Some(Event::VideoMotion { region }))
            }
            code => {
                log::trace!("unrecognized event: {}", code);
                Ok(None)
            }
        }
    }
    fn event_state_update_parse(item: &str) -> Result<Option<EventStateUpdate>, Error> {
        lazy_static! {
            static ref PATTERN: Regex =
                RegexBuilder::new(r"^Code=(\w+);action=(\w+);index=0(;data=(.+))?$")
                    .dot_matches_new_line(true)
                    .build()
                    .unwrap();
        }

        let captures = PATTERN
            .captures(item)
            .ok_or_else(|| anyhow!("event item does not match required pattern"))?;

        let code = captures.get(1).unwrap().as_str();

        let data = match captures.get(4) {
            Some(data) => {
                let data = data.as_str();
                let data = serde_json::from_str::<serde_json::Value>(data).context("from_str")?;
                Some(data)
            }
            None => None,
        };

        let event = match Self::event_parse(code, data).context("event_parse")? {
            Some(event) => event,
            None => return Ok(None),
        };

        let active = match captures.get(2).unwrap().as_str() {
            "Start" => true,
            "Stop" => false,
            other => bail!("unrecognized action: {}", other),
        };

        Ok(Some(EventStateUpdate { event, active }))
    }
    fn event_state_update_handle(
        &self,
        event_time: Instant,
        event_state_update: EventStateUpdate,
    ) -> bool {
        let mut events_active = self.events_active.lease();
        if event_state_update.active {
            events_active
                .insert(event_state_update.event, event_time)
                .is_none()
        } else {
            events_active.remove(&event_state_update.event).is_some()
        }
    }
    fn events_fixer_handle(
        &self,
        now: Instant,
    ) -> bool {
        let fix_before = now - Self::EVENT_DURATION_THRESHOLD;

        let mut events_active = self.events_active.lease();
        events_active
            .drain_filter(|event, started| {
                if *started < fix_before {
                    log::warn!("removing outdated events: {:?}", event);
                    true
                } else {
                    false
                }
            })
            .count()
            > 0
    }

    fn events_propagate(&self) {
        let events = self
            .events_active
            .lease()
            .keys()
            .cloned()
            .collect::<Events>();

        self.events_sender.send(events).unwrap();
    }

    pub async fn run_once(&self) -> Result<!, Error> {
        let item_stream = self
            .api
            .http_request_boundary_stream(
                "/cgi-bin/eventManager.cgi?action=attach&codes=[All]"
                    .parse()
                    .unwrap(),
            )
            .await
            .context("http_request_boundary_stream")?;

        let item_stream_runner = item_stream
            .try_for_each(async move |item| -> Result<(), Error> {
                let event_state_update =
                    Self::event_state_update_parse(&item).context("event_state_update_parse")?;

                if let Some(event_state_update) = event_state_update {
                    let event_time = Instant::now();
                    if self.event_state_update_handle(event_time, event_state_update) {
                        self.events_propagate();
                    }
                }
                Ok(())
            })
            .map(|result| match result.context("item_stream_runner") {
                Ok(()) => anyhow!("item_stream completed"),
                Err(error) => error,
            });
        pin_mut!(item_stream_runner);
        let mut item_stream_runner = item_stream_runner.fuse();

        let events_fixer_runner = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(Self::EVENT_FIXER_INTERVAL),
        )
        .for_each(async move |time_point| {
            if self.events_fixer_handle(time_point.into_std()) {
                self.events_propagate();
            }
        });
        pin_mut!(events_fixer_runner);
        let mut events_fixer_runner = events_fixer_runner.fuse();

        select! {
            item_stream_runner_error = item_stream_runner => bail!(item_stream_runner_error),
            _ = events_fixer_runner => bail!("events_fixer_runner yielded"),
        }
    }
    pub async fn run(&self) -> ! {
        loop {
            let error = self.run_once().await.context("run_once");
            log::error!("event stream failed: {:?}", error);
            tokio::time::sleep(Self::ERROR_RESTART_DELAY).await;
        }
    }
}

#[cfg(test)]
mod tests_manager {
    use super::{Event, EventStateUpdate, Manager};
    use indoc::indoc;

    #[test]
    fn test_unsupported() {
        let event = indoc!(
            "
            Code=NTPAdjustTime;action=Pulse;index=0;data={
                \"Address\" : \"pool.ntp.org\",
                \"Before\" : \"2021-05-18 20:33:00\",
                \"result\" : true
            }
        "
        );

        let event_state_update = Manager::event_state_update_parse(event).unwrap();

        assert!(event_state_update.is_none());
    }

    #[test]
    fn test_single() {
        let event_state_update =
            Manager::event_state_update_parse("Code=AudioMutation;action=Stop;index=0").unwrap();

        assert_eq!(
            event_state_update,
            Some(EventStateUpdate {
                event: Event::AudioMutation,
                active: false
            })
        );
    }

    #[test]
    fn test_parametrized() {
        let event = indoc!(
            "
            Code=VideoMotion;action=Start;index=0;data={
                \"RegionName\" : [ \"MD1\" ]
            }
        "
        );

        let event_state_update = Manager::event_state_update_parse(event).unwrap();

        assert_eq!(
            event_state_update,
            Some(EventStateUpdate {
                event: Event::VideoMotion {
                    region: "MD1".to_owned()
                },
                active: true,
            })
        );
    }
}
