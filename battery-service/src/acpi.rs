#![allow(dead_code)]

use battery_service_interface::BatteryError;
use battery_service_interface::fuel_gauge::{DynamicBatteryData, FuelGauge, StaticBatteryData};
use embedded_batteries_async::acpi::{PowerSourceState, PowerUnit};
use embedded_batteries_async::smart_battery::CapacityModeValue;
use embedded_services::sync::Lockable;
use embedded_services::{info, trace};

use battery_service_interface::{
    BctReturnResult, BixFixedStrings, Bmd, Bpc, Bps, BstReturn, BtmReturnResult, DeviceId, PifFixedStrings, PsrReturn,
    STD_BIX_BATTERY_SIZE, STD_BIX_MODEL_SIZE, STD_BIX_OEM_SIZE, STD_BIX_SERIAL_SIZE, STD_PIF_MODEL_SIZE,
    STD_PIF_OEM_SIZE, STD_PIF_SERIAL_SIZE, StaReturn,
};

use power_policy_interface::capability::PowerCapability;

/// Cached power-supply state used when answering ACPI power-source queries.
///
/// Currently always the default; this is a placeholder for future power policy
/// integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct PsuState {
    pub psu_connected: bool,
    pub power_capability: Option<PowerCapability>,
}

/// Extract the raw numeric value from a [`CapacityModeValue`], discarding the unit
/// tag. The unit (mA/mAh vs centiWatt) is conveyed to ACPI separately via the BIX
/// `power_unit` field, which is derived from the battery's capacity mode.
fn capacity_raw(value: CapacityModeValue) -> u32 {
    match value {
        CapacityModeValue::MilliAmpUnsigned(v) | CapacityModeValue::CentiWattUnsigned(v) => u32::from(v),
    }
}

pub(crate) fn compute_bst<D: DynamicBatteryData>(cache: &D) -> embedded_batteries_async::acpi::BstReturn {
    let cache = cache.standard();
    let charging = if cache.battery_status & (1 << 6) == 0 {
        embedded_batteries_async::acpi::BatteryState::CHARGING
    } else {
        embedded_batteries_async::acpi::BatteryState::DISCHARGING
    };

    // TODO: add critical energy state and charge limiting state
    embedded_batteries_async::acpi::BstReturn {
        battery_state: charging,
        battery_remaining_capacity: capacity_raw(cache.remaining_capacity),
        battery_present_rate: cache.current.unsigned_abs().into(),
        battery_present_voltage: cache.voltage.into(),
    }
}

pub(crate) fn compute_bix<S: StaticBatteryData, D: DynamicBatteryData>(
    static_cache: &S,
    dynamic_cache: &D,
) -> Result<BixFixedStrings, ()> {
    let static_cache = static_cache.standard();
    let dynamic_cache = dynamic_cache.standard();
    let mut bix_return = BixFixedStrings {
        revision: 1,
        power_unit: if static_cache.battery_mode.capacity_mode() {
            PowerUnit::MilliWatts
        } else {
            PowerUnit::MilliAmps
        },
        design_capacity: capacity_raw(static_cache.design_capacity),
        last_full_charge_capacity: capacity_raw(dynamic_cache.full_charge_capacity),
        battery_technology: embedded_batteries_async::acpi::BatteryTechnology::Secondary,
        design_voltage: static_cache.design_voltage.into(),
        design_cap_of_warning: capacity_raw(static_cache.design_cap_warning),
        design_cap_of_low: capacity_raw(static_cache.design_cap_low),
        cycle_count: dynamic_cache.cycle_count.into(),
        measurement_accuracy: u32::from(100 - dynamic_cache.max_error) * 1000u32,
        max_sampling_time: static_cache.max_sample_time,
        min_sampling_time: static_cache.min_sample_time,
        max_averaging_interval: static_cache.max_averaging_interval,
        min_averaging_interval: static_cache.min_averaging_interval,
        battery_capacity_granularity_1: capacity_raw(static_cache.cap_granularity_1),
        battery_capacity_granularity_2: capacity_raw(static_cache.cap_granularity_2),
        model_number: [0u8; STD_BIX_MODEL_SIZE],
        serial_number: [0u8; STD_BIX_SERIAL_SIZE],
        battery_type: [0u8; STD_BIX_BATTERY_SIZE],
        oem_info: [0u8; STD_BIX_OEM_SIZE],
        battery_swapping_capability: embedded_batteries_async::acpi::BatterySwapCapability::NonSwappable,
    };
    let model_number_len = core::cmp::min(STD_BIX_MODEL_SIZE - 1, static_cache.device_name.len() - 1);
    bix_return
        .model_number
        .get_mut(..model_number_len)
        .ok_or(())?
        .copy_from_slice(static_cache.device_name.get(..model_number_len).ok_or(())?);

    let serial_number_len = core::cmp::min(STD_BIX_SERIAL_SIZE - 1, static_cache.serial_num.len() - 1);
    bix_return
        .serial_number
        .get_mut(..serial_number_len)
        .ok_or(())?
        .copy_from_slice(static_cache.serial_num.get(..serial_number_len).ok_or(())?);

    let battery_type_len = core::cmp::min(STD_BIX_BATTERY_SIZE - 1, static_cache.device_chemistry.len() - 1);
    bix_return
        .battery_type
        .get_mut(..battery_type_len)
        .ok_or(())?
        .copy_from_slice(static_cache.device_chemistry.get(..battery_type_len).ok_or(())?);

    let oem_info_len = core::cmp::min(STD_BIX_OEM_SIZE - 1, static_cache.manufacturer_name.len() - 1);
    bix_return
        .oem_info
        .get_mut(..oem_info_len)
        .ok_or(())?
        .copy_from_slice(static_cache.manufacturer_name.get(..oem_info_len).ok_or(())?);

    Ok(bix_return)
}

pub(crate) fn compute_bps<D: DynamicBatteryData>(dynamic_cache: &D) -> embedded_batteries_async::acpi::Bps {
    let dynamic_cache = dynamic_cache.standard();
    // TODO: period values are correct for bq40z50, add to config to support other fuel gauges
    embedded_batteries_async::acpi::Bps {
        revision: 1,
        instantaneous_peak_power_level: dynamic_cache.max_power,
        instantaneous_peak_power_period: 10,
        sustainable_peak_power_level: dynamic_cache.sus_power,
        sustainable_peak_power_period: 10000,
    }
}

pub(crate) fn compute_bpc<S: StaticBatteryData>(static_cache: &S) -> embedded_batteries_async::acpi::Bpc {
    let static_cache = static_cache.standard();
    embedded_batteries_async::acpi::Bpc {
        revision: 1,
        power_threshold_support: static_cache.power_threshold_support,
        max_instantaneous_peak_power_threshold: static_cache.max_instant_pwr_threshold,
        max_sustainable_peak_power_threshold: static_cache.max_sus_pwr_threshold,
    }
}

pub(crate) fn compute_bmd<S: StaticBatteryData, D: DynamicBatteryData>(
    static_cache: &S,
    dynamic_cache: &D,
) -> embedded_batteries_async::acpi::Bmd {
    let static_cache = static_cache.standard();
    let dynamic_cache = dynamic_cache.standard();
    embedded_batteries_async::acpi::Bmd {
        status_flags: dynamic_cache.bmd_status,
        capability_flags: static_cache.bmd_capability,
        recalibrate_count: static_cache.bmd_recalibrate_count,
        quick_recalibrate_time: static_cache.bmd_quick_recalibrate_time,
        slow_recalibrate_time: static_cache.bmd_slow_recalibrate_time,
    }
}

pub(crate) fn compute_bct<D: DynamicBatteryData>(
    payload: &embedded_batteries_async::acpi::Bct,
    _dynamic_cache: &D,
) -> embedded_batteries_async::acpi::BctReturnResult {
    // Just echo back charge level for now
    // TODO: Actually compute time from charge level
    embedded_batteries_async::acpi::BctReturnResult::from(payload.charge_level_percent)
}

pub(crate) fn compute_btm<D: DynamicBatteryData>(
    payload: &embedded_batteries_async::acpi::Btm,
    _dynamic_cache: &D,
) -> embedded_batteries_async::acpi::BtmReturnResult {
    // Just echo back charge level for now
    // TODO: Actually compute time from charge level
    embedded_batteries_async::acpi::BtmReturnResult::from(payload.discharge_rate)
}

pub(crate) fn compute_sta() -> embedded_batteries_async::acpi::StaReturn {
    // TODO: Grab real state values
    embedded_batteries_async::acpi::StaReturn::all()
}

pub(crate) fn compute_psr(psu_state: &PsuState) -> embedded_batteries_async::acpi::PsrReturn {
    // TODO: Refactor to check if battery if force discharged,
    // which should give an offline result even when the PSU is attached.
    embedded_batteries_async::acpi::PsrReturn {
        power_source: if psu_state.psu_connected {
            embedded_batteries_async::acpi::PowerSource::Online
        } else {
            embedded_batteries_async::acpi::PowerSource::Offline
        },
    }
}

pub(crate) fn compute_pif(psu_state: &PsuState) -> PifFixedStrings {
    // TODO: Grab real values from power policy
    let capability = psu_state.power_capability.unwrap_or(PowerCapability {
        voltage_mv: 0,
        current_ma: 0,
    });

    PifFixedStrings {
        power_source_state: PowerSourceState::empty(),
        max_output_power: capability.max_power_mw(),
        max_input_power: capability.max_power_mw(),
        model_number: [0u8; STD_PIF_MODEL_SIZE],
        serial_number: [0u8; STD_PIF_SERIAL_SIZE],
        oem_info: [0u8; STD_PIF_OEM_SIZE],
    }
}

impl<'hw, Reg: crate::registration::Registration<'hw>> crate::Service<'hw, Reg> {
    /// Look up the fuel gauge registered at `device_id`.
    ///
    /// The [`BatteryService`](battery_service_interface::BatteryService) trait impl translates a
    /// [`DeviceId`] into its registered fuel gauge with this helper, locks it, and delegates to the
    /// corresponding reference-based query method below, which holds the actual logic.
    pub(super) fn fuel_gauge(&self, device_id: DeviceId) -> Result<&'hw Reg::FuelGauge, BatteryError> {
        self.registration
            .get_fuel_gauge(device_id)
            .ok_or(BatteryError::UnknownDeviceId)
    }
}

/// Reference-based ACPI query API.
///
/// Unlike the [`BatteryService`](battery_service_interface::BatteryService) trait
/// methods (which identify a battery by [`DeviceId`] and look the fuel gauge up
/// through the service's [`Registration`](crate::registration::Registration)),
/// these methods take an exclusive reference to the fuel gauge directly. The
/// exclusive borrow proves the caller has sole access to the fuel gauge's cached
/// state for the duration of the query, replacing the registration lookup and
/// runtime lock with a compile-time guarantee.
///
/// TODO: Use this over DeviceId based approach?
impl<'hw, Reg: crate::registration::Registration<'hw>> crate::Service<'hw, Reg> {
    /// Queries the estimated time remaining until the battery reaches the specified charge level. Corresponds to ACPI's _BCT method.
    pub fn battery_charge_time(
        &self,
        fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
        bct: embedded_batteries_async::acpi::Bct,
    ) -> Result<BctReturnResult, BatteryError> {
        trace!("Battery service: got BCT command!");
        info!("Recvd BCT charge_level_percent: {}", bct.charge_level_percent);
        Ok(compute_bct(&bct, fuel_gauge.state().dynamic_cache()))
    }

    /// Returns static information about the battery. Corresponds to ACPI's _BIX method.
    pub fn battery_info(
        &self,
        fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<BixFixedStrings, BatteryError> {
        trace!("Battery service: got BIX command!");
        compute_bix(fuel_gauge.state().static_cache(), fuel_gauge.state().dynamic_cache())
            .map_err(|_| BatteryError::UnspecifiedFailure)
    }

    /// Sets the averaging interval of battery capacity measurement in milliseconds. Corresponds to ACPI's _BMA method.
    pub fn set_battery_measurement_averaging_interval(
        &self,
        _fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
        bma: embedded_batteries_async::acpi::Bma,
    ) -> Result<(), BatteryError> {
        trace!("Battery service: got BMA command!");
        info!("Recvd BMA averaging_interval_ms: {}", bma.averaging_interval_ms);
        Ok(())
    }

    /// Battery maintenance control. Corresponds to ACPI's _BMC method.
    pub fn battery_maintenance_control(
        &self,
        _fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
        bmc: embedded_batteries_async::acpi::Bmc,
    ) -> Result<(), BatteryError> {
        trace!("Battery service: got BMC command!");
        info!("Battery service: Bmc {}", bmc.maintenance_control_flags.bits());
        Ok(())
    }

    /// Retrieves battery maintenance data. Corresponds to ACPI's _BMD method.
    pub fn battery_maintenance_data(
        &self,
        fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<Bmd, BatteryError> {
        trace!("Battery service: got BMD command!");
        Ok(compute_bmd(
            fuel_gauge.state().static_cache(),
            fuel_gauge.state().dynamic_cache(),
        ))
    }

    /// Sets the battery measurement sampling time in milliseconds. Corresponds to ACPI's _BMS method.
    pub fn set_battery_measurement_sampling_time(
        &self,
        _fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
        bms: embedded_batteries_async::acpi::Bms,
    ) -> Result<(), BatteryError> {
        trace!("Battery service: got BMS command!");
        info!("Recvd BMS sampling_time: {}", bms.sampling_time_ms);
        Ok(())
    }

    /// Queries the current power characteristics of the battery. Corresponds to ACPI's _BPC method.
    pub fn battery_power_characteristics(
        &self,
        fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<Bpc, BatteryError> {
        trace!("Battery service: got BPC command!");
        Ok(compute_bpc(fuel_gauge.state().static_cache()))
    }

    /// Queries the current state of the battery. Corresponds to ACPI's _BPS method.
    pub fn battery_power_state(
        &self,
        fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<Bps, BatteryError> {
        trace!("Battery service: got BPS command!");
        Ok(compute_bps(fuel_gauge.state().dynamic_cache()))
    }

    /// Sets battery power threshold. Corresponds to ACPI's _BPT method.
    pub fn set_battery_power_threshold(
        &self,
        _fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
        bpt: embedded_batteries_async::acpi::Bpt,
    ) -> Result<(), BatteryError> {
        trace!("Battery service: got BPT command!");
        info!(
            "Battery service: Threshold ID: {:?}, Threshold value: {:?}",
            bpt.threshold_id as u32, bpt.threshold_value
        );
        Ok(())
    }

    /// Queries the battery's current estimated remaining capacity. Corresponds to ACPI's _BST method.
    pub fn battery_status(
        &self,
        fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<BstReturn, BatteryError> {
        trace!("Battery service: got BST command!");
        Ok(compute_bst(fuel_gauge.state().dynamic_cache()))
    }

    /// Queries the estimated time remaining until the battery is fully discharged at the current discharge rate. Corresponds to ACPI's _BTM method.
    pub fn battery_time_to_empty(
        &self,
        fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
        btm: embedded_batteries_async::acpi::Btm,
    ) -> Result<BtmReturnResult, BatteryError> {
        trace!("Battery service: got BTM command!");
        info!("Recvd BTM discharge_rate: {}", btm.discharge_rate);
        Ok(compute_btm(&btm, fuel_gauge.state().dynamic_cache()))
    }

    /// Sets a battery trip point. Corresponds to ACPI's _BTP method.
    pub fn set_battery_trip_point(
        &self,
        _fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
        btp: embedded_batteries_async::acpi::Btp,
    ) -> Result<(), BatteryError> {
        trace!("Battery service: got BTP command!");
        // TODO: Save trip point
        info!("Battery service: New BTP {}", btp.trip_point);
        Ok(())
    }

    /// Queries whether the power supply unit is currently in use (i.e., providing power to the system). Corresponds to ACPI's _PSR method.
    pub fn is_psu_in_use(
        &self,
        _fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<PsrReturn, BatteryError> {
        trace!("Battery service: got PSR command!");
        Ok(compute_psr(&PsuState::default()))
    }

    /// Queries information about the battery's power source. Corresponds to ACPI's _PIF method.
    pub fn power_source_information(
        &self,
        _fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<PifFixedStrings, BatteryError> {
        trace!("Battery service: got PIF command!");
        Ok(compute_pif(&PsuState::default()))
    }

    /// Queries the battery's status. Corresponds to ACPI's _STA method.
    pub fn device_status(
        &self,
        _fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<StaReturn, BatteryError> {
        trace!("Battery service: got STA command!");
        Ok(compute_sta())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use embedded_batteries_async::smart_battery::CapacityModeValue;

    use super::{compute_bix, compute_bpc, compute_bst};
    use battery_service_interface::fuel_gauge::{
        DynamicBatteryData, DynamicBatteryMsgs, StaticBatteryData, StaticBatteryMsgs,
    };

    /// An OEM dynamic data type that embeds the standard messages and extends
    /// them with extra fields.
    struct OemDynamic {
        standard: DynamicBatteryMsgs,
        oem_cell_imbalance_mv: u16,
    }

    impl DynamicBatteryData for OemDynamic {
        fn standard(&self) -> &DynamicBatteryMsgs {
            &self.standard
        }
    }

    /// An OEM static data type that embeds the standard messages and extends them.
    struct OemStatic {
        standard: StaticBatteryMsgs,
        oem_part_number: u32,
    }

    impl StaticBatteryData for OemStatic {
        fn standard(&self) -> &StaticBatteryMsgs {
            &self.standard
        }
    }

    /// The service must compute standard ACPI values from the embedded standard
    /// data of an OEM-extended type, identically to using the standard type directly.
    #[test]
    fn compute_from_extended_type_matches_standard() {
        let standard = DynamicBatteryMsgs {
            remaining_capacity: CapacityModeValue::MilliAmpUnsigned(5000),
            voltage: 12000,
            current: -1500,
            battery_status: 0,
            ..Default::default()
        };
        let oem = OemDynamic {
            standard,
            oem_cell_imbalance_mv: 42,
        };

        let from_standard = compute_bst(&standard);
        let from_oem = compute_bst(&oem);

        assert_eq!(
            from_oem.battery_remaining_capacity,
            from_standard.battery_remaining_capacity
        );
        assert_eq!(from_oem.battery_present_voltage, from_standard.battery_present_voltage);
        assert_eq!(from_oem.battery_present_rate, from_standard.battery_present_rate);
        // The extended field is still readable from the concrete type.
        assert_eq!(oem.oem_cell_imbalance_mv, 42);
    }

    /// `compute_*` functions taking both caches accept OEM-extended types for each.
    #[test]
    fn compute_bix_and_bpc_accept_extended_types() {
        let oem_static = OemStatic {
            standard: StaticBatteryMsgs {
                design_capacity: CapacityModeValue::MilliAmpUnsigned(9000),
                design_voltage: 12000,
                ..Default::default()
            },
            oem_part_number: 0xABCD,
        };
        let oem_dynamic = OemDynamic {
            standard: DynamicBatteryMsgs {
                full_charge_capacity: CapacityModeValue::MilliAmpUnsigned(8500),
                ..Default::default()
            },
            oem_cell_imbalance_mv: 7,
        };

        let bix = compute_bix(&oem_static, &oem_dynamic).expect("bix should compute");
        assert_eq!(bix.design_capacity, 9000);
        assert_eq!(bix.last_full_charge_capacity, 8500);

        let bpc = compute_bpc(&oem_static);
        assert_eq!(
            bpc.max_instantaneous_peak_power_threshold,
            oem_static.standard.max_instant_pwr_threshold
        );
        assert_eq!(oem_static.oem_part_number, 0xABCD);
    }
}
