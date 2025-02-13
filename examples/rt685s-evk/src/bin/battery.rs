#![no_std]
#![no_main]

extern crate embedded_services_examples;

use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::once_lock::OnceLock;

use battery_service::Service;

use bq25773::Bq25773;

// embassy_imxrt::bind_interrupts!(struct Irqs {
//     FLEXCOMM2 => embassy_imxrt::i2c::InterruptHandler<embassy_imxrt::peripherals::FLEXCOMM2>;
// });

mod example_battery_service {
    static SERVICE: OnceLock<Service<Bq25773>> = OnceLock::new();

    pub async fn init() {
        let battery_service = SERVICE.get
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_imxrt::init(Default::default());

    info!("Platform initialization complete ...");

    embedded_services::init().await;

    info!("Service initialization complete...");

    // let mut i2c = embassy_imxrt::i2c::master::I2cMaster::new_async(
    //     p.FLEXCOMM2,
    //     p.PIO0_18,
    //     p.PIO0_17,
    //     Irqs,
    //     embassy_imxrt::i2c::master::Speed::Standard,
    //     p.DMA0_CH5,
    // )
    // .unwrap();
    // create an activity service subscriber
    spawner.spawn(activity_example::backlight::task()).unwrap();

    // create an activity service publisher
    spawner.spawn(activity_example::publisher::keyboard_task()).unwrap();

    info!("Subsystem initialization complete...");

    embassy_time::Timer::after_millis(1000).await;
}
