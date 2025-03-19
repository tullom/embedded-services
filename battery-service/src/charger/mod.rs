use core::cell::RefCell;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embedded_services::error;

use crate::{BatteryMsgTrait, BatteryMsgs};

pub enum ChargerError {
    Bus,
}

pub struct Charger<SmartCharger: embedded_batteries_async::charger::Charger, CustomMsgs: BatteryMsgTrait> {
    device: RefCell<SmartCharger>,
    pub(crate) rx: Channel<NoopRawMutex, crate::BatteryMsgs<CustomMsgs>, 1>,

    // Should size of channel be increased as a flurry of messages will need to be sent with broadcasts?
    pub(crate) tx: Channel<NoopRawMutex, Result<crate::BatteryMsgs<CustomMsgs>, ChargerError>, 1>,
}

impl<SmartCharger: embedded_batteries_async::charger::Charger, CustomMsgs: BatteryMsgTrait>
    Charger<SmartCharger, CustomMsgs>
{
    pub fn new(smart_charger: SmartCharger) -> Self {
        Charger {
            device: RefCell::new(smart_charger),
            rx: Channel::new(),
            tx: Channel::new(),
        }
    }

    pub async fn process_service_message(&self) {
        let rx_message = self.rx.receive().await;
        match rx_message {
            BatteryMsgs::Oem(msg) => match msg {
                crate::OemMessage::ChargeVoltage(voltage) => {
                    let res = self
                        .device
                        .borrow_mut()
                        .charging_voltage(voltage)
                        .await
                        // Use voltage returned by fn because the original voltage might not be valid
                        .map(|v| BatteryMsgs::Oem(crate::OemMessage::ChargeVoltage(v)))
                        .map_err(|_| ChargerError::Bus);
                    self.tx.send(res).await;
                }
                crate::OemMessage::ChargeCurrent(current) => {
                    let res = self
                        .device
                        .borrow_mut()
                        .charging_current(current)
                        .await
                        .map(|c| BatteryMsgs::Oem(crate::OemMessage::ChargeCurrent(c)))
                        .map_err(|_| ChargerError::Bus);
                    self.tx.send(res).await;
                }
            }
            BatteryMsgs::OemGeneric(msg),
            _ => error!("Unexpected message sent to charger"),
        }
    }
}
