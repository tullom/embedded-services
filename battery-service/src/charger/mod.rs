// use defmt::info;
use embassy_executor::Spawner;
use embedded_batteries_async::charger::{self, ErrorKind, MilliVolts};

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};

use crate::BatteryMsgs;

/// Tasks breakdown:
/// Task to recv messages from battery_service (rx_msg_from_service())

pub enum ChargerError {
    Bus,
}

pub struct Charger<SmartCharger: embedded_batteries_async::charger::Charger> {
    device: SmartCharger,
    pub rx: Channel<NoopRawMutex, crate::BatteryMsgs, 1>,

    // Should size of channel be increased as a flurry of messages will need to be sent with broadcasts?
    pub tx: Channel<NoopRawMutex, Result<crate::BatteryMsgs, ChargerError>, 1>,
}

impl<SmartCharger: embedded_batteries_async::charger::Charger> Charger<SmartCharger> {
    pub fn new(smart_charger: SmartCharger) -> Self {
        Charger {
            device: smart_charger,
            rx: Channel::new(),
            tx: Channel::new(),
        }
    }

    async fn rx_msg_from_service(&mut self) {
        let rx_message = self.rx.receive().await;
        // info!("Recv'd charger message!");
        match rx_message {
            BatteryMsgs::Acpi(msg) => todo!(),
            BatteryMsgs::Oem(msg) => match msg {
                crate::OemMessage::ChargeVoltage(voltage) => {
                    let res = self
                        .charge_voltage(voltage)
                        .await
                        // Use voltage returned by fn because the original voltage might not be valid
                        .map(|v| BatteryMsgs::Oem(crate::OemMessage::ChargeVoltage(v)))
                        .map_err(|_| ChargerError::Bus);
                    self.tx.send(res).await;
                }
                _ => todo!(),
            },
        }
    }

    // async fn write_msg(&mut self, msg: crate::BatteryMsgs) {
    //     match msg {
    //         crate::BatteryMsgs::Acpi(acpi_msg) => match acpi_msg {
    //             _ => todo!(),
    //         },
    //         crate::BatteryMsgs::Oem(oem_msg) => match oem_msg {
    //             crate::OemMessage::ChargeVoltage(val) => self.charge_voltage(val).await.unwrap(),
    //             _ => todo!(),
    //         },
    //     }
    // }

    async fn charge_voltage(&mut self, voltage: MilliVolts) -> Result<MilliVolts, ErrorKind> {
        match self.device.charging_voltage(voltage).await {
            Ok(_) => Ok(voltage),
            // TODO: Handle error
            Err(_) => panic!(),
        }
    }
}

#[embassy_executor::task]
async fn wait_for_msg(spawner: Spawner) {}
