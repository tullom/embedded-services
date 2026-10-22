//! Fuel gauge registration with the battery service.
//!
//! [`Registration`] supplies fuel gauges to the service; [`ArrayRegistration`]
//! stores them in an array. The OEM owns each fuel gauge (typically behind an
//! `embassy_sync` `Mutex`) and drives it directly through the
//! [`FuelGauge`](battery_service_interface::fuel_gauge::FuelGauge) trait methods.
//! The battery service reads each fuel gauge's cached battery and external-power
//! state to answer ACPI queries.

use battery_service_interface::DeviceId;
use embedded_services::sync::Lockable;

use crate::PowerPolicyRouting;

/// Registration trait that abstracts over how fuel gauges are provided to the service.
pub trait Registration<'hw> {
    /// The lockable fuel gauge type. Its inner type implements
    /// [`FuelGauge`](battery_service_interface::fuel_gauge::FuelGauge).
    type FuelGauge: Lockable<Inner: battery_service_interface::fuel_gauge::FuelGauge> + 'hw;

    /// The service's required power-policy router.
    type Routing: PowerPolicyRouting;

    /// The router used for every power-policy notification handled by the service.
    fn routing(&self) -> &Self::Routing;

    /// Returns a slice of the registered fuel gauges.
    ///
    /// A fuel gauge's position in this slice is its [`DeviceId`] for ACPI queries:
    /// the first registered gauge is battery `0`, and so on. Power-source queries
    /// use the same ID to read the gauge's cached external-power state.
    fn fuel_gauges(&self) -> &[&'hw Self::FuelGauge];

    /// Look up a registered fuel gauge by its device ID.
    ///
    /// The device ID is the fuel gauge's position in the registration (the first
    /// registered fuel gauge is battery `0`, and so on).
    fn get_fuel_gauge(&self, id: DeviceId) -> Option<&'hw Self::FuelGauge> {
        self.fuel_gauges().get(usize::from(id.0)).copied()
    }
}

/// An array-backed [`Registration`] implementation.
pub struct ArrayRegistration<
    'hw,
    FG: Lockable<Inner: battery_service_interface::fuel_gauge::FuelGauge> + 'hw,
    const N: usize,
    R: PowerPolicyRouting,
> {
    /// The registered fuel gauges.
    pub fuel_gauges: [&'hw FG; N],
    /// The required router, whose type determines which notifications the service accepts.
    pub routing: R,
}

impl<
    'hw,
    FG: Lockable<Inner: battery_service_interface::fuel_gauge::FuelGauge> + 'hw,
    const N: usize,
    R: PowerPolicyRouting,
> Registration<'hw> for ArrayRegistration<'hw, FG, N, R>
{
    type FuelGauge = FG;
    type Routing = R;

    fn routing(&self) -> &Self::Routing {
        &self.routing
    }

    fn fuel_gauges(&self) -> &[&'hw Self::FuelGauge] {
        &self.fuel_gauges
    }
}
