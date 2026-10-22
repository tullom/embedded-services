//! Standard battery example
//!
//! Demonstrates the battery service registration system: the OEM owns the fuel
//! gauge (behind a `Mutex`) and drives it directly through the [`bs::FuelGauge`]
//! trait methods, while the battery service holds the registration and answers
//! ACPI queries by reading the fuel gauge's cached state.
//! The battery task handles power-policy notifications and periodic polling.
//!
//! The example can be run simply by typing `cargo run --bin battery`

use battery_service as bs;
use bs::mock::MockFuelGauge;
use bs::{DefaultPowerPolicyRouting, FuelGauge as _};
use embassy_executor::{Executor, Spawner};
use embassy_sync::{
    channel::{Channel, DynamicReceiver},
    mutex::Mutex,
};
use embassy_time::{Duration, Ticker, Timer};
use embedded_services::GlobalRawMutex;
use static_cell::StaticCell;

/// The fuel gauge, wrapped in a mutex so it can be shared between the OEM driving
/// code and the battery service.
type FuelGauge = Mutex<GlobalRawMutex, MockFuelGauge>;

/// The registration: a single fuel gauge, which becomes battery `0`.
type Reg = bs::ArrayRegistration<'static, FuelGauge, 1, DefaultPowerPolicyRouting>;
type BatteryEvents = bs::EventReceiver<DynamicReceiver<'static, bs::PowerPolicyEventData>>;

#[embassy_executor::task]
async fn embassy_main(spawner: Spawner) {
    embedded_services::debug!("Initializing battery service");
    embedded_services::init().await;

    // The OEM owns the fuel gauge. A shared reference is handed both to the
    // service (via registration) and to the task that drives it.
    static FUEL_GAUGE: StaticCell<FuelGauge> = StaticCell::new();
    let fuel_gauge: &'static FuelGauge = FUEL_GAUGE.init(Mutex::new(MockFuelGauge::new()));

    let battery_service = bs::Service::new(bs::ArrayRegistration {
        fuel_gauges: [fuel_gauge],
        routing: DefaultPowerPolicyRouting,
    });

    // A platform's power-policy notifier sends into this channel.
    static POWER_POLICY_EVENTS: StaticCell<Channel<GlobalRawMutex, bs::PowerPolicyEventData, 4>> = StaticCell::new();
    let power_policy_events = POWER_POLICY_EVENTS.init(Channel::new());
    let events = bs::EventReceiver::new(power_policy_events.dyn_receiver());

    spawner.spawn(run_app(fuel_gauge, battery_service, events).expect("Failed to create run_app task"));
}

#[embassy_executor::task]
async fn run_app(
    fuel_gauge: &'static FuelGauge,
    battery_service: bs::Service<'static, Reg>,
    mut events: BatteryEvents,
) {
    // Initialize the fuel gauge by driving it directly.
    let mut retries = 5;
    while let Err(e) = bs::mock::init_state_machine(fuel_gauge).await {
        retries -= 1;
        if retries <= 0 {
            embedded_services::error!("Failed to initialize Battery: {:?}", e);
            return;
        }
        Timer::after(Duration::from_secs(1)).await;
    }

    let mut failures: u32 = 0;
    let mut count: usize = 1;
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        match events.wait_next_with_poll(ticker.next()).await {
            bs::Event::Poll => {}
            bs::Event::PowerPolicy(event) => {
                if let Err(e) = battery_service.process_power_policy_event(&event).await {
                    embedded_services::error!("Battery power-policy routing error: {:?}", e);
                }
                continue;
            }
        }

        if count.is_multiple_of(const { 60 * 60 * 60 })
            && let Err(e) = fuel_gauge.lock().await.update_static_data().await
        {
            failures += 1;
            embedded_services::error!("Fuel gauge static data error: {:?}", e);
        }
        if let Err(e) = fuel_gauge.lock().await.update_dynamic_data().await {
            failures += 1;
            embedded_services::error!("Fuel gauge dynamic data error: {:?}", e);
        }

        // The battery service answers ACPI queries by reading the fuel gauge's
        // cached state. The caller hands the service exclusive access to the
        // fuel gauge for the duration of the query.
        {
            let mut fg = fuel_gauge.lock().await;
            if battery_service.battery_status(&mut *fg).is_ok() {
                embedded_services::debug!("Queried battery status via the battery service");
            }
        }

        if failures > 10 {
            failures = 0;
            count = 0;
            embedded_services::error!("FG: Too many errors, timing out and starting recovery...");
            if bs::mock::recover_state_machine(fuel_gauge).await.is_err() {
                embedded_services::error!("FG: Fatal error");
                return;
            }
        }

        count = count.wrapping_add(1);
    }
}

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Trace).init();
    embedded_services::info!("battery example started");

    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());
    // Run battery service
    executor.run(|spawner| {
        spawner.spawn(embassy_main(spawner).expect("Failed to create embassy_main task"));
    });
}
