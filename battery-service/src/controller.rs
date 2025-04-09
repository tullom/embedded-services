/// Fuel gauge controller trait that device drivers may use to integrate with internal messaging system
pub trait Controller: embedded_batteries_async::smart_battery::SmartBattery {
    type BusError;

    async fn initialize() -> Result<(), Self::BusError>;
}
