use core::cell::RefCell;

use embassy_futures::select::select;
use embedded_services::{
    error, info,
    power::policy::charger::{
        self, ChargeController, ChargerEvent, ChargerResponse, InternalState, PolicyEvent, State,
    },
    trace, warn,
};

pub struct Wrapper<'a, C: ChargeController> {
    charger_policy_state: &'a charger::Device,
    controller: RefCell<C>,
}

impl<'a, C: ChargeController> Wrapper<'a, C> {
    pub fn new(charger_policy_state: &'a charger::Device, controller: C) -> Self {
        Self {
            charger_policy_state,
            controller: RefCell::new(controller),
        }
    }

    pub async fn get_state(&self) -> charger::InternalState {
        self.charger_policy_state.state().await
    }

    pub async fn set_state(&self, new_state: charger::InternalState) {
        self.charger_policy_state.set_state(new_state).await
    }

    async fn wait_policy_command(&self) -> PolicyEvent {
        self.charger_policy_state.wait_command().await
    }

    #[allow(clippy::single_match)]
    async fn process_controller_event(&self, _controller: &mut C, event: ChargerEvent) {
        let state = self.get_state().await;
        match state.state {
            State::Init => match event {
                ChargerEvent::Initialized => {
                    self.set_state(InternalState {
                        state: State::Idle,
                        capability: state.capability,
                    })
                    .await
                }
                // If we are initializing, we don't care about anything else
                _ => (),
            },
            State::Idle => match event {
                ChargerEvent::PsuAttached => {
                    self.set_state(InternalState {
                        state: State::PsuAttached,
                        capability: state.capability,
                    })
                    .await
                }
                ChargerEvent::PsuDetached => {
                    self.set_state(InternalState {
                        state: State::PsuDetached,
                        capability: state.capability,
                    })
                    .await
                }
                ChargerEvent::Timeout => {
                    self.set_state(InternalState {
                        state: State::Idle,
                        capability: None,
                    })
                    .await
                }
                _ => (),
            },
            State::PsuAttached => match event {
                ChargerEvent::PsuDetached => {
                    self.set_state(InternalState {
                        state: State::PsuDetached,
                        capability: state.capability,
                    })
                    .await
                }
                ChargerEvent::Timeout => {
                    self.set_state(InternalState {
                        state: State::Idle,
                        capability: None,
                    })
                    .await
                }
                _ => (),
            },
            State::PsuDetached => match event {
                ChargerEvent::PsuAttached => {
                    self.set_state(InternalState {
                        state: State::PsuAttached,
                        capability: state.capability,
                    })
                    .await
                }
                ChargerEvent::Timeout => {
                    self.set_state(InternalState {
                        state: State::Idle,
                        capability: None,
                    })
                    .await
                }
                _ => (),
            },
            State::Oem(_id) => todo!(),
        }
    }

    async fn process_policy_command(&self, controller: &mut C, event: PolicyEvent) {
        let state = self.get_state().await;
        let res: ChargerResponse = match state.state {
            State::PsuAttached => match event {
                PolicyEvent::PolicyConfiguration(config) => {
                    info!(
                        "Charger detected new power policy configuration. Writing charge current {}mA.",
                        config.current_ma
                    );
                    if controller
                        .charging_current(config.current_ma)
                        .await
                        .inspect_err(|_| error!("Error writing new power policy to charger!"))
                        .is_err()
                    {
                        Err(charger::ChargerError::BusError)
                    } else {
                        Ok(charger::ChargerResponseData::Ack)
                    }
                }
                PolicyEvent::Oem(_oem_state_id) => todo!(),
                PolicyEvent::InitRequest => {
                    error!("Charger received request to initialize but it's already initialized!");
                    Err(charger::ChargerError::InvalidState(state.state))
                }
            },
            State::PsuDetached => match event {
                PolicyEvent::PolicyConfiguration(config) => {
                    if config.current_ma != 0 {
                        warn!("Charger detected new non-zero power policy configuration but charger is in a PSU detached state.");
                        Err(charger::ChargerError::InvalidState(state.state))
                    } else {
                        info!(
                            "Charger detected new power policy configuration. Writing charge current {}mA.",
                            config.current_ma
                        );
                        if let Err(_err) = controller
                            .charging_current(config.current_ma)
                            .await
                            .inspect_err(|_| error!("Error writing new power policy to charger!"))
                        {
                            Err(charger::ChargerError::BusError)
                        } else {
                            Ok(charger::ChargerResponseData::Ack)
                        }
                    }
                }
                PolicyEvent::Oem(_oem_state_id) => todo!(),
                PolicyEvent::InitRequest => {
                    error!("Charger received request to initialize but it's already initialized!");
                    Err(charger::ChargerError::InvalidState(state.state))
                }
            },
            State::Idle => match event {
                PolicyEvent::PolicyConfiguration(_) => {
                    warn!("Charger detected new power policy configuration but charger is still initializing.");
                    Err(charger::ChargerError::InvalidState(state.state))
                }
                PolicyEvent::Oem(_oem_state_id) => todo!(),
                PolicyEvent::InitRequest => {
                    error!("Charger received request to initialize but it's already initialized!");
                    Err(charger::ChargerError::InvalidState(state.state))
                }
            },
            State::Init => match event {
                PolicyEvent::PolicyConfiguration(_) => {
                    warn!("Charger detected new power policy configuration but charger is still initializing.");
                    Err(charger::ChargerError::InvalidState(state.state))
                }
                PolicyEvent::Oem(_oem_state_id) => todo!(),
                PolicyEvent::InitRequest => {
                    info!("Charger received request to initialize.");
                    if let Err(_err) = controller.init_charger().await {
                        Err(charger::ChargerError::BusError)
                    } else {
                        Ok(charger::ChargerResponseData::Ack)
                    }
                }
            },
            State::Oem(_oem_state_id) => todo!(),
        };

        // Send response
        self.charger_policy_state.send_response(res).await;
    }

    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn process(&self) {
        let mut controller = self.controller.borrow_mut();
        loop {
            let res = select(controller.wait_event(), self.wait_policy_command()).await;
            match res {
                embassy_futures::select::Either::First(event) => {
                    trace!("New charger device event.");
                    self.process_controller_event(&mut controller, event).await;
                }
                embassy_futures::select::Either::Second(event) => {
                    trace!("New charger policy command.");
                    self.process_policy_command(&mut controller, event).await;
                }
            };
        }
    }
}
