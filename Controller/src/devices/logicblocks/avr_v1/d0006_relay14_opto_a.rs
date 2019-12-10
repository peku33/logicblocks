use super::super::houseblocks_v1::common::{Address, AddressDeviceType, AddressSerial, Payload};
use super::super::houseblocks_v1::master::Master;
use super::driver::{ApplicationModeDriver, Driver};
use crate::devices::device::{DeviceTrait, RunObjectTrait};
use crate::devices::device_event_stream;
use crate::util::bus2;
use crate::web::router::uri_cursor::{Handler, UriCursor};
use crate::web::{Request, Response};
use failure::{err_msg, format_err, Error};
use futures::future::{BoxFuture, FutureExt, LocalBoxFuture};
use futures::select;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cell::RefCell;
use std::slice;
use std::time::Duration;

pub const RELAYS: usize = 14;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(transparent)]
pub struct RelayStates {
    values: [bool; RELAYS],
}
impl Default for RelayStates {
    fn default() -> Self {
        return Self {
            values: [false; RELAYS],
        };
    }
}

#[derive(Serialize, Clone, Copy, Debug)]
#[serde(tag = "state")]
pub enum State {
    Initializing,
    Running { relay_states: RelayStates },
    Error,
}

pub struct Device<'m> {
    driver: Driver<'m>,

    state: RefCell<State>,

    desired_relay_states: RefCell<RelayStates>,
    desired_relay_states_changed_sender: bus2::Sender<()>,
    desired_relay_states_receiver_factory: bus2::ReceiverFactory<()>,
}
impl<'m> Device<'m> {
    pub fn new(
        master: &'m RefCell<Master>,
        address_serial: AddressSerial,
    ) -> Self {
        let (desired_relay_states_changed_sender, desired_relay_states_receiver_factory) =
            bus2::channel();

        return Self {
            driver: Driver::new(
                master,
                Address::new(
                    AddressDeviceType::new_from_ordinal(6).unwrap(),
                    address_serial,
                ),
            ),
            state: RefCell::new(State::Initializing),

            desired_relay_states: RefCell::new(RelayStates::default()),
            desired_relay_states_changed_sender,
            desired_relay_states_receiver_factory,
        };
    }

    fn relay_states_serialize(relay_states: &RelayStates) -> Payload {
        let mut state_u16: u16 = 0;
        for (i, v) in relay_states.values.iter().enumerate() {
            state_u16 |= if *v { 1 } else { 0 } << i;
        }
        let state_hex = hex::encode_upper(state_u16.to_be_bytes());
        return Payload::new(Box::from(
            [slice::from_ref(&b'H'), state_hex.as_bytes()].concat(),
        ))
        .unwrap();
    }
    async fn relay_states_push<'f>(
        desired_relay_states: &RelayStates,
        application_mode_driver: &'f ApplicationModeDriver<'m, 'f>,
    ) -> Result<(), Error> {
        let response = application_mode_driver
            .transaction_out_in(
                Self::relay_states_serialize(&desired_relay_states),
                Duration::from_millis(250),
            )
            .await?;

        if response.len() != 0 {
            return Err(format_err!(
                "Expected empty response, received: {:?}",
                response
            ));
        }

        return Ok(());
    }

    async fn run_once(
        &self,
        device_event_stream_sender: &device_event_stream::Sender,
    ) -> Result<(), Error> {
        // Set state to initializing
        self.state.replace(State::Initializing);
        device_event_stream_sender.send_empty();

        // Initialize the device
        let application_mode_driver = self.driver.initialize().await?;

        // Device initialization
        let desired_relay_states = *self.desired_relay_states.borrow();
        Self::relay_states_push(&desired_relay_states, &application_mode_driver).await?;
        self.state.replace(State::Running {
            relay_states: desired_relay_states,
        });
        device_event_stream_sender.send_empty();

        // Run main device loop
        let mut desired_relay_states_receiver =
            self.desired_relay_states_receiver_factory.receiver().fuse();

        loop {
            let mut ping_timer = tokio_timer::delay_for(Duration::from_secs(5)).fuse();

            select! {
                _ = ping_timer => {
                    application_mode_driver.healthcheck().await?;
                },
                _ = desired_relay_states_receiver.next() => {
                    let mut state = self.state.borrow_mut();
                    let mut state: &mut State = &mut state;

                    let current_relay_states = match &mut state {
                        State::Running { relay_states } => relay_states,
                        _ => panic!("expected State::Running in main loop"),
                    };

                    let desired_relay_states = *self.desired_relay_states.borrow();

                    if *current_relay_states != desired_relay_states {
                        Self::relay_states_push(&desired_relay_states, &application_mode_driver).await?;
                        *current_relay_states = desired_relay_states;
                        device_event_stream_sender.send_empty();
                    }
                },
            }
        }
    }

    async fn run_loop(
        &self,
        device_event_stream_sender: &device_event_stream::Sender,
    ) -> () {
        loop {
            // Run once
            let error = match self.run_once(device_event_stream_sender).await {
                Ok(_) => panic!("run_once() exited without error"),
                Err(e) => e,
            };
            log::error!("error: {:?}", error);

            // Set error state
            self.state.replace(State::Error);
            device_event_stream_sender.send_empty();

            // Wait and restart
            tokio_timer::delay_for(Duration::from_secs(30)).await;
        }
    }
}
impl<'m> DeviceTrait for Device<'m> {
    fn device_class_get(&self) -> &'static str {
        return "logicblocks/avr_v1/0006_relay14_opto_a";
    }
    fn device_run<'s>(&'s self) -> Box<dyn RunObjectTrait<'s> + 's> {
        let (device_event_stream_sender, device_event_stream_receiver_factory) =
            device_event_stream::channel();

        return Box::new(RunObject {
            run_future: RefCell::new(
                async move {
                    return self.run_loop(&device_event_stream_sender).await;
                }
                .boxed_local(),
            ),
            device_event_stream_receiver_factory,
        });
    }
    fn device_as_routed_handler(&self) -> Option<&dyn Handler> {
        return Some(self);
    }
}
impl<'m> Handler for Device<'m> {
    fn handle(
        &self,
        request: &Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        return match (request.method(), uri_cursor.next_item()) {
            (&http::Method::GET, ("", None)) => {
                let state = *self.state.borrow();
                let desired_relay_states = *self.desired_relay_states.borrow();

                async move {
                    return Response::ok_json(json!({
                        "state": state,
                        "desired_relay_states": desired_relay_states,
                    }));
                }
                .boxed()
            }
            (&http::Method::POST, ("relay_state_transition", None)) => {
                #[derive(Deserialize, Copy, Clone, Debug)]
                pub struct RelayStateTransition {
                    id: usize,
                    state: bool,
                }

                let relay_state_transition = match request.body_parse_json_validate(
                    |relay_state_transition: RelayStateTransition| {
                        if
                        /* relay_state_transition.id < 0 || */
                        relay_state_transition.id >= RELAYS {
                            return Err(err_msg("id out of bounds"));
                        }
                        return Ok(relay_state_transition);
                    },
                ) {
                    Ok(relay_state_transition) => relay_state_transition,
                    Err(error) => {
                        return async move { Response::error_400_from_error(&error) }.boxed()
                    }
                };

                let mut desired_relay_states = self.desired_relay_states.borrow_mut();
                let mut desired_relay_states: &mut RelayStates = &mut desired_relay_states;

                if desired_relay_states.values[relay_state_transition.id]
                    != relay_state_transition.state
                {
                    desired_relay_states.values[relay_state_transition.id] =
                        relay_state_transition.state;
                    self.desired_relay_states_changed_sender.send(());
                }

                async move { Response::ok_empty() }.boxed()
            }
            _ => async move {
                return Response::error_404();
            }
            .boxed(),
        };
    }
}

struct RunObject<'d> {
    run_future: RefCell<LocalBoxFuture<'d, ()>>,
    device_event_stream_receiver_factory: device_event_stream::ReceiverFactory,
}
impl<'d> RunObjectTrait<'d> for RunObject<'d> {
    fn get_run_future(&self) -> &RefCell<LocalBoxFuture<'d, ()>> {
        return &self.run_future;
    }
    fn event_stream_subscribe(&self) -> Option<device_event_stream::Receiver> {
        return Some(self.device_event_stream_receiver_factory.receiver());
    }
}
