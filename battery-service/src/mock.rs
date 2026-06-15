use battery_service_interface::fuel_gauge::{
    DEVICE_CHEMISTRY_ID_SIZE, DEVICE_CHEMISTRY_SIZE, DEVICE_NAME_SIZE, DynamicBatteryMsgs, FuelGauge, FuelGaugeError,
    MANUFACTURER_NAME_SIZE, State, StaticBatteryMsgs,
};
use embassy_time::{Duration, Timer};
use embedded_batteries_async::{
    acpi, charger,
    smart_battery::{self, SmartBattery},
};
use embedded_services::sync::Lockable;
use embedded_services::{error, info, trace};

/// Convenience helper that drives a fuel gauge through its initial bring-up sequence:
/// initialize, then collect static and dynamic data once.
pub async fn init_state_machine<FG>(fuel_gauge: &FG) -> Result<(), <FG::Inner as FuelGauge>::FuelGaugeError>
where
    FG: Lockable,
    FG::Inner: FuelGauge,
{
    let mut fg = fuel_gauge.lock().await;
    fg.initialize()
        .await
        .inspect_err(|_| embedded_services::debug!("Fuel gauge init error"))?;
    fg.update_static_data()
        .await
        .inspect_err(|_| embedded_services::debug!("Fuel gauge static data error"))?;
    fg.update_dynamic_data()
        .await
        .inspect_err(|_| embedded_services::debug!("Fuel gauge dynamic data error"))?;
    Ok(())
}

/// Convenience helper that repeatedly pings a fuel gauge to recover communication,
/// backing off between attempts.
pub async fn recover_state_machine<FG>(fuel_gauge: &FG) -> Result<(), ()>
where
    FG: Lockable,
    FG::Inner: FuelGauge,
{
    let mut retries = 5u32;
    loop {
        let result = fuel_gauge.lock().await.ping().await;
        if result.is_ok() {
            info!("FG recovered!");
            return Ok(());
        }
        retries = retries.saturating_sub(1);
        if retries == 0 {
            error!("Couldn't recover, reinit needed");
            return Err(());
        }
        trace!("Recovery failed, trying again after a backoff period");
        Timer::after(Duration::from_secs(10)).await;
    }
}

/// Helper method to build a zero-padded fixed-size byte array from a byte slice, truncating the
/// slice if it is longer than `N`.
fn padded<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut arr = [0u8; N];
    for (dst, src) in arr.iter_mut().zip(bytes.iter()) {
        *dst = *src;
    }
    arr
}

/// A mock fuel gauge that manages its own state and produces static, arbitrary data.
pub struct MockFuelGauge {
    state: State,
}

impl MockFuelGauge {
    /// Construct a [`MockFuelGauge`] emulating a 3S (three-cell-series) Li-ion
    /// pack at roughly 80% charge under a moderate discharge load, with a
    /// 3000 mAh design capacity (nominal 11.1 V, full charge 12.6 V).
    pub fn new() -> Self {
        Self::with_series_cells(3, padded(b"ODP-3S-3000"))
    }

    /// Construct a [`MockFuelGauge`] emulating a 2S (two-cell-series) Li-ion
    /// pack at roughly 80% charge under a moderate discharge load, with a
    /// 3000 mAh design capacity (nominal 7.4 V, full charge 8.4 V).
    pub fn new_2s() -> Self {
        Self::with_series_cells(2, padded(b"ODP-2S-3000"))
    }

    /// Construct a [`MockFuelGauge`] emulating a 4S (four-cell-series) Li-ion
    /// pack at roughly 80% charge under a moderate discharge load, with a
    /// 3000 mAh design capacity (nominal 14.8 V, full charge 16.8 V).
    pub fn new_4s() -> Self {
        Self::with_series_cells(4, padded(b"ODP-4S-3000"))
    }

    /// Build a [`MockFuelGauge`] preloaded with coherent data for a `cells`-cell
    /// series Li-ion pack reporting `device_name` as its model string.
    ///
    /// Every static and dynamic cache field is populated. Pack voltages scale with
    /// the series cell count (3.7 V nominal, 4.2 V full charge and 3.0 V load
    /// cutoff per cell); the 3000 mAh cell capacity and discharge currents are the
    /// same across topologies, so the reported power quantities scale with the pack
    /// voltage. Capacity and rate are reported in current units (mA/mAh).
    fn with_series_cells(cells: u16, device_name: [u8; 21]) -> Self {
        // Per-cell Li-ion voltages.
        const NOMINAL_MV_PER_CELL: u16 = 3_700;
        const FULL_MV_PER_CELL: u16 = 4_200;
        const CUTOFF_MV_PER_CELL: u16 = 3_000;
        const SOC80_MV_PER_CELL: u16 = 3_950;
        // Pack-independent discharge / threshold currents (mA); power = current * voltage.
        const PEAK_DISCHARGE_MA: u32 = 4_500;
        const SUS_DISCHARGE_MA: u32 = 2_700;
        const INSTANT_THRESHOLD_MA: u32 = 5_400;
        const SUS_THRESHOLD_MA: u32 = 3_150;

        let design_voltage = cells * NOMINAL_MV_PER_CELL;
        let full_charge_voltage = cells * FULL_MV_PER_CELL;
        let terminal_voltage = cells * SOC80_MV_PER_CELL; // ~80% SoC
        let cutoff_voltage = u32::from(cells) * u32::from(CUTOFF_MV_PER_CELL);
        // Power (mW) at the pack's nominal voltage for a given current draw.
        let power_mw = |current_ma: u32| current_ma * u32::from(design_voltage) / 1_000;

        let mut state: State = State::default();
        let s = state.static_cache_mut();
        s.manufacturer_name = padded(b"ODP Batteries");
        s.device_name = device_name;
        s.device_chemistry = padded(b"LION");
        s.design_capacity = smart_battery::CapacityModeValue::MilliAmpUnsigned(3_000); // 3000 mAh design charge
        s.design_voltage = design_voltage;
        s.device_chemistry_id = *b"LI";
        s.serial_num = [0x34, 0x12, 0x00, 0x00]; // serial 0x1234
        // Report capacity/rate in current units (mA/mAh) to match the fields here;
        // alarm and charger-broadcast modes left enabled (the SBS defaults).
        s.battery_mode = smart_battery::BatteryModeFields::new().with_capacity_mode(false);
        s.design_cap_warning = smart_battery::CapacityModeValue::MilliAmpUnsigned(300); // 10% of design (300 mAh)
        s.design_cap_low = smart_battery::CapacityModeValue::MilliAmpUnsigned(150); // 5% of design (150 mAh)
        s.measurement_accuracy = 99_000; // 99.000%, consistent with 1% max error
        s.max_sample_time = 1_000;
        s.min_sample_time = 31;
        s.max_averaging_interval = 4_250;
        s.min_averaging_interval = 25;
        s.cap_granularity_1 = smart_battery::CapacityModeValue::MilliAmpUnsigned(10); // 10 mAh reporting granularity
        s.cap_granularity_2 = smart_battery::CapacityModeValue::MilliAmpUnsigned(10);
        s.power_threshold_support =
            acpi::PowerThresholdSupport::INSTANTANEOUS | acpi::PowerThresholdSupport::SUSTAINABLE;
        s.max_instant_pwr_threshold = power_mw(INSTANT_THRESHOLD_MA);
        s.max_sus_pwr_threshold = power_mw(SUS_THRESHOLD_MA);
        s.bmc_flags = acpi::BmcControlFlags::empty();
        s.bmd_capability =
            acpi::BmdCapabilityFlags::AML_CALIBRATION_SUPPORTED | acpi::BmdCapabilityFlags::CHARGER_DISABLE_SUPPORTED;
        s.bmd_recalibrate_count = 2;
        s.bmd_quick_recalibrate_time = 120;
        s.bmd_slow_recalibrate_time = 600;
        s.manufacture_date = smart_battery::ManufactureDate::new()
            .with_day(16)
            .with_month(10)
            .with_year(45); // 1980 + 45 = 2025
        // Stored as raw u16 (see `StaticBatteryMsgs`); revision 1.1 / version 1.1.
        s.specification_info = smart_battery::SpecificationInfoFields::from(0u16)
            .with_revision(smart_battery::Revision::Version1And1Dot1)
            .with_version(smart_battery::Version::Version1Dot1)
            .into();
        s.remaining_capacity_alarm = smart_battery::CapacityModeValue::MilliAmpUnsigned(300); // 10% of design (300 mAh)
        s.remaining_time_alarm = 10;

        let d = state.dynamic_cache_mut();
        d.max_power = power_mw(PEAK_DISCHARGE_MA);
        d.sus_power = power_mw(SUS_DISCHARGE_MA);
        d.turbo_vload = cutoff_voltage;
        d.turbo_rhf_effective = 150; // pack effective resistance
        d.full_charge_capacity = smart_battery::CapacityModeValue::MilliAmpUnsigned(2_880); // ~96% of design after wear (mAh)
        d.remaining_capacity = smart_battery::CapacityModeValue::MilliAmpUnsigned(2_304); // 80% of full charge (mAh)
        d.relative_soc = 80;
        d.cycle_count = 150;
        d.voltage = terminal_voltage;
        d.max_error = 1;
        // Initialized and discharging; the discharging bit drives the ACPI BST state.
        d.battery_status = smart_battery::BatteryStatusFields::new()
            .with_initialized(true)
            .with_discharging(true)
            .into();
        d.charging_voltage = full_charge_voltage; // desired full-charge voltage
        d.charging_current = 1_500; // desired 0.5C charge current
        d.battery_temp = 3_031; // 30.0 degC
        d.current = -1_500; // discharging
        d.average_current = -1_450;
        d.bmd_status = acpi::BmdStatusFlags::empty();
        d.absolute_soc = 77; // 2_304 / 3_000
        d.at_rate = smart_battery::CapacityModeSignedValue::MilliAmpSigned(-1_500);
        d.at_rate_ok = true;
        d.at_rate_time_to_full = u16::MAX; // over-range: not charging at this rate
        d.at_rate_time_to_empty = 86;
        d.run_time_to_empty = 86;
        d.average_time_to_empty = 88;
        d.average_time_to_full = u16::MAX; // over-range: not charging
        MockFuelGauge { state }
    }

    async fn set_capacity_bit(&mut self, mwh: bool) -> Result<(), MockBatteryError> {
        let battery_mode = self.battery_mode().await?;
        SmartBattery::set_battery_mode(self, battery_mode.with_capacity_mode(mwh)).await?;

        Ok(())
    }
}

impl Default for MockFuelGauge {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MockBatteryError;

impl From<MockBatteryError> for FuelGaugeError {
    fn from(_value: MockBatteryError) -> Self {
        FuelGaugeError::BusError
    }
}

impl FuelGauge for MockFuelGauge {
    type FuelGaugeError = MockBatteryError;
    type StaticData = StaticBatteryMsgs;
    type DynamicData = DynamicBatteryMsgs;

    async fn initialize(&mut self) -> Result<(), Self::FuelGaugeError> {
        // Milliamps
        let mwh = false;
        self.set_capacity_bit(mwh)
            .await
            .inspect_err(|_| error!("FG: failed to initialize"))?;

        info!("FG: initialized");
        self.state_mut().on_initialized();
        Ok(())
    }

    async fn ping(&mut self) -> Result<(), Self::FuelGaugeError> {
        if let Err(e) = self.charging_voltage().await {
            error!("FG: failed to ping");
            Err(e)
        } else {
            info!("FG: ping success");
            self.state_mut().on_recovered();
            Ok(())
        }
    }

    async fn update_dynamic_data(&mut self) -> Result<(), Self::FuelGaugeError> {
        let average_current = self.average_current().await?;
        let battery_status: u16 = self.battery_status().await?.into();
        let battery_temp = self.temperature().await?;
        let charging_current = self.charging_current().await?;
        let charging_voltage = self.charging_voltage().await?;
        let voltage = self.voltage().await?;
        let current = self.current().await?;
        let full_charge_capacity = self.full_charge_capacity().await?;
        let remaining_capacity = self.remaining_capacity().await?;
        let relative_soc = self.relative_state_of_charge().await?;
        let cycle_count = self.cycle_count().await?;
        let max_error = self.max_error().await?;
        let absolute_soc = self.absolute_state_of_charge().await?;
        let at_rate = self.at_rate().await?;
        let at_rate_ok = self.at_rate_ok().await?;
        let at_rate_time_to_full = self.at_rate_time_to_full().await?;
        let at_rate_time_to_empty = self.at_rate_time_to_empty().await?;
        let run_time_to_empty = self.run_time_to_empty().await?;
        let average_time_to_empty = self.average_time_to_empty().await?;
        let average_time_to_full = self.average_time_to_full().await?;

        self.state_mut().on_dynamic_data(|d| {
            d.average_current = average_current;
            d.battery_status = battery_status;
            d.max_power = 100;
            d.battery_temp = battery_temp;
            d.sus_power = 42;
            d.charging_current = charging_current;
            d.charging_voltage = charging_voltage;
            d.voltage = voltage;
            d.current = current;
            d.full_charge_capacity = full_charge_capacity;
            d.remaining_capacity = remaining_capacity;
            d.relative_soc = relative_soc;
            d.cycle_count = cycle_count;
            d.max_error = max_error;
            d.bmd_status = acpi::BmdStatusFlags::default();
            d.turbo_vload = 0;
            d.turbo_rhf_effective = 0;
            d.absolute_soc = absolute_soc;
            d.at_rate = at_rate;
            d.at_rate_ok = at_rate_ok;
            d.at_rate_time_to_full = at_rate_time_to_full;
            d.at_rate_time_to_empty = at_rate_time_to_empty;
            d.run_time_to_empty = run_time_to_empty;
            d.average_time_to_empty = average_time_to_empty;
            d.average_time_to_full = average_time_to_full;
        });
        Ok(())
    }

    async fn update_static_data(&mut self) -> Result<(), Self::FuelGaugeError> {
        let design_capacity = self.design_capacity().await?;
        let design_capacity_value: u32 = match design_capacity {
            smart_battery::CapacityModeValue::CentiWattUnsigned(v) => v.into(),
            smart_battery::CapacityModeValue::MilliAmpUnsigned(v) => v.into(),
        };
        let design_voltage = self.design_voltage().await?;
        let battery_mode = self.battery_mode().await?;
        let measurement_accuracy: u32 = self.max_error().await?.into();
        let manufacture_date = self.manufacture_date().await?;
        let specification_info: u16 = self.specification_info().await?.into();
        let remaining_capacity_alarm = self.remaining_capacity_alarm().await?;
        let remaining_time_alarm = self.remaining_time_alarm().await?;

        let mut manufacturer_name = [0u8; 21];
        self.manufacturer_name(&mut manufacturer_name).await?;
        let mut device_name = [0u8; 21];
        self.device_name(&mut device_name).await?;
        let mut device_chemistry = [0u8; 5];
        self.device_chemistry(&mut device_chemistry).await?;
        let mut device_chemistry_id = [0u8; 2];
        self.device_chemistry(&mut device_chemistry_id).await?;
        let [serial_lsb, serial_msb] = self.serial_number().await?.to_le_bytes();

        self.state_mut().on_static_data(|s| {
            s.manufacturer_name = manufacturer_name;
            s.device_name = device_name;
            s.device_chemistry = device_chemistry;
            s.design_capacity = design_capacity;
            s.design_voltage = design_voltage;
            s.device_chemistry_id = device_chemistry_id;
            s.serial_num = [serial_lsb, serial_msb, 0, 0];
            s.battery_mode = battery_mode;
            s.design_cap_warning =
                smart_battery::CapacityModeValue::MilliAmpUnsigned((design_capacity_value / 4) as u16);
            s.design_cap_low = smart_battery::CapacityModeValue::MilliAmpUnsigned((design_capacity_value / 10) as u16);
            s.measurement_accuracy = measurement_accuracy;
            s.max_sample_time = Default::default();
            s.min_sample_time = Default::default();
            s.max_averaging_interval = Default::default();
            s.min_averaging_interval = Default::default();
            s.cap_granularity_1 = smart_battery::CapacityModeValue::MilliAmpUnsigned(0);
            s.cap_granularity_2 = smart_battery::CapacityModeValue::MilliAmpUnsigned(0);
            s.power_threshold_support = battery_service_interface::PowerThresholdSupport::empty();
            s.max_instant_pwr_threshold = Default::default();
            s.max_sus_pwr_threshold = Default::default();
            s.bmc_flags = battery_service_interface::BmcControlFlags::empty();
            s.bmd_capability = battery_service_interface::BmdCapabilityFlags::empty();
            s.bmd_recalibrate_count = Default::default();
            s.bmd_quick_recalibrate_time = Default::default();
            s.bmd_slow_recalibrate_time = Default::default();
            s.manufacture_date = manufacture_date;
            s.specification_info = specification_info;
            s.remaining_capacity_alarm = remaining_capacity_alarm;
            s.remaining_time_alarm = remaining_time_alarm;
        });
        Ok(())
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

impl smart_battery::Error for MockBatteryError {
    fn kind(&self) -> smart_battery::ErrorKind {
        smart_battery::ErrorKind::Other
    }
}

impl smart_battery::ErrorType for MockFuelGauge {
    type Error = MockBatteryError;
}

// Revisit: Have this generate realistic data dynamically (right now just static arbitrary values)
impl smart_battery::SmartBattery for MockFuelGauge {
    async fn absolute_state_of_charge(&mut self) -> Result<smart_battery::Percent, Self::Error> {
        Ok(self.state.dynamic_cache().absolute_soc)
    }

    async fn at_rate(&mut self) -> Result<smart_battery::CapacityModeSignedValue, Self::Error> {
        Ok(self.state.dynamic_cache().at_rate)
    }

    async fn at_rate_ok(&mut self) -> Result<bool, Self::Error> {
        Ok(self.state.dynamic_cache().at_rate_ok)
    }

    async fn at_rate_time_to_empty(&mut self) -> Result<smart_battery::Minutes, Self::Error> {
        Ok(self.state.dynamic_cache().at_rate_time_to_empty)
    }

    async fn at_rate_time_to_full(&mut self) -> Result<smart_battery::Minutes, Self::Error> {
        Ok(self.state.dynamic_cache().at_rate_time_to_full)
    }

    async fn average_current(&mut self) -> Result<smart_battery::MilliAmpsSigned, Self::Error> {
        Ok(self.state.dynamic_cache().average_current)
    }

    async fn average_time_to_empty(&mut self) -> Result<smart_battery::Minutes, Self::Error> {
        Ok(self.state.dynamic_cache().average_time_to_empty)
    }

    async fn average_time_to_full(&mut self) -> Result<smart_battery::Minutes, Self::Error> {
        Ok(self.state.dynamic_cache().average_time_to_full)
    }

    async fn battery_mode(&mut self) -> Result<smart_battery::BatteryModeFields, Self::Error> {
        Ok(self.state.static_cache().battery_mode)
    }

    async fn battery_status(&mut self) -> Result<smart_battery::BatteryStatusFields, Self::Error> {
        Ok(self.state.dynamic_cache().battery_status.into())
    }

    async fn charging_current(&mut self) -> Result<charger::MilliAmps, Self::Error> {
        Ok(self.state.dynamic_cache().charging_current)
    }

    async fn charging_voltage(&mut self) -> Result<charger::MilliVolts, Self::Error> {
        Ok(self.state.dynamic_cache().charging_voltage)
    }

    async fn current(&mut self) -> Result<smart_battery::MilliAmpsSigned, Self::Error> {
        Ok(self.state.dynamic_cache().current)
    }

    async fn cycle_count(&mut self) -> Result<smart_battery::Cycles, Self::Error> {
        Ok(self.state.dynamic_cache().cycle_count)
    }

    async fn design_capacity(&mut self) -> Result<smart_battery::CapacityModeValue, Self::Error> {
        Ok(self.state.static_cache().design_capacity)
    }

    async fn design_voltage(&mut self) -> Result<charger::MilliVolts, Self::Error> {
        Ok(self.state.static_cache().design_voltage)
    }

    #[allow(clippy::indexing_slicing)]
    async fn device_chemistry(&mut self, chemistry: &mut [u8]) -> Result<(), Self::Error> {
        let bytes = self.state.static_cache().device_chemistry;
        let bytes_to_copy = core::cmp::min(bytes.len(), chemistry.len());
        chemistry[..bytes_to_copy].copy_from_slice(&bytes[..bytes_to_copy]);
        Ok(())
    }

    #[allow(clippy::indexing_slicing)]
    async fn device_name(&mut self, name: &mut [u8]) -> Result<(), Self::Error> {
        let bytes = self.state.static_cache().device_name;
        let bytes_to_copy = core::cmp::min(bytes.len(), name.len());
        name[..bytes_to_copy].copy_from_slice(&bytes[..bytes_to_copy]);
        Ok(())
    }

    async fn full_charge_capacity(&mut self) -> Result<smart_battery::CapacityModeValue, Self::Error> {
        Ok(self.state.dynamic_cache().full_charge_capacity)
    }

    async fn manufacture_date(&mut self) -> Result<smart_battery::ManufactureDate, Self::Error> {
        Ok(self.state.static_cache().manufacture_date)
    }

    #[allow(clippy::indexing_slicing)]
    async fn manufacturer_name(&mut self, name: &mut [u8]) -> Result<(), Self::Error> {
        let bytes = self.state.static_cache().manufacturer_name;
        let bytes_to_copy = core::cmp::min(bytes.len(), name.len());
        name[..bytes_to_copy].copy_from_slice(&bytes[..bytes_to_copy]);
        Ok(())
    }

    async fn max_error(&mut self) -> Result<smart_battery::Percent, Self::Error> {
        Ok(self.state.dynamic_cache().max_error)
    }

    async fn relative_state_of_charge(&mut self) -> Result<smart_battery::Percent, Self::Error> {
        Ok(self.state.dynamic_cache().relative_soc)
    }

    async fn remaining_capacity(&mut self) -> Result<smart_battery::CapacityModeValue, Self::Error> {
        Ok(self.state.dynamic_cache().remaining_capacity)
    }

    async fn remaining_capacity_alarm(&mut self) -> Result<smart_battery::CapacityModeValue, Self::Error> {
        Ok(self.state.static_cache().remaining_capacity_alarm)
    }

    async fn remaining_time_alarm(&mut self) -> Result<smart_battery::Minutes, Self::Error> {
        Ok(self.state.static_cache().remaining_time_alarm)
    }

    async fn run_time_to_empty(&mut self) -> Result<smart_battery::Minutes, Self::Error> {
        Ok(self.state.dynamic_cache().run_time_to_empty)
    }

    async fn serial_number(&mut self) -> Result<u16, Self::Error> {
        let [lsb, msb, _, _] = self.state.static_cache().serial_num;
        Ok(u16::from_le_bytes([lsb, msb]))
    }

    async fn set_at_rate(&mut self, rate: smart_battery::CapacityModeSignedValue) -> Result<(), Self::Error> {
        self.state_mut().dynamic_cache_mut().at_rate = rate;
        Ok(())
    }

    async fn set_battery_mode(&mut self, flags: smart_battery::BatteryModeFields) -> Result<(), Self::Error> {
        self.state_mut().static_cache_mut().battery_mode = flags;
        Ok(())
    }

    async fn set_remaining_capacity_alarm(
        &mut self,
        capacity: smart_battery::CapacityModeValue,
    ) -> Result<(), Self::Error> {
        self.state_mut().static_cache_mut().remaining_capacity_alarm = capacity;
        Ok(())
    }

    async fn set_remaining_time_alarm(&mut self, time: smart_battery::Minutes) -> Result<(), Self::Error> {
        self.state_mut().static_cache_mut().remaining_time_alarm = time;
        Ok(())
    }

    async fn specification_info(&mut self) -> Result<smart_battery::SpecificationInfoFields, Self::Error> {
        Ok(self.state.static_cache().specification_info.into())
    }

    async fn temperature(&mut self) -> Result<smart_battery::DeciKelvin, Self::Error> {
        Ok(self.state.dynamic_cache().battery_temp)
    }

    async fn voltage(&mut self) -> Result<smart_battery::MilliVolts, Self::Error> {
        Ok(self.state.dynamic_cache().voltage)
    }
}
