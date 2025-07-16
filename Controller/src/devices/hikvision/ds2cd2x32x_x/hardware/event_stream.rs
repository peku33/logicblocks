use super::api::Api;
use anyhow::{Context, Error, anyhow, bail};
use atomic_refcell::AtomicRefCell;
use futures::{
    future::FutureExt,
    pin_mut, select,
    stream::{StreamExt, TryStreamExt},
};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use tokio::sync::watch;
use xmltree::Element;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Event {
    CameraFailure,
    VideoLoss,
    TamperingDetection,
    MotionDetection,
    LineDetection,
    FieldDetection,
}

pub type Events = HashSet<Event>;

#[derive(Debug)]
pub struct EventStateUpdate {
    event: Event,
    active: bool,
}

#[derive(Debug)]
pub struct Manager<'a> {
    api: &'a Api,

    events_active: AtomicRefCell<HashMap<Event, usize>>, // Event -> Ticks left

    events_sender: watch::Sender<Events>,
    events_receiver: watch::Receiver<Events>,
}
impl<'a> Manager<'a> {
    const EVENT_STREAM_TIMEOUT: Duration = Duration::from_secs(1);
    const EVENTS_DISABLER_TICK_INTERVAL: Duration = Duration::from_millis(250);
    const EVENTS_DISABLER_TICKS: usize = 5; // 1250ms
    const ERROR_RESTART_DELAY: Duration = Duration::from_secs(1);

    pub fn new(api: &'a Api) -> Self {
        let events_active = HashMap::<Event, usize>::new();
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

    fn event_state_update_parse(element: Element) -> Result<EventStateUpdate, Error> {
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

        let event = match &*event_type {
            "videoloss" => Event::VideoLoss,
            "shelteralarm" => Event::TamperingDetection,
            "VMD" => Event::MotionDetection,
            "linedetection" => Event::LineDetection,
            "fielddetection" => Event::FieldDetection,
            _ => bail!("unknown event type: {}", event_type),
        };
        let active = match &*event_state {
            "inactive" => false,
            "active" => true,
            _ => bail!("unknown event state: {}", event_state),
        };

        Ok(EventStateUpdate { event, active })
    }
    fn event_state_update_handle(
        &self,
        event_state_update: EventStateUpdate,
    ) -> bool {
        let mut events_active = self.events_active.borrow_mut();

        if event_state_update.active {
            events_active
                .insert(event_state_update.event, Self::EVENTS_DISABLER_TICKS)
                .is_none()
        } else {
            events_active.remove(&event_state_update.event).is_some()
        }
    }
    fn events_disabler_handle(&self) -> bool {
        self.events_active
            .borrow_mut()
            .extract_if(|_event, ticks_left| {
                *ticks_left -= 1;
                *ticks_left == 0
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
        let element_stream = self
            .api
            .request_boundary_stream("/ISAPI/Event/notification/alertStream".parse().unwrap())
            .await
            .context("request_boundary_stream")?;

        // TODO: Add timeout
        let element_stream_runner = element_stream
            .try_for_each(async |item| {
                let event_state_update =
                    Self::event_state_update_parse(item).context("event_state_update_parse")?;

                if self.event_state_update_handle(event_state_update) {
                    self.events_propagate();
                }
                Ok(())
            })
            .map(|result| match result.context("element_stream_runner") {
                Ok(()) => anyhow!("data_stream completed"),
                Err(error) => error,
            }).fuse();
        pin_mut!(element_stream_runner);

        let events_disabler_runner = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(Self::EVENTS_DISABLER_TICK_INTERVAL),
        )
        .for_each(async |_time_point| {
            let mut events_changed = false;
            events_changed |= self.events_disabler_handle();
            if events_changed {
                self.events_propagate();
            }
        }).fuse();
        pin_mut!(events_disabler_runner);

        select! {
            element_stream_runner_error = element_stream_runner => bail!(element_stream_runner_error),
            _ = events_disabler_runner => bail!("events_disabler_runner"),
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
