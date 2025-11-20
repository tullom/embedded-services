use battery_service::task;
use embedded_hal_async::delay::DelayNs;

async fn init_task(mut delay: pico_de_gallo_hal::Delay) {
    embedded_services::init().await;
    loop {
        println!("embedded-services init'd");
        delay.delay_ns(1_000_000_000).await;
    }
}

// async fn thermal_service_task(mut delay: pico_de_gallo_hal::Delay) {
//     thermal_service::init().await;
//     loop {
//         println!("embedded-services init'd");
//         delay.delay_ns(1_000_000_000).await;
//     }
// }

#[tokio::main]
async fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Trace).init();
    println!("hello world");
    let p = pico_de_gallo_hal::Hal::new();

    let fg = bq40z50_rx::Bq40z50R5::new(p.i2c(), p.delay());

    embassy_futures::join::join(
        tokio::spawn(battery_service::task()),
        tokio::spawn(init_task(p.delay())),
    )
    .await;
}
