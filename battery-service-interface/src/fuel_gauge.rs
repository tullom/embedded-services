//! Fuel gauge trait, state machine, and cached battery data.
//!
//! Device drivers implement the [`FuelGauge`] trait (which extends
//! [`embedded_batteries_async::smart_battery::SmartBattery`]) and own their own
//! [`State`]. The battery service drives the fuel gauge entirely through direct
//! async calls on the trait. State transitions are driven by
//! the driver (trait implementer) by calling the `on_*` methods on [`State`]
//! (accessed via [`FuelGauge::state_mut`]).

use core::future::Future;

use embedded_batteries_async::{
    acpi::{BmcControlFlags, BmdCapabilityFlags, BmdStatusFlags, PowerThresholdSupport},
    charger::{MilliAmps, MilliVolts},
    smart_battery::{
        BatteryModeFields, CapacityModeSignedValue, CapacityModeValue, Cycles, DeciKelvin, ManufactureDate,
        MilliAmpsSigned, Minutes, Percent,
    },
};

/// Fuel gauge errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum FuelGaugeError {
    /// The fuel gauge hardware timed out responding.
    Timeout,
    /// The underlying bus reported an error.
    BusError,
}

impl From<core::convert::Infallible> for FuelGaugeError {
    fn from(_value: core::convert::Infallible) -> Self {
        Self::BusError
    }
}

/// Size (in bytes) of the cached manufacturer name string in [`StaticBatteryMsgs`], including the null terminator.
pub const MANUFACTURER_NAME_SIZE: usize = 21;
/// Size (in bytes) of the cached device name string in [`StaticBatteryMsgs`], including the null terminator.
pub const DEVICE_NAME_SIZE: usize = 21;
/// Size (in bytes) of the cached device chemistry string in [`StaticBatteryMsgs`], including the null terminator.
pub const DEVICE_CHEMISTRY_SIZE: usize = 5;
/// Size (in bytes) of the cached device chemistry ID in [`StaticBatteryMsgs`].
pub const DEVICE_CHEMISTRY_ID_SIZE: usize = 2;
/// Size (in bytes) of the cached battery serial number in [`StaticBatteryMsgs`].
pub const SERIAL_NUM_SIZE: usize = 4;

/// Standard static battery data cache.
#[derive(Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StaticBatteryMsgs {
    /// Manufacturer Name.
    pub manufacturer_name: [u8; MANUFACTURER_NAME_SIZE],

    /// Device Name.
    pub device_name: [u8; DEVICE_NAME_SIZE],

    /// Device Chemistry.
    pub device_chemistry: [u8; DEVICE_CHEMISTRY_SIZE],

    /// Design Capacity. Units (mA/mAh or centiWatt) are encoded by the [`CapacityModeValue`] variant.
    pub design_capacity: CapacityModeValue,

    /// Design Voltage in mV.
    pub design_voltage: MilliVolts,

    /// Device Chemistry Id.
    pub device_chemistry_id: [u8; DEVICE_CHEMISTRY_ID_SIZE],

    /// Device Serial Number.
    pub serial_num: [u8; SERIAL_NUM_SIZE],

    /// Battery Mode.
    pub battery_mode: BatteryModeFields,

    /// Warning (OEM-designed) capacity threshold.
    ///
    /// Units (mA/mAh or centiWatt) are encoded by the [`CapacityModeValue`] variant.
    pub design_cap_warning: CapacityModeValue,

    /// Low (OEM-designed) capacity threshold.
    ///
    /// Units (mA/mAh or centiWatt) are encoded by the [`CapacityModeValue`] variant.
    pub design_cap_low: CapacityModeValue,

    /// Measurement accuracy in thousandths of a percent (e.g. `80_000` = 80.000%).
    pub measurement_accuracy: u32,

    /// Maximum supported sampling time, in milliseconds.
    pub max_sample_time: u32,

    /// Minimum supported sampling time, in milliseconds.
    pub min_sample_time: u32,

    /// Maximum supported averaging interval, in milliseconds.
    pub max_averaging_interval: u32,

    /// Minimum supported averaging interval, in milliseconds.
    pub min_averaging_interval: u32,

    /// Capacity measurement granularity between the low and warning thresholds.
    ///
    /// Units (mA/mAh or centiWatt) are encoded by the [`CapacityModeValue`] variant.
    pub cap_granularity_1: CapacityModeValue,

    /// Capacity measurement granularity between the warning and full thresholds.
    ///
    /// Units (mA/mAh or centiWatt) are encoded by the [`CapacityModeValue`] variant.
    pub cap_granularity_2: CapacityModeValue,

    /// Which peak-power thresholds the platform supports (ACPI `_BPC`).
    pub power_threshold_support: PowerThresholdSupport,

    /// Maximum supported threshold for instantaneous peak power, in mW.
    pub max_instant_pwr_threshold: u32,

    /// Maximum supported threshold for sustainable peak power, in mW.
    pub max_sus_pwr_threshold: u32,

    /// Battery maintenance control flags configuring calibration and charger behavior (ACPI `_BMC`).
    pub bmc_flags: BmcControlFlags,

    /// Battery maintenance capability flags indicating supported maintenance features (ACPI `_BMD`).
    pub bmd_capability: BmdCapabilityFlags,

    /// Recommended recalibration count.
    ///
    /// `0` means only recalibrate when the recalibration status flag is set;
    /// otherwise recalibrate after this many battery cycles.
    pub bmd_recalibrate_count: u32,

    /// Estimated time, in seconds, to recalibrate if the system enters standby.
    ///
    /// `0` indicates standby is not supported and `0xFFFF_FFFF` indicates the
    /// time is unknown.
    pub bmd_quick_recalibrate_time: u32,

    /// Estimated time, in seconds, to recalibrate without standby.
    ///
    /// `0` indicates calibration may not succeed and `0xFFFF_FFFF` indicates the
    /// time is unknown.
    pub bmd_slow_recalibrate_time: u32,

    /// Manufacture Date. Fixed manufacturing datum, so cached as static data.
    pub manufacture_date: ManufactureDate,

    /// Specification Info, stored as the raw `u16` bitfield representation.
    pub specification_info: u16,

    /// Remaining (low) Capacity Alarm threshold. Units (mA/mAh or centiWatt) are encoded by the [`CapacityModeValue`] variant.
    pub remaining_capacity_alarm: CapacityModeValue,

    /// Remaining Time Alarm threshold in minutes.
    pub remaining_time_alarm: Minutes,
}

impl Default for StaticBatteryMsgs {
    fn default() -> Self {
        Self {
            // Capacity quantities default to the mA variant (matching the SBS
            // default capacity mode); they have no `Default` of their own.
            design_capacity: CapacityModeValue::MilliAmpUnsigned(0),
            remaining_capacity_alarm: CapacityModeValue::MilliAmpUnsigned(0),
            design_cap_warning: CapacityModeValue::MilliAmpUnsigned(0),
            design_cap_low: CapacityModeValue::MilliAmpUnsigned(0),
            cap_granularity_1: CapacityModeValue::MilliAmpUnsigned(0),
            cap_granularity_2: CapacityModeValue::MilliAmpUnsigned(0),
            manufacturer_name: Default::default(),
            device_name: Default::default(),
            device_chemistry: Default::default(),
            design_voltage: Default::default(),
            device_chemistry_id: Default::default(),
            serial_num: Default::default(),
            battery_mode: Default::default(),
            measurement_accuracy: Default::default(),
            max_sample_time: Default::default(),
            min_sample_time: Default::default(),
            max_averaging_interval: Default::default(),
            min_averaging_interval: Default::default(),
            power_threshold_support: Default::default(),
            max_instant_pwr_threshold: Default::default(),
            max_sus_pwr_threshold: Default::default(),
            bmc_flags: Default::default(),
            bmd_capability: Default::default(),
            bmd_recalibrate_count: Default::default(),
            bmd_quick_recalibrate_time: Default::default(),
            bmd_slow_recalibrate_time: Default::default(),
            manufacture_date: Default::default(),
            specification_info: Default::default(),
            remaining_time_alarm: Default::default(),
        }
    }
}

/// Standard dynamic battery data cache.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DynamicBatteryMsgs {
    /// Battery Max Power in mW.
    pub max_power: u32,

    /// Battery Sustained Power in mW.
    pub sus_power: u32,

    /// Turbo Load Voltage in mV.
    pub turbo_vload: u32,

    /// Turbo RHF Effective in mOhm.
    pub turbo_rhf_effective: u32,

    /// Full Charge Capacity. Units (mA/mAh or centiWatt) are encoded by the [`CapacityModeValue`] variant.
    pub full_charge_capacity: CapacityModeValue,

    /// Remaining Capacity. Units (mA/mAh or centiWatt) are encoded by the [`CapacityModeValue`] variant.
    pub remaining_capacity: CapacityModeValue,

    /// Rsoc in %.
    pub relative_soc: Percent,

    /// Charge/Discharge Cycle Count.
    pub cycle_count: Cycles,

    /// Battery Voltage in mV.
    pub voltage: MilliVolts,

    /// Maximum Error in %.
    pub max_error: Percent,

    /// Battery Status (Standard Smart Battery Defined).
    pub battery_status: u16,

    /// Desired Charging Voltage in mV.
    pub charging_voltage: MilliVolts,

    /// Desired Charging Current in mA.
    pub charging_current: MilliAmps,

    /// Battery Temperature in dK.
    pub battery_temp: DeciKelvin,

    /// Battery Current in mA.
    pub current: MilliAmpsSigned,

    /// Battery Avg Current.
    pub average_current: MilliAmpsSigned,

    /// Battery maintenance status flags indicating the current battery maintenance state (ACPI `_BMD`).
    pub bmd_status: BmdStatusFlags,

    /// Absolute State of Charge in % (relative to design capacity).
    pub absolute_soc: Percent,

    /// AtRate value. Units (mA or centiWatt) are encoded by the [`CapacityModeSignedValue`] variant.
    ///
    /// Host-written scratch value that drives the AtRate predictions below, so
    /// cached as dynamic data.
    pub at_rate: CapacityModeSignedValue,

    /// Whether the battery can supply the AtRate value for at least 10 seconds.
    pub at_rate_ok: bool,

    /// Predicted time to fully charge at the AtRate value, in minutes.
    pub at_rate_time_to_full: Minutes,

    /// Predicted time to fully discharge at the AtRate value, in minutes.
    pub at_rate_time_to_empty: Minutes,

    /// Predicted remaining run time to empty at the present discharge rate, in minutes.
    pub run_time_to_empty: Minutes,

    /// Averaged time to empty, in minutes.
    pub average_time_to_empty: Minutes,

    /// Averaged time to full, in minutes.
    pub average_time_to_full: Minutes,
}

impl Default for DynamicBatteryMsgs {
    fn default() -> Self {
        Self {
            // Capacity/rate quantities default to the mA variant (matching the
            // SBS default capacity mode); they have no `Default` of their own.
            full_charge_capacity: CapacityModeValue::MilliAmpUnsigned(0),
            remaining_capacity: CapacityModeValue::MilliAmpUnsigned(0),
            at_rate: CapacityModeSignedValue::MilliAmpSigned(0),
            max_power: Default::default(),
            sus_power: Default::default(),
            turbo_vload: Default::default(),
            turbo_rhf_effective: Default::default(),
            relative_soc: Default::default(),
            cycle_count: Default::default(),
            voltage: Default::default(),
            max_error: Default::default(),
            battery_status: Default::default(),
            charging_voltage: Default::default(),
            charging_current: Default::default(),
            battery_temp: Default::default(),
            current: Default::default(),
            average_current: Default::default(),
            bmd_status: Default::default(),
            absolute_soc: Default::default(),
            at_rate_ok: Default::default(),
            at_rate_time_to_full: Default::default(),
            at_rate_time_to_empty: Default::default(),
            run_time_to_empty: Default::default(),
            average_time_to_empty: Default::default(),
            average_time_to_full: Default::default(),
        }
    }
}

/// Access to the standard [`StaticBatteryMsgs`] within a static battery data type.
///
/// The battery service answers standard ACPI queries from the fields in
/// [`StaticBatteryMsgs`]. A [`FuelGauge`] may cache [`StaticBatteryMsgs`] directly
/// (the default) or a custom OEM type that embeds it and adds extra fields. In the
/// latter case the custom type implements this trait so the service can still read
/// the standard fields, while OEM code reads the extended data from the concrete
/// type via [`State::static_cache`].
pub trait StaticBatteryData {
    /// Returns a reference to the standard static battery data.
    fn standard(&self) -> &StaticBatteryMsgs;
}

impl StaticBatteryData for StaticBatteryMsgs {
    fn standard(&self) -> &StaticBatteryMsgs {
        self
    }
}

/// Access to the standard [`DynamicBatteryMsgs`] within a dynamic battery data type.
///
/// The battery service answers standard ACPI queries from the fields in
/// [`DynamicBatteryMsgs`]. A [`FuelGauge`] may cache [`DynamicBatteryMsgs`] directly
/// (the default) or a custom OEM type that embeds it and adds extra fields. In the
/// latter case the custom type implements this trait so the service can still read
/// the standard fields, while OEM code reads the extended data from the concrete
/// type via [`State::dynamic_cache`].
pub trait DynamicBatteryData {
    /// Returns a reference to the standard dynamic battery data.
    fn standard(&self) -> &DynamicBatteryMsgs;
}

impl DynamicBatteryData for DynamicBatteryMsgs {
    fn standard(&self) -> &DynamicBatteryMsgs {
        self
    }
}

/// Operational state substates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OperationalSubstate {
    /// The fuel gauge is initialized but has not yet collected static data.
    Init,
    /// The fuel gauge is initialized and ready to be polled for dynamic data.
    Polling,
}

/// Present state substates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PresentSubstate {
    /// The fuel gauge is present but communication has been lost and recovery is required.
    NotOperational,
    /// The fuel gauge is present and communicating.
    Operational(OperationalSubstate),
}

/// Current internal state of the fuel gauge.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum InternalState {
    /// No fuel gauge is present or it has not been initialized.
    #[default]
    NotPresent,
    /// A fuel gauge is present.
    Present(PresentSubstate),
}

/// Fuel gauge state, owned by the driver (OEM) and managed via the `on_*` transition methods.
///
/// This holds both the fuel gauge state machine state and the cached static and
/// dynamic battery data. The battery service reads this state (via
/// [`FuelGauge::state`]) when answering ACPI queries.
///
/// The cached data types are generic and default to the standard
/// [`StaticBatteryMsgs`] / [`DynamicBatteryMsgs`]. OEMs may substitute custom
/// types that embed the standard data and implement [`StaticBatteryData`] /
/// [`DynamicBatteryData`] to expose it to the service.
#[derive(Default)]
pub struct State<S: StaticBatteryData = StaticBatteryMsgs, D: DynamicBatteryData = DynamicBatteryMsgs> {
    state: InternalState,
    static_cache: S,
    dynamic_cache: D,
}

impl<S: StaticBatteryData, D: DynamicBatteryData> State<S, D> {
    /// The current internal state.
    pub fn internal_state(&self) -> InternalState {
        self.state
    }

    /// A reference to the cached static battery data.
    pub fn static_cache(&self) -> &S {
        &self.static_cache
    }

    /// A mutable reference to the cached static battery data.
    pub fn static_cache_mut(&mut self) -> &mut S {
        &mut self.static_cache
    }

    /// A reference to the cached dynamic battery data.
    pub fn dynamic_cache(&self) -> &D {
        &self.dynamic_cache
    }

    /// A mutable reference to the cached dynamic battery data.
    pub fn dynamic_cache_mut(&mut self) -> &mut D {
        &mut self.dynamic_cache
    }

    /// Returns `true` if the fuel gauge is present.
    pub fn is_present(&self) -> bool {
        matches!(self.state, InternalState::Present(_))
    }

    /// Returns `true` if the fuel gauge is present and operational.
    pub fn is_operational(&self) -> bool {
        matches!(self.state, InternalState::Present(PresentSubstate::Operational(_)))
    }

    /// Returns `true` if the fuel gauge is present, operational, and polling.
    pub fn is_polling(&self) -> bool {
        matches!(
            self.state,
            InternalState::Present(PresentSubstate::Operational(OperationalSubstate::Polling))
        )
    }

    /// Handle fuel gauge initialization completing.
    ///
    /// Transitions to `Present(Operational(Init))`. Should be called by the
    /// driver after hardware initialization succeeds.
    pub fn on_initialized(&mut self) {
        self.state = InternalState::Present(PresentSubstate::Operational(OperationalSubstate::Init));
    }

    /// Update the cached static battery data in place.
    ///
    /// The `update` closure is given a mutable reference to the cached data and
    /// writes the freshly read values directly into it, so a (potentially large)
    /// `S` is never moved or copied through this call. If the fuel gauge is
    /// operational this also advances the state to `Present(Operational(Polling))`.
    /// Should be called by the driver after a successful static-data read.
    pub fn on_static_data(&mut self, update: impl FnOnce(&mut S)) {
        update(&mut self.static_cache);
        if self.is_operational() {
            self.state = InternalState::Present(PresentSubstate::Operational(OperationalSubstate::Polling));
        }
    }

    /// Update the cached dynamic battery data in place.
    ///
    /// The `update` closure is given a mutable reference to the cached data and
    /// writes the freshly read values directly into it, so a (potentially large)
    /// `D` is never moved or copied through this call. Should be called by the
    /// driver after a successful dynamic-data read while in the
    /// `Present(Operational(Polling))` state.
    pub fn on_dynamic_data(&mut self, update: impl FnOnce(&mut D)) {
        update(&mut self.dynamic_cache);
    }

    /// Handle a communication timeout.
    ///
    /// Transitions a present fuel gauge to `Present(NotOperational)`. Should be
    /// called by the driver when a communication timeout is detected.
    pub fn on_timeout(&mut self) {
        if self.is_present() {
            self.state = InternalState::Present(PresentSubstate::NotOperational);
        }
    }

    /// Handle recovery after re-establishing communication.
    ///
    /// Transitions `Present(NotOperational)` back to `Present(Operational(Init))`.
    /// No-op in any other state. Should be called by the driver after a
    /// successful ping while recovering.
    pub fn on_recovered(&mut self) {
        if matches!(self.state, InternalState::Present(PresentSubstate::NotOperational)) {
            self.state = InternalState::Present(PresentSubstate::Operational(OperationalSubstate::Init));
        }
    }
}

/// Fuel gauge controller trait that device drivers implement to integrate with the battery service.
///
/// The driver is responsible for driving the fuel gauge state machine — it must
/// call the appropriate [`State`] transition methods (via [`Self::state_mut`])
/// based on hardware observations.
pub trait FuelGauge: embedded_batteries_async::smart_battery::SmartBattery {
    /// Type of error returned by the fuel gauge hardware.
    type FuelGaugeError: Into<FuelGaugeError> + embedded_batteries_async::smart_battery::Error;

    /// The cached static battery data type.
    ///
    /// Use [`StaticBatteryMsgs`] (the standard data) directly, or a custom OEM
    /// type that embeds it and implements [`StaticBatteryData`] to add extra fields.
    type StaticData: StaticBatteryData;

    /// The cached dynamic battery data type.
    ///
    /// Use [`DynamicBatteryMsgs`] (the standard data) directly, or a custom OEM
    /// type that embeds it and implements [`DynamicBatteryData`] to add extra fields.
    type DynamicData: DynamicBatteryData;

    /// Initialize the fuel gauge hardware.
    ///
    /// The driver should call [`State::on_initialized`] after a successful
    /// initialization.
    fn initialize(&mut self) -> impl Future<Output = Result<(), Self::FuelGaugeError>>;

    /// Ping the fuel gauge hardware to verify communication.
    ///
    /// When used for recovery, the driver should call [`State::on_recovered`]
    /// after a successful ping.
    fn ping(&mut self) -> impl Future<Output = Result<(), Self::FuelGaugeError>>;

    /// Read static battery data from the hardware.
    ///
    /// The driver should cache the result by calling [`State::on_static_data`].
    fn update_static_data(&mut self) -> impl Future<Output = Result<(), Self::FuelGaugeError>>;

    /// Read dynamic battery data from the hardware.
    ///
    /// The driver should cache the result by calling [`State::on_dynamic_data`].
    fn update_dynamic_data(&mut self) -> impl Future<Output = Result<(), Self::FuelGaugeError>>;

    /// Return an immutable reference to the current fuel gauge state.
    fn state(&self) -> &State<Self::StaticData, Self::DynamicData>;

    /// Return a mutable reference to the current fuel gauge state.
    fn state_mut(&mut self) -> &mut State<Self::StaticData, Self::DynamicData>;
}
