use core::cell::RefCell;

use embassy_futures::select::select;
use embassy_time::Timer;
use embedded_services::{
    error, info,
    power::policy::charger::{self, ChargeController, ChargerEvent, State},
    trace,
};

pub struct ControllerWrapper<C: ChargeController> {
    charger_state: charger::Device,
    controller: RefCell<C>,
}

const STATE_MACHINE_TIMEOUT_SECS: u64 = 10;

impl<C: ChargeController> ControllerWrapper<C> {
    pub fn new(charger_state: charger::Device, controller: C) -> Self {
        Self {
            charger_state,
            controller: RefCell::new(controller),
        }
    }

    pub async fn get_state(&self) -> State {
        self.charger_state.state().await
    }

    pub async fn set_state(&self, new_state: State) {
        self.charger_state.set_state(new_state).await
    }

    pub async fn process(&self) {
        let mut controller = self.controller.borrow_mut();
        loop {
            let sm_fut = self.run_state_machine(&mut controller);
            let timeout = Timer::after_secs(STATE_MACHINE_TIMEOUT_SECS);

            let res = select(sm_fut, timeout).await;
            match res {
                embassy_futures::select::Either::First(_) => {
                    trace!("Charger state machine heartbeat");
                }
                embassy_futures::select::Either::Second(_) => {
                    error!("Charger state machine timeout!");
                    self.set_state(State::Unpowered).await;
                }
            };
        }
    }

    pub async fn run_state_machine(&self, controller: &mut C) {
        // First loop we want to attempt charger initialization
        loop {
            match self.get_state().await {
                State::Init => {
                    if controller
                        .init_charger()
                        .await
                        .inspect_err(|_| error!("Error initializing charger"))
                        .is_ok()
                    {
                        self.charger_state.send_event(ChargerEvent::Initialized).await
                    }
                }
                State::Idle => { /*  Wait for event */ }
                State::PsuAttached(capability) => {
                    let res = controller.charging_current(capability.current_ma).await;
                    if res.is_ok() {
                        info!("Successfully wrote new charging current to charger: {}mA", res.unwrap());
                    } else {
                        error!("Error writing charging current to charger");
                        self.charger_state.send_event(ChargerEvent::BusError).await
                    }
                }
                State::PsuDetached => {
                    let res = controller.charging_current(0).await;
                    if res.is_err() {
                        error!("Error writing charging current to charger");
                        self.charger_state.send_event(ChargerEvent::BusError).await
                    }
                }
                State::Unpowered => {
                    self.set_state(State::Init).await;
                }
                State::Oem(id) => match id {
                    _ => todo!(),
                },
            }

            // Block until an event occurs
            let event = self.charger_state.wait_event().await;

            match event {
                charger::ChargerEvent::Initialized => self.set_state(State::Idle).await,
                charger::ChargerEvent::PsuAttached(capability) => self.set_state(State::PsuAttached(capability)).await,
                charger::ChargerEvent::PsuDetached => self.set_state(State::PsuDetached).await,
                charger::ChargerEvent::Timeout | charger::ChargerEvent::BusError => {
                    self.set_state(State::Unpowered).await
                }
                charger::ChargerEvent::Oem(state_id) => self.set_state(State::Oem(state_id)).await,
            }
        }
    }
}
