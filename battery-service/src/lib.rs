#![no_std]

use battery_service_interface::{
    BatteryError, Bct, BctReturnResult, BixFixedStrings, Bma, Bmc, Bmd, Bms, Bpc, Bps, Bpt, BstReturn, Btm,
    BtmReturnResult, Btp, PifFixedStrings, PsrReturn, StaReturn,
};
use core::marker::PhantomData;
use embedded_services::info;
use embedded_services::sync::Lockable;

mod acpi;
#[cfg(feature = "mock")]
pub mod mock;
pub mod registration;

pub use registration::{ArrayRegistration, Registration};

// Re-export the fuel gauge interface so that OEM drivers and integrators can
// implement and use the battery service without depending on the interface crate directly.
pub use battery_service_interface::fuel_gauge::{
    DynamicBatteryData, DynamicBatteryMsgs, FuelGauge, FuelGaugeError, InternalState, OperationalSubstate,
    PresentSubstate, State, StaticBatteryData, StaticBatteryMsgs,
};
pub use battery_service_interface::{BatteryService, DeviceId};

/// The battery service.
///
/// Owns the [`Registration`] that provides the set of fuel gauges, and answers
/// ACPI battery queries (via the [`BatteryService`] trait) by reading each
/// registered fuel gauge's cached state. The OEM drives each registered fuel
/// gauge directly through the [`FuelGauge`] trait methods.
pub struct Service<'hw, Reg: Registration<'hw>> {
    registration: Reg,
    _phantom: PhantomData<&'hw ()>,
}

impl<'hw, Reg: Registration<'hw>> Service<'hw, Reg> {
    /// Create a new battery service that owns the provided registration.
    pub fn new(registration: Reg) -> Self {
        info!("Starting battery-service");
        Self {
            registration,
            _phantom: PhantomData,
        }
    }

    /// Returns the registered fuel gauges.
    pub fn fuel_gauges(&self) -> &[&'hw Reg::FuelGauge] {
        self.registration.fuel_gauges()
    }

    /// Look up a registered fuel gauge by its device ID.
    pub fn get_fuel_gauge(&self, id: DeviceId) -> Option<&'hw Reg::FuelGauge> {
        self.registration.get_fuel_gauge(id)
    }
}

impl<'hw, Reg: Registration<'hw>> battery_service_interface::BatteryService for Service<'hw, Reg> {
    async fn battery_charge_time(
        &self,
        battery_id: DeviceId,
        charge_level: Bct,
    ) -> Result<BctReturnResult, BatteryError> {
        self.battery_charge_time(&mut *self.fuel_gauge(battery_id)?.lock().await, charge_level)
    }

    async fn battery_info(&self, battery_id: DeviceId) -> Result<BixFixedStrings, BatteryError> {
        self.battery_info(&mut *self.fuel_gauge(battery_id)?.lock().await)
    }

    async fn set_battery_measurement_averaging_interval(
        &self,
        battery_id: DeviceId,
        bma: Bma,
    ) -> Result<(), BatteryError> {
        self.set_battery_measurement_averaging_interval(&mut *self.fuel_gauge(battery_id)?.lock().await, bma)
    }

    async fn battery_maintenance_control(&self, battery_id: DeviceId, bmc: Bmc) -> Result<(), BatteryError> {
        self.battery_maintenance_control(&mut *self.fuel_gauge(battery_id)?.lock().await, bmc)
    }

    async fn battery_maintenance_data(&self, battery_id: DeviceId) -> Result<Bmd, BatteryError> {
        self.battery_maintenance_data(&mut *self.fuel_gauge(battery_id)?.lock().await)
    }

    async fn set_battery_measurement_sampling_time(
        &self,
        battery_id: DeviceId,
        battery_measurement_sampling: Bms,
    ) -> Result<(), BatteryError> {
        self.set_battery_measurement_sampling_time(
            &mut *self.fuel_gauge(battery_id)?.lock().await,
            battery_measurement_sampling,
        )
    }

    async fn battery_power_characteristics(&self, battery_id: DeviceId) -> Result<Bpc, BatteryError> {
        self.battery_power_characteristics(&mut *self.fuel_gauge(battery_id)?.lock().await)
    }

    async fn battery_power_state(&self, battery_id: DeviceId) -> Result<Bps, BatteryError> {
        self.battery_power_state(&mut *self.fuel_gauge(battery_id)?.lock().await)
    }

    async fn set_battery_power_threshold(
        &self,
        battery_id: DeviceId,
        power_threshold: Bpt,
    ) -> Result<(), BatteryError> {
        self.set_battery_power_threshold(&mut *self.fuel_gauge(battery_id)?.lock().await, power_threshold)
    }

    async fn battery_status(&self, battery_id: DeviceId) -> Result<BstReturn, BatteryError> {
        self.battery_status(&mut *self.fuel_gauge(battery_id)?.lock().await)
    }

    async fn battery_time_to_empty(
        &self,
        battery_id: DeviceId,
        battery_discharge_rate: Btm,
    ) -> Result<BtmReturnResult, BatteryError> {
        self.battery_time_to_empty(&mut *self.fuel_gauge(battery_id)?.lock().await, battery_discharge_rate)
    }

    async fn set_battery_trip_point(&self, battery_id: DeviceId, btp: Btp) -> Result<(), BatteryError> {
        self.set_battery_trip_point(&mut *self.fuel_gauge(battery_id)?.lock().await, btp)
    }

    async fn is_psu_in_use(&self, psu_id: DeviceId) -> Result<PsrReturn, BatteryError> {
        self.is_psu_in_use(&mut *self.fuel_gauge(psu_id)?.lock().await)
    }

    async fn power_source_information(&self, power_source_id: DeviceId) -> Result<PifFixedStrings, BatteryError> {
        self.power_source_information(&mut *self.fuel_gauge(power_source_id)?.lock().await)
    }

    async fn device_status(&self, battery_id: DeviceId) -> Result<StaReturn, BatteryError> {
        self.device_status(&mut *self.fuel_gauge(battery_id)?.lock().await)
    }
}
