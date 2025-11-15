/// In general camera events are fucked up.
/// For example if motion starts in region 1 and ends in region 2, the camera
/// will issue start for region 1 and stop for region 2 Even sometimes there
/// are two ending regions, which makes it even more useless.
use super::api::Api;
use anyhow::{Context, Error, anyhow, bail};
use atomic_refcell::AtomicRefCell;
use futures::{
    future::FutureExt,
    pin_mut, select,
    stream::{StreamExt, TryStreamExt},
};
use regex::{Regex, RegexBuilder};
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
    time::{Duration, Instant},
};
use tokio::sync::watch;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Event {
    VideoBlind,
    SceneChange,
    VideoMotion,
    AudioMutation,
    SmartMotionHuman,
    SmartMotionVehicle,
}

pub type Events = HashSet<Event>;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct EventStateUpdate {
    event: Event,
    active: bool,
}

#[derive(Debug)]
pub struct Manager<'a> {
    api: &'a Api,

    events_active: AtomicRefCell<HashMap<Event, Instant>>, // {event: started}

    events_sender: watch::Sender<Events>,
    events_receiver: watch::Receiver<Events>,
}
impl<'a> Manager<'a> {
    const EVENT_DURATION_THRESHOLD: Duration = Duration::from_secs(60 * 60);
    const EVENT_FIXER_INTERVAL: Duration = Duration::from_secs(60);
    const ERROR_RESTART_DELAY: Duration = Duration::from_secs(1);

    pub fn new(api: &'a Api) -> Self {
        let events_active = HashMap::<Event, Instant>::new();
        let events_active = AtomicRefCell::new(events_active);

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
        _data: Option<serde_json::Value>,
    ) -> Result<Option<Event>, Error> {
        match code {
            "VideoBlind" => Ok(Some(Event::VideoBlind)),
            "AudioMutation" => Ok(Some(Event::AudioMutation)),
            "SceneChange" => Ok(Some(Event::SceneChange)),
            "VideoMotion" => Ok(Some(Event::VideoMotion)),
            "SmartMotionHuman" => Ok(Some(Event::SmartMotionHuman)),
            "SmartMotionVehicle" => Ok(Some(Event::SmartMotionVehicle)),
            code => {
                log::debug!("unrecognized event: {code}");
                Ok(None)
            }
        }
    }
    fn event_state_update_parse(item: &str) -> Result<Option<EventStateUpdate>, Error> {
        static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            RegexBuilder::new(r"^Code=(\w+);action=(\w+);index=0(;data=(.+))?$")
                .dot_matches_new_line(true)
                .build()
                .unwrap()
        });

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
        let mut events_active = self.events_active.borrow_mut();

        let mut changed = false;

        if event_state_update.active {
            match events_active.insert(event_state_update.event, event_time) {
                None => {
                    changed = true;
                }
                Some(previous) => {
                    log::warn!("adding already added event: {previous:?} ({events_active:?})");
                }
            }
        } else {
            match events_active.remove(&event_state_update.event) {
                Some(_) => {
                    changed = true;
                }
                None => {
                    log::warn!(
                        "removing not added element: {:?} ({:?})",
                        event_state_update.event,
                        events_active
                    );
                }
            }
        }

        changed
    }
    fn events_fixer_handle(
        &self,
        now: Instant,
    ) -> bool {
        let fix_before = now - Self::EVENT_DURATION_THRESHOLD;

        self.events_active
            .borrow_mut()
            .extract_if(|event, started| {
                if *started < fix_before {
                    log::warn!("removing outdated events: {event:?}");
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
            .borrow()
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
            .try_for_each(async |item| {
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
            })
            .fuse();
        pin_mut!(item_stream_runner);

        let events_fixer_runner = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(Self::EVENT_FIXER_INTERVAL),
        )
        .for_each(async |time_point| {
            if self.events_fixer_handle(time_point.into_std()) {
                self.events_propagate();
            }
        })
        .fuse();
        pin_mut!(events_fixer_runner);

        select! {
            item_stream_runner_error = item_stream_runner => bail!(item_stream_runner_error),
            _ = events_fixer_runner => bail!("events_fixer_runner yielded"),
        }
    }
    pub async fn run(&self) -> ! {
        loop {
            let error = self.run_once().await.context("run_once");
            log::error!("event stream failed: {error:?}");
            tokio::time::sleep(Self::ERROR_RESTART_DELAY).await;
        }
    }
}
#[cfg(test)]
mod tests_manager {
    use super::{Event, EventStateUpdate, Manager};
    use indoc::indoc;

    #[test]
    fn unsupported() {
        let event = indoc!(
            r#"
            Code=NTPAdjustTime;action=Pulse;index=0;data={
                "Address" : "pool.ntp.org",
                "Before" : "2021-05-18 20:33:00",
                "result" : true
            }
        "#
        );

        let event_state_update = Manager::event_state_update_parse(event).unwrap();

        assert!(event_state_update.is_none());
    }

    #[test]
    fn single() {
        let event_state_update =
            Manager::event_state_update_parse("Code=AudioMutation;action=Stop;index=0").unwrap();

        let event_state_update_expected = EventStateUpdate {
            event: Event::AudioMutation,
            active: false,
        };

        assert_eq!(event_state_update, Some(event_state_update_expected));
    }

    #[test]
    fn video_motion_by_id() {
        let event = indoc!(
            r#"
                Code=VideoMotion;action=Start;index=0;data={
                    "Id" : [ 1 ],
                    "RegionName" : [ "Region2" ],
                    "SmartMotionEnable" : true
                }
            "#
        );

        let event_state_update = Manager::event_state_update_parse(event).unwrap();

        let event_state_update_expected = EventStateUpdate {
            event: Event::VideoMotion,
            active: true,
        };

        assert_eq!(event_state_update, Some(event_state_update_expected));
    }

    #[test]
    fn video_motion_by_name() {
        let event = indoc!(
            r#"
                Code=VideoMotion;action=Start;index=0;data={
                    "RegionName" : [ "MD1" ]
                }
            "#
        );

        let event_state_update = Manager::event_state_update_parse(event).unwrap();

        let event_state_update_expected = EventStateUpdate {
            event: Event::VideoMotion,
            active: true,
        };

        assert_eq!(event_state_update, Some(event_state_update_expected));
    }

    #[test]
    fn smart_motion_human() {
        let event = indoc!(
            r#"
                Code=SmartMotionHuman;action=Start;index=0;data={
                    "RegionName" : [ "Region2" ],
                    "WindowId" : [ 1 ],
                    "object" : [
                        {
                            "HumamID" : 17,
                            "Rect" : [ 4736, 5744, 6272, 8168 ]
                        }
                    ]
                }
            "#
        );

        let event_state_update = Manager::event_state_update_parse(event).unwrap();

        let event_state_update_expected = EventStateUpdate {
            event: Event::SmartMotionHuman,
            active: true,
        };

        assert_eq!(event_state_update, Some(event_state_update_expected));
    }

    #[test]
    fn smart_motion_vehicle() {
        let event = indoc!(
            r#"
            Code=SmartMotionVehicle;action=Stop;index=0;data={
                "RegionName" : [ "Motion Detection" ],
                "WindowId" : [ 0 ],
                "object" : [
                   {
                      "Rect" : [ 2608, 2624, 4360, 3952 ],
                      "VehicleID" : 15
                   }
                ]
            }
            "#
        );

        let event_state_update = Manager::event_state_update_parse(event).unwrap();

        let event_state_update_expected = EventStateUpdate {
            event: Event::SmartMotionVehicle,
            active: false,
        };

        assert_eq!(event_state_update, Some(event_state_update_expected));
    }

    #[test]
    fn video_motion_multiple() {
        let event = indoc!(
            r#"
            Code=VideoMotion;action=Stop;index=0;data={
                "Id" : [ 0, 1 ],
                "RegionName" : [ "Motion Detection", "Region2" ],
                "SmartMotionEnable" : true
            }
            "#
        );

        let event_state_update = Manager::event_state_update_parse(event).unwrap();

        let event_state_update_expected = EventStateUpdate {
            event: Event::VideoMotion,
            active: false,
        };

        assert_eq!(event_state_update, Some(event_state_update_expected));
    }
}
