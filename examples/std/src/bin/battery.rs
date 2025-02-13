use std::convert::Infallible;

use embedded_batteries_async::charger::{MilliAmps, MilliVolts};
use log::*;
use embassy_executor::{Executor, Spawner};
use embassy_sync::once_lock::OnceLock;

use battery_service::Service;
use static_cell::StaticCell;

struct MockCharger {}

impl embedded_batteries_async::charger::ErrorType for MockCharger {
    type Error = Infallible;
}

impl embedded_batteries_async::charger::Charger for MockCharger {
    async fn charging_current(&mut self, current: MilliAmps) -> Result<MilliAmps, Self::Error> {
        Ok(0)
    }

    async fn charging_voltage(&mut self, voltage: MilliVolts) -> Result<MilliVolts, Self::Error> {
        Ok(0)
    }
}

mod example_battery_service {
    use battery_service::Service;
    use embassy_sync::once_lock::OnceLock;
    use embedded_services::comms;

    use crate::MockCharger;

    

}

#[embassy_executor::task]
async fn task(spawner: Spawner) {
    embedded_services::init().await;
    info!("services init'd");
    
    example_battery_service::init().await;
    info!("battery service init'd");
    
}

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Info).init();

    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
        spawner.spawn(task(spawner)).unwrap();
    });
}
