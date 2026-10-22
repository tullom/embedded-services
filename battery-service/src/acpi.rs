#![allow(dead_code)]

use battery_service_interface::BatteryError;
use battery_service_interface::fuel_gauge::{DynamicBatteryData, FuelGauge, StaticBatteryData};
use embedded_batteries_async::acpi::{PowerSource, PowerSourceState, PowerUnit};
use embedded_batteries_async::smart_battery::CapacityModeValue;
use embedded_services::sync::Lockable;
use embedded_services::{error, info, trace};

use battery_service_interface::{
    BctReturnResult, BixFixedStrings, Bmd, Bpc, Bps, BstReturn, BtmReturnResult, DeviceId, PifFixedStrings, PsrReturn,
    STD_BIX_BATTERY_SIZE, STD_BIX_MODEL_SIZE, STD_BIX_OEM_SIZE, STD_BIX_SERIAL_SIZE, STD_PIF_MODEL_SIZE,
    STD_PIF_OEM_SIZE, STD_PIF_SERIAL_SIZE, StaReturn, UNLIMITED_POWER_MW,
};

use power_policy_interface::psu::PsuState;

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
        measurement_accuracy: static_cache.measurement_accuracy,
        max_sampling_time: static_cache.max_sample_time_ms,
        min_sampling_time: static_cache.min_sample_time_ms,
        max_averaging_interval: static_cache.max_averaging_interval_ms,
        min_averaging_interval: static_cache.min_averaging_interval_ms,
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
        instantaneous_peak_power_level: dynamic_cache.max_power_mw,
        instantaneous_peak_power_period: 10,
        sustainable_peak_power_level: dynamic_cache.sus_power_mw,
        sustainable_peak_power_period: 10000,
    }
}

pub(crate) fn compute_bpc<S: StaticBatteryData>(static_cache: &S) -> embedded_batteries_async::acpi::Bpc {
    let static_cache = static_cache.standard();
    embedded_batteries_async::acpi::Bpc {
        revision: 1,
        power_threshold_support: static_cache.power_threshold_support,
        max_instantaneous_peak_power_threshold: static_cache.max_instant_pwr_threshold_mw,
        max_sustainable_peak_power_threshold: static_cache.max_sus_pwr_threshold_mw,
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
        quick_recalibrate_time: static_cache.bmd_quick_recalibrate_time_s,
        slow_recalibrate_time: static_cache.bmd_slow_recalibrate_time_s,
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

pub(crate) fn compute_psr(psu_state: PsuState, unconstrained: bool) -> PsrReturn {
    PsrReturn {
        power_source: if unconstrained || matches!(psu_state, PsuState::ConnectedConsumer(_)) {
            PowerSource::Online
        } else {
            PowerSource::Offline
        },
    }
}

pub(crate) fn compute_pif(psu_state: PsuState, unconstrained: bool) -> PifFixedStrings {
    PifFixedStrings {
        power_source_state: PowerSourceState::empty(),
        // Policy reports the available or negotiated output, which may differ from the adapter's nameplate rating.
        max_output_power: if unconstrained {
            UNLIMITED_POWER_MW
        } else {
            match psu_state {
                PsuState::ConnectedConsumer(capability) => capability.capability.max_power_mw(),
                _ => u32::MAX,
            }
        },
        max_input_power: u32::MAX,
        model_number: [0; STD_PIF_MODEL_SIZE],
        serial_number: [0; STD_PIF_SERIAL_SIZE],
        oem_info: [0; STD_PIF_OEM_SIZE],
    }
}

impl<'hw, Reg: crate::registration::Registration<'hw>> crate::Service<'hw, Reg> {
    /// Look up the fuel gauge registered at `device_id`.
    ///
    /// The [`BatteryService`](battery_service_interface::BatteryService) trait impl translates a
    /// [`DeviceId`] into its registered fuel gauge with this helper, locks it, and delegates to the
    /// corresponding reference-based query method below, which holds the actual logic.
    pub(super) fn fuel_gauge(&self, device_id: DeviceId) -> Result<&'hw Reg::FuelGauge, BatteryError> {
        self.registration.get_fuel_gauge(device_id).ok_or_else(|| {
            error!("Unknown fuel gauge ID {:?}", device_id);
            BatteryError::UnknownDeviceId
        })
    }
}

/// Reference-based ACPI query API.
///
/// Battery queries take an exclusive reference to the fuel gauge. The borrow
/// guarantees at compile time that the caller has sole access to its cached
/// state during the query. No registration lookup or runtime lock is needed.
///
/// The [`BatteryService`](battery_service_interface::BatteryService) trait
/// methods use a [`DeviceId`] to look up the fuel gauge through the service's
/// [`Registration`](crate::registration::Registration).
///
/// Power-source queries use the same reference-based API and read only the
/// external PSU state cached in the fuel gauge.
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
        fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<PsrReturn, BatteryError> {
        trace!("Battery service: got PSR command!");
        let state = fuel_gauge.state();
        Ok(compute_psr(state.psu_state(), state.is_psu_unconstrained()))
    }

    /// Queries information about the battery's power source. Corresponds to ACPI's _PIF method.
    ///
    /// Reports [`UNLIMITED_POWER_MW`] for unconstrained power, otherwise the cached
    /// consumer's negotiated wattage or `u32::MAX` without a consumer.
    /// The negotiated value may be below the rated maximum. Input rating is always unknown.
    /// Notifications do not include adapter identity or redundancy/shared-source
    /// flags, so the strings are empty and those flags are clear.
    pub fn power_source_information(
        &self,
        fuel_gauge: &mut <Reg::FuelGauge as Lockable>::Inner,
    ) -> Result<PifFixedStrings, BatteryError> {
        trace!("Battery service: got PIF command!");
        let state = fuel_gauge.state();
        Ok(compute_pif(state.psu_state(), state.is_psu_unconstrained()))
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
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use core::{
        future::Future,
        pin::pin,
        task::{Context, Waker},
    };

    use embedded_batteries_async::smart_battery::CapacityModeValue;

    use embassy_futures::block_on;
    use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
    use power_policy_interface::{
        capability::{ConsumerFlags, ConsumerPowerCapability, DisconnectFlags, DisconnectReason, PowerCapability},
        service::{UnconstrainedState, event::EventData as PowerPolicyEventData},
    };

    use super::*;
    use crate::{
        ArrayRegistration, BatteryService, DefaultPowerPolicyRouting, PowerPolicyRouting, PowerPolicyUpdate,
        Registration, Service, State, mock::MockFuelGauge,
    };
    use battery_service_interface::fuel_gauge::{
        DynamicBatteryData, DynamicBatteryMsgs, StaticBatteryData, StaticBatteryMsgs,
    };

    struct RouteTo<const N: usize>([DeviceId; N]);

    struct InheritedRouting;

    impl PowerPolicyRouting for InheritedRouting {
        type Event = SourceEvent;
    }

    impl<const N: usize> PowerPolicyRouting for RouteTo<N> {
        type Event = PowerPolicyEventData;

        fn route(&self, event: &Self::Event) -> impl Iterator<Item = (DeviceId, PowerPolicyUpdate)> + Clone {
            DefaultPowerPolicyRouting
                .route(event)
                .flat_map(move |(_, state)| self.0.into_iter().map(move |id| (id, state)))
        }
    }

    struct UpdateRouting<const N: usize>([(DeviceId, PsuState); N]);

    impl<const N: usize> PowerPolicyRouting for UpdateRouting<N> {
        type Event = PowerPolicyEventData;

        fn route(&self, _event: &Self::Event) -> impl Iterator<Item = (DeviceId, PowerPolicyUpdate)> + Clone {
            self.0
                .into_iter()
                .map(|(id, state)| (id, PowerPolicyUpdate::PsuState(state)))
        }
    }

    #[derive(Clone)]
    struct SourceEvent {
        source: DeviceId,
        data: PowerPolicyEventData,
    }

    impl From<SourceEvent> for PowerPolicyEventData {
        fn from(event: SourceEvent) -> Self {
            event.data
        }
    }

    struct SourceRouting;

    impl PowerPolicyRouting for SourceRouting {
        type Event = SourceEvent;

        fn route(&self, event: &Self::Event) -> impl Iterator<Item = (DeviceId, PowerPolicyUpdate)> + Clone {
            DefaultPowerPolicyRouting
                .route(&event.data)
                .filter_map(move |(_, state)| match event.source {
                    DeviceId(0) | DeviceId(1) => Some((event.source, state)),
                    _ => None,
                })
        }
    }

    async fn assert_power_source<'hw, Reg: Registration<'hw>>(
        service: &Service<'hw, Reg>,
        id: DeviceId,
        power_mw: Option<u32>,
    ) {
        let status = BatteryService::is_psu_in_use(service, id).await.unwrap();
        assert!(
            status.power_source
                == if power_mw.is_some() {
                    PowerSource::Online
                } else {
                    PowerSource::Offline
                }
        );
        let information = BatteryService::power_source_information(service, id).await.unwrap();
        assert_eq!(information.max_output_power, power_mw.unwrap_or(u32::MAX));
        assert_eq!(information.max_input_power, u32::MAX);
        assert!(information.power_source_state.is_empty());
        assert_eq!(information.model_number, [0; STD_PIF_MODEL_SIZE]);
        assert_eq!(information.serial_number, [0; STD_PIF_SERIAL_SIZE]);
        assert_eq!(information.oem_info, [0; STD_PIF_OEM_SIZE]);
        let mut gauge = service.get_fuel_gauge(id).unwrap().lock().await;
        assert!(service.is_psu_in_use(&mut *gauge).unwrap().power_source == status.power_source);
        assert!(service.power_source_information(&mut *gauge).unwrap() == information);
    }

    #[test]
    fn shared_references_forward_battery_queries() {
        async fn query(service: impl BatteryService) {
            for (id, voltage) in [(DeviceId(0), 11_100), (DeviceId(1), 7400)] {
                assert_eq!(service.battery_info(id).await.unwrap().design_voltage, voltage);
            }
            assert_eq!(
                service.battery_info(DeviceId(2)).await.err(),
                Some(BatteryError::UnknownDeviceId)
            );
            assert!(service.is_psu_in_use(DeviceId(0)).await.unwrap().power_source == PowerSource::Offline);
            assert!(service.is_psu_in_use(DeviceId(1)).await.unwrap().power_source == PowerSource::Online);
            assert_eq!(
                service
                    .power_source_information(DeviceId(1))
                    .await
                    .unwrap()
                    .max_output_power,
                65_000
            );
        }

        block_on(async {
            let first = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let second = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new_2s());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&first, &second],
                routing: SourceRouting,
            });
            service
                .process_power_policy_event(&SourceEvent {
                    source: DeviceId(1),
                    data: PowerPolicyEventData::ConsumerConnected(
                        PowerCapability {
                            voltage_mv: 20_000,
                            current_ma: 3250,
                        }
                        .into(),
                    ),
                })
                .await
                .unwrap();

            query(&service).await;
            query(&&service).await;
        });
    }

    #[test]
    fn default_routing_updates_only_gauge_zero_for_all_sources() {
        block_on(async {
            let first = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let second = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&first, &second],
                routing: InheritedRouting,
            });

            for source in 0..=u8::MAX {
                let capability = PowerCapability {
                    voltage_mv: 5000,
                    current_ma: 1000 + u16::from(source),
                };
                service
                    .process_power_policy_event(&SourceEvent {
                        source: DeviceId(source),
                        data: PowerPolicyEventData::ConsumerConnected(capability.into()),
                    })
                    .await
                    .unwrap();
                assert_power_source(&service, DeviceId(0), Some(capability.max_power_mw())).await;
                assert_power_source(&service, DeviceId(1), None).await;

                service
                    .process_power_policy_event(&SourceEvent {
                        source: DeviceId(source),
                        data: PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default()),
                    })
                    .await
                    .unwrap();
                assert_power_source(&service, DeviceId(0), None).await;
                assert_power_source(&service, DeviceId(1), None).await;
            }
        });
    }

    #[test]
    fn routed_events_drive_cached_psu_queries() {
        block_on(async {
            let gauge = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let unrelated = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&gauge, &unrelated],
                routing: DefaultPowerPolicyRouting,
            });
            assert!(!gauge.lock().await.state().is_present());
            assert_eq!(gauge.lock().await.state().psu_state(), PsuState::Detached);
            assert_power_source(&service, DeviceId(0), None).await;

            for (capability, expected_mw) in [
                (
                    PowerCapability {
                        voltage_mv: 5000,
                        current_ma: 1500,
                    },
                    7500,
                ),
                (
                    PowerCapability {
                        voltage_mv: 20_000,
                        current_ma: 3250,
                    },
                    65_000,
                ),
                (
                    PowerCapability {
                        voltage_mv: u16::MAX,
                        current_ma: u16::MAX,
                    },
                    4_294_836,
                ),
                (
                    PowerCapability {
                        voltage_mv: 5000,
                        current_ma: 0,
                    },
                    0,
                ),
            ] {
                service
                    .process_power_policy_event(&PowerPolicyEventData::ConsumerConnected(capability.into()))
                    .await
                    .unwrap();
                assert_eq!(
                    gauge.lock().await.state().psu_state(),
                    PsuState::ConnectedConsumer(capability.into())
                );
                assert_power_source(&service, DeviceId(0), Some(expected_mw)).await;
                assert_power_source(&service, DeviceId(1), None).await;
            }

            for reason in [
                None,
                Some(DisconnectReason::Detached),
                Some(DisconnectReason::Switching),
                Some(DisconnectReason::AutoRenegotiation),
                Some(DisconnectReason::ManualRenegotiation),
                Some(DisconnectReason::RoleSwap),
                Some(DisconnectReason::Reset),
            ] {
                service
                    .process_power_policy_event(&PowerPolicyEventData::ConsumerConnected(
                        PowerCapability {
                            voltage_mv: 20_000,
                            current_ma: 3250,
                        }
                        .into(),
                    ))
                    .await
                    .unwrap();
                let event = PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags { reason });
                let expected_state = if reason == Some(DisconnectReason::Detached) {
                    PsuState::Detached
                } else {
                    PsuState::Idle
                };
                for _ in 0..2 {
                    service.process_power_policy_event(&event).await.unwrap();
                    assert_eq!(gauge.lock().await.state().psu_state(), expected_state);
                    assert_power_source(&service, DeviceId(0), None).await;
                    assert_power_source(&service, DeviceId(1), None).await;
                    assert_eq!(unrelated.lock().await.state().psu_state(), PsuState::Detached);
                }
            }
        });
    }

    #[test]
    fn unconstrained_notifications_override_and_restore_cached_input() {
        block_on(async {
            let capability = PowerCapability {
                voltage_mv: 20_000,
                current_ma: 3250,
            };
            for (psu_state, restored_power) in [
                (PsuState::Detached, None),
                (PsuState::Idle, None),
                (PsuState::ConnectedProvider(capability.into()), None),
                (PsuState::ConnectedConsumer(capability.into()), Some(65_000)),
            ] {
                let gauge = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
                let unrelated = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
                gauge.lock().await.state_mut().set_psu_state(psu_state);
                let service = Service::new(ArrayRegistration {
                    fuel_gauges: [&gauge, &unrelated],
                    routing: DefaultPowerPolicyRouting,
                });
                assert_power_source(&service, DeviceId(0), restored_power).await;

                for notification in [
                    UnconstrainedState::new(true, 1),
                    UnconstrainedState::new(true, 2),
                    UnconstrainedState::new(false, 2),
                    UnconstrainedState::new(false, 0),
                    UnconstrainedState::new(true, 0),
                    UnconstrainedState::new(false, 0),
                ] {
                    service
                        .process_power_policy_event(&PowerPolicyEventData::Unconstrained(notification))
                        .await
                        .unwrap();
                    assert_eq!(gauge.lock().await.state().psu_state(), psu_state);
                    assert_eq!(
                        gauge.lock().await.state().is_psu_unconstrained(),
                        notification.unconstrained
                    );
                    assert_power_source(
                        &service,
                        DeviceId(0),
                        if notification.unconstrained {
                            Some(0xffff_fffe)
                        } else {
                            restored_power
                        },
                    )
                    .await;
                    assert_power_source(&service, DeviceId(1), None).await;
                }
            }
        });
    }

    #[test]
    fn consumer_notifications_refresh_unconstrained_power() {
        block_on(async {
            let gauge = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&gauge],
                routing: DefaultPowerPolicyRouting,
            });
            let constrained: ConsumerPowerCapability = PowerCapability {
                voltage_mv: 20_000,
                current_ma: 3250,
            }
            .into();
            let unconstrained = ConsumerPowerCapability {
                flags: ConsumerFlags {
                    unconstrained_power: true,
                    ..Default::default()
                },
                ..constrained
            };
            for (event, expected_power) in [
                (
                    PowerPolicyEventData::Unconstrained(UnconstrainedState::new(true, 1)),
                    Some(0xffff_fffe),
                ),
                (PowerPolicyEventData::ConsumerConnected(constrained), Some(65_000)),
                (
                    PowerPolicyEventData::ConsumerConnected(unconstrained),
                    Some(0xffff_fffe),
                ),
                (
                    PowerPolicyEventData::Unconstrained(UnconstrainedState::new(false, 1)),
                    Some(65_000),
                ),
                (
                    PowerPolicyEventData::ConsumerConnected(unconstrained),
                    Some(0xffff_fffe),
                ),
                (
                    PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default()),
                    None,
                ),
                (
                    PowerPolicyEventData::Unconstrained(UnconstrainedState::new(true, 1)),
                    Some(0xffff_fffe),
                ),
                (
                    PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags {
                        reason: Some(DisconnectReason::Detached),
                    }),
                    None,
                ),
            ] {
                service.process_power_policy_event(&event).await.unwrap();
                assert_power_source(&service, DeviceId(0), expected_power).await;
                assert_eq!(
                    gauge.lock().await.state().is_psu_unconstrained(),
                    expected_power == Some(0xffff_fffe)
                );
            }
        });
    }

    #[test]
    fn attached_psu_without_input_power_reports_offline_and_unknown_output() {
        block_on(async {
            let gauge = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&gauge],
                routing: DefaultPowerPolicyRouting,
            });
            for state in [
                PsuState::Idle,
                PsuState::ConnectedProvider(
                    PowerCapability {
                        voltage_mv: 20_000,
                        current_ma: 3250,
                    }
                    .into(),
                ),
            ] {
                gauge.lock().await.state_mut().set_psu_state(state);
                assert_power_source(&service, DeviceId(0), None).await;
                assert_eq!(gauge.lock().await.state().psu_state(), state);
            }
        });
    }

    #[test]
    fn psu_queries_are_independent_per_fuel_gauge() {
        block_on(async {
            let first = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let second = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new_2s());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&first, &second],
                routing: SourceRouting,
            });

            for (source, capability) in [
                (
                    DeviceId(0),
                    PowerCapability {
                        voltage_mv: 5000,
                        current_ma: 1500,
                    },
                ),
                (
                    DeviceId(1),
                    PowerCapability {
                        voltage_mv: 20_000,
                        current_ma: 3250,
                    },
                ),
            ] {
                service
                    .process_power_policy_event(&SourceEvent {
                        source,
                        data: PowerPolicyEventData::ConsumerConnected(capability.into()),
                    })
                    .await
                    .unwrap();
            }
            assert_power_source(&service, DeviceId(0), Some(7500)).await;
            assert_power_source(&service, DeviceId(1), Some(65_000)).await;

            for unconstrained in [true, false] {
                service
                    .process_power_policy_event(&SourceEvent {
                        source: DeviceId(0),
                        data: PowerPolicyEventData::Unconstrained(UnconstrainedState::new(unconstrained, 2)),
                    })
                    .await
                    .unwrap();
                assert_power_source(
                    &service,
                    DeviceId(0),
                    Some(if unconstrained { 0xffff_fffe } else { 7500 }),
                )
                .await;
                assert_power_source(&service, DeviceId(1), Some(65_000)).await;
            }

            service
                .process_power_policy_event(&SourceEvent {
                    source: DeviceId(0),
                    data: PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default()),
                })
                .await
                .unwrap();
            assert_power_source(&service, DeviceId(0), None).await;
            assert_power_source(&service, DeviceId(1), Some(65_000)).await;
            service
                .process_power_policy_event(&SourceEvent {
                    source: DeviceId(1),
                    data: PowerPolicyEventData::ConsumerConnected(
                        PowerCapability {
                            voltage_mv: 20_000,
                            current_ma: 5000,
                        }
                        .into(),
                    ),
                })
                .await
                .unwrap();
            assert_power_source(&service, DeviceId(0), None).await;
            assert_power_source(&service, DeviceId(1), Some(100_000)).await;

            for (id, voltage) in [(DeviceId(0), 11_100), (DeviceId(1), 7400)] {
                assert_eq!(
                    BatteryService::battery_info(&service, id).await.unwrap().design_voltage,
                    voltage
                );
            }
        });
    }

    #[test]
    fn routing_can_fan_out_a_shared_source() {
        block_on(async {
            let first = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let second = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&first, &second],
                routing: RouteTo([DeviceId(0), DeviceId(1)]),
            });

            service
                .process_power_policy_event(&PowerPolicyEventData::ConsumerConnected(
                    PowerCapability {
                        voltage_mv: 20_000,
                        current_ma: 3250,
                    }
                    .into(),
                ))
                .await
                .unwrap();
            for id in [DeviceId(0), DeviceId(1)] {
                assert_power_source(&service, id, Some(65_000)).await;
            }
            for (unconstrained, expected_power) in [(true, 0xffff_fffe), (false, 65_000)] {
                service
                    .process_power_policy_event(&PowerPolicyEventData::Unconstrained(UnconstrainedState::new(
                        unconstrained,
                        1,
                    )))
                    .await
                    .unwrap();
                for id in [DeviceId(0), DeviceId(1)] {
                    assert_power_source(&service, id, Some(expected_power)).await;
                }
            }
            service
                .process_power_policy_event(&PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default()))
                .await
                .unwrap();
            for id in [DeviceId(0), DeviceId(1)] {
                assert_power_source(&service, id, None).await;
            }
        });
    }

    #[test]
    fn custom_routing_controls_event_filtering_and_each_gauge_state() {
        block_on(async {
            let first = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let second = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let updates = [
                (DeviceId(0), PsuState::Idle),
                (
                    DeviceId(1),
                    PsuState::ConnectedConsumer(
                        PowerCapability {
                            voltage_mv: 20_000,
                            current_ma: 3250,
                        }
                        .into(),
                    ),
                ),
            ];
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&first, &second],
                routing: UpdateRouting(updates),
            });
            for event in [
                PowerPolicyEventData::ConsumerConnected(
                    PowerCapability {
                        voltage_mv: 5000,
                        current_ma: 1500,
                    }
                    .into(),
                ),
                PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags {
                    reason: Some(DisconnectReason::Detached),
                }),
                PowerPolicyEventData::ProviderDisconnected,
                PowerPolicyEventData::Unconstrained(UnconstrainedState::new(true, 2)),
            ] {
                for gauge in [&first, &second] {
                    gauge.lock().await.state_mut().set_psu_state(PsuState::Detached);
                }
                service.process_power_policy_event(&event).await.unwrap();
                for (id, expected) in updates {
                    assert_eq!(
                        service.get_fuel_gauge(id).unwrap().lock().await.state().psu_state(),
                        expected
                    );
                }
                assert_power_source(&service, DeviceId(0), None).await;
                assert_power_source(&service, DeviceId(1), Some(65_000)).await;
            }
        });
    }

    #[test]
    fn unrouted_gauges_remain_offline() {
        block_on(async {
            let first = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let second = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&first, &second],
                routing: SourceRouting,
            });
            assert_power_source(&service, DeviceId(0), None).await;
            assert_power_source(&service, DeviceId(1), None).await;
            service
                .process_power_policy_event(&SourceEvent {
                    source: DeviceId(1),
                    data: PowerPolicyEventData::ConsumerConnected(
                        PowerCapability {
                            voltage_mv: 5000,
                            current_ma: 1500,
                        }
                        .into(),
                    ),
                })
                .await
                .unwrap();
            assert_power_source(&service, DeviceId(0), None).await;
            assert_power_source(&service, DeviceId(1), Some(7500)).await;

            service
                .process_power_policy_event(&SourceEvent {
                    source: DeviceId(2),
                    data: PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default()),
                })
                .await
                .unwrap();
            assert_power_source(&service, DeviceId(0), None).await;
            assert_power_source(&service, DeviceId(1), Some(7500)).await;
        });
    }

    #[test]
    fn psu_queries_reject_unregistered_gauge_ids() {
        block_on(async {
            type Gauge = Mutex<NoopRawMutex, MockFuelGauge>;
            let empty = Service::new(ArrayRegistration::<Gauge, 0, _> {
                fuel_gauges: [],
                routing: RouteTo([]),
            });
            for id in 0..=u8::MAX {
                assert!(matches!(
                    BatteryService::is_psu_in_use(&empty, DeviceId(id)).await,
                    Err(BatteryError::UnknownDeviceId)
                ));
                assert!(matches!(
                    BatteryService::power_source_information(&empty, DeviceId(id)).await,
                    Err(BatteryError::UnknownDeviceId)
                ));
            }
            let first = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let second = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&first, &second],
                routing: RouteTo([]),
            });
            for id in 2..=u8::MAX {
                assert!(matches!(
                    BatteryService::is_psu_in_use(&service, DeviceId(id)).await,
                    Err(BatteryError::UnknownDeviceId)
                ));
                assert!(matches!(
                    BatteryService::power_source_information(&service, DeviceId(id)).await,
                    Err(BatteryError::UnknownDeviceId)
                ));
            }
        });
    }

    #[test]
    fn cancelled_fanout_can_be_retried_without_losing_the_event() {
        block_on(async {
            let first = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let second = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&first, &second],
                routing: RouteTo([DeviceId(0), DeviceId(1)]),
            });
            for (event, expected_power) in [
                (
                    PowerPolicyEventData::ConsumerConnected(
                        PowerCapability {
                            voltage_mv: 20_000,
                            current_ma: 3250,
                        }
                        .into(),
                    ),
                    65_000,
                ),
                (
                    PowerPolicyEventData::Unconstrained(UnconstrainedState::new(true, 1)),
                    0xffff_fffe,
                ),
                (
                    PowerPolicyEventData::Unconstrained(UnconstrainedState::new(false, 0)),
                    65_000,
                ),
            ] {
                let second_guard = second.lock().await;
                {
                    let mut update = pin!(service.process_power_policy_event(&event));
                    assert!(
                        update
                            .as_mut()
                            .poll(&mut Context::from_waker(Waker::noop()))
                            .is_pending()
                    );
                    assert_power_source(&service, DeviceId(0), Some(expected_power)).await;
                }
                drop(second_guard);
                service.process_power_policy_event(&event).await.unwrap();
                for id in [DeviceId(0), DeviceId(1)] {
                    assert_power_source(&service, id, Some(expected_power)).await;
                }
            }
        });
    }

    #[test]
    fn backpressured_notifications_and_cached_queries_do_not_lock_the_psu() {
        use crate::{Event, EventReceiver};
        use core::future::{pending, ready};
        use embassy_sync::channel::Channel;
        use embedded_services::event::{MapSender, NoopSender, Receiver};
        use power_policy_interface::{
            psu::event::NonBlockingSenderNotifier as PsuNotifier,
            service::{event::SenderNotifier, notification::Notifier},
        };
        use power_policy_interface_test_mocks::psu::Mock as MockPsu;

        block_on(async {
            type TestPsu = Mutex<NoopRawMutex, MockPsu<PsuNotifier<NoopSender>>>;
            let psu = TestPsu::new(MockPsu::new("PSU", PsuNotifier(NoopSender)));
            let gauge = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&gauge],
                routing: RouteTo([DeviceId(0)]),
            });
            let events = Channel::<NoopRawMutex, PowerPolicyEventData, 1>::new();
            let mut event_receiver = EventReceiver::new(events.dyn_receiver());
            let mut notifier = SenderNotifier::new(MapSender::new(events.dyn_sender(), PowerPolicyEventData::from));
            let capability = PowerCapability {
                voltage_mv: 20_000,
                current_ma: 3250,
            }
            .into();

            let mut psu_guard = psu.lock().await;
            psu_guard.state.attach().unwrap();
            psu_guard.state.connect_consumer(capability).unwrap();
            notifier.notify_consumer_connected(&psu, capability).await.unwrap();
            psu_guard.state.disconnect(false).unwrap();

            let mut send = pin!(notifier.notify_consumer_disconnected(&psu, DisconnectFlags::default()));
            assert!(send.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
            assert_eq!(event_receiver.wait_next_with_poll(ready(())).await, Event::Poll);
            assert!(send.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
            let connected = event_receiver.wait_next_with_poll(pending()).await;
            assert_eq!(
                connected,
                Event::PowerPolicy(PowerPolicyEventData::ConsumerConnected(capability))
            );
            if let Event::PowerPolicy(event) = connected {
                service.process_power_policy_event(&event).await.unwrap();
            }
            assert_eq!(psu_guard.state.psu_state, PsuState::Idle);
            assert_eq!(
                gauge.lock().await.state().psu_state(),
                PsuState::ConnectedConsumer(capability)
            );
            assert_power_source(&service, DeviceId(0), Some(65_000)).await;
            send.await.unwrap();
            let disconnected = event_receiver.wait_next_with_poll(pending()).await;
            assert_eq!(
                disconnected,
                Event::PowerPolicy(PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default()))
            );
            if let Event::PowerPolicy(event) = disconnected {
                service.process_power_policy_event(&event).await.unwrap();
            }
            assert_eq!(gauge.lock().await.state().psu_state(), PsuState::Idle);
            assert_power_source(&service, DeviceId(0), None).await;
            assert!(psu.try_lock().is_err());
            assert!(event_receiver.try_next().is_none());
        });
    }

    #[test]
    fn invalid_routes_are_rejected_before_any_cache_is_changed() {
        block_on(async {
            let gauge = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            gauge
                .lock()
                .await
                .state_mut()
                .set_psu_state(PsuState::ConnectedConsumer(
                    PowerCapability {
                        voltage_mv: 5000,
                        current_ma: 1500,
                    }
                    .into(),
                ));
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&gauge],
                routing: UpdateRouting([(DeviceId(0), PsuState::Idle), (DeviceId(1), PsuState::Detached)]),
            });
            for event in [
                PowerPolicyEventData::ConsumerConnected(
                    PowerCapability {
                        voltage_mv: 20_000,
                        current_ma: 3250,
                    }
                    .into(),
                ),
                PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default()),
                PowerPolicyEventData::ProviderDisconnected,
            ] {
                assert!(matches!(
                    service.process_power_policy_event(&event).await,
                    Err(BatteryError::UnknownDeviceId)
                ));
                assert_power_source(&service, DeviceId(0), Some(7500)).await;
            }
        });
    }

    #[test]
    fn unconstrained_routes_validate_all_ids_before_updating_caches() {
        block_on(async {
            let gauge = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
            gauge
                .lock()
                .await
                .state_mut()
                .set_psu_state(PsuState::ConnectedConsumer(
                    PowerCapability {
                        voltage_mv: 20_000,
                        current_ma: 3250,
                    }
                    .into(),
                ));
            let service = Service::new(ArrayRegistration {
                fuel_gauges: [&gauge],
                routing: RouteTo([DeviceId(0), DeviceId(1)]),
            });
            for unconstrained in [true, false] {
                gauge.lock().await.state_mut().set_psu_unconstrained(!unconstrained);
                assert!(matches!(
                    service
                        .process_power_policy_event(&PowerPolicyEventData::Unconstrained(UnconstrainedState::new(
                            unconstrained,
                            1,
                        )))
                        .await,
                    Err(BatteryError::UnknownDeviceId)
                ));
                assert_power_source(
                    &service,
                    DeviceId(0),
                    Some(if unconstrained { 65_000 } else { 0xffff_fffe }),
                )
                .await;
            }
        });
    }

    #[test]
    fn provider_events_do_not_change_cached_input() {
        block_on(async {
            let capability = ConsumerPowerCapability {
                capability: PowerCapability {
                    voltage_mv: 5000,
                    current_ma: 1500,
                },
                flags: ConsumerFlags {
                    unconstrained_power: true,
                    psu_type: None,
                },
            };
            for state in [
                PsuState::Detached,
                PsuState::Idle,
                PsuState::ConnectedConsumer(capability),
            ] {
                let gauge = Mutex::<NoopRawMutex, _>::new(MockFuelGauge::new());
                gauge.lock().await.state_mut().set_psu_state(state);
                let service = Service::new(ArrayRegistration {
                    fuel_gauges: [&gauge],
                    routing: RouteTo([DeviceId(0)]),
                });
                for event in [
                    PowerPolicyEventData::ProviderConnected(
                        PowerCapability {
                            voltage_mv: 20_000,
                            current_ma: 5000,
                        }
                        .into(),
                    ),
                    PowerPolicyEventData::ProviderDisconnected,
                ] {
                    service.process_power_policy_event(&event).await.unwrap();
                    assert_eq!(gauge.lock().await.state().psu_state(), state);
                    assert_power_source(
                        &service,
                        DeviceId(0),
                        matches!(state, PsuState::ConnectedConsumer(_)).then_some(0xffff_fffe),
                    )
                    .await;
                }
            }
        });
    }

    #[test]
    fn cached_power_is_independent_of_fuel_gauge_state_transitions() {
        let capability = PowerCapability {
            voltage_mv: 20_000,
            current_ma: 3250,
        };
        for psu_state in [
            PsuState::Detached,
            PsuState::Idle,
            PsuState::ConnectedConsumer(capability.into()),
            PsuState::ConnectedProvider(capability.into()),
        ] {
            let mut state: State = State::default();
            state.set_psu_state(psu_state);
            assert!(!state.is_psu_unconstrained());
            state.set_psu_unconstrained(true);
            assert!(!state.is_present());
            state.on_initialized();
            state.on_static_data(|_| {});
            state.on_dynamic_data(|_| {});
            state.on_timeout();
            state.on_recovered();
            assert_eq!(state.psu_state(), psu_state);
            assert!(state.is_psu_unconstrained());
            state.set_psu_state(PsuState::Detached);
            assert_eq!(state.psu_state(), PsuState::Detached);
            assert!(!state.is_psu_unconstrained());
            assert!(state.is_operational());
        }
    }

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
            oem_static.standard.max_instant_pwr_threshold_mw
        );
        assert_eq!(oem_static.oem_part_number, 0xABCD);
    }
}
