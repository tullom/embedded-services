use embedded_batteries_async::acpi::PowerUnit;

use crate::device::{DynamicBatteryMsgs, StaticBatteryMsgs};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct Payload<'a> {
    pub version: u8,
    pub instance: u8,
    pub reserved: u8,
    pub command: AcpiCmd,
    pub data: &'a [u8],
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum PayloadError {
    MalformedPayload,
    BufTooSmall,
}

impl<'a> Payload<'a> {
    pub(crate) fn from_raw(raw: &'a [u8], size: usize) -> Result<Self, PayloadError> {
        Ok(Payload {
            version: raw[0],
            instance: raw[1],
            reserved: raw[2],
            command: AcpiCmd::try_from(raw[3])?,
            data: &raw[4..size],
        })
    }

    pub(crate) fn to_raw(&self, buf: &mut [u8]) -> Result<usize, PayloadError> {
        if buf.len() < self.data.len() + 4 {
            return Err(PayloadError::BufTooSmall);
        }

        buf[0] = self.version;
        buf[1] = self.instance;
        buf[2] = self.reserved;
        buf[3] = self.command as u8;
        buf[4..self.data.len() + 4].copy_from_slice(self.data);

        Ok(self.data.len() + 4)
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum AcpiCmd {
    GetBix = 1,
    GetBst = 2,
    GetPsr = 3,
    GetPif = 4,
    GetBps = 5,
    SetBtp = 6,
    SetBpt = 7,
    GetBpc = 8,
    SetBmc = 9,
    GetBmd = 10,
    GetBct = 11,
    GetBtm = 12,
    SetBms = 13,
    SetBma = 14,
    GetSta = 15,
}

impl TryFrom<u8> for AcpiCmd {
    type Error = PayloadError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AcpiCmd::GetBix),
            2 => Ok(AcpiCmd::GetBst),
            3 => Ok(AcpiCmd::GetPsr),
            4 => Ok(AcpiCmd::GetPif),
            5 => Ok(AcpiCmd::GetBps),
            6 => Ok(AcpiCmd::SetBtp),
            7 => Ok(AcpiCmd::SetBpt),
            8 => Ok(AcpiCmd::GetBpc),
            9 => Ok(AcpiCmd::SetBmc),
            10 => Ok(AcpiCmd::GetBmd),
            11 => Ok(AcpiCmd::GetBct),
            12 => Ok(AcpiCmd::GetBtm),
            13 => Ok(AcpiCmd::SetBms),
            14 => Ok(AcpiCmd::SetBma),
            15 => Ok(AcpiCmd::GetSta),
            _ => Err(PayloadError::MalformedPayload),
        }
    }
}

pub(crate) fn compute_bst(cache: &DynamicBatteryMsgs) -> [u8; 16] {
    let mut bst = [0u8; 16];

    let charging = if cache.battery_status & (1 << 6) == 0 {
        embedded_batteries_async::acpi::BatteryState::CHARGING
    } else {
        embedded_batteries_async::acpi::BatteryState::DISCHARGING
    };

    // TODO: add critical energy state and charge limiting state
    let bst_return = embedded_batteries_async::acpi::BstReturn {
        battery_state: charging,
        battery_remaining_capacity: cache.remaining_capacity_mwh,
        battery_present_rate: cache.current_ma.unsigned_abs().into(),
        battery_present_voltage: cache.voltage_mv.into(),
    };

    bst[..4].copy_from_slice(&u32::to_le_bytes(bst_return.battery_state.bits()));
    bst[4..8].copy_from_slice(&u32::to_le_bytes(bst_return.battery_present_rate));
    bst[8..12].copy_from_slice(&u32::to_le_bytes(bst_return.battery_remaining_capacity));
    bst[12..16].copy_from_slice(&u32::to_le_bytes(bst_return.battery_present_voltage));

    bst
}

pub(crate) fn compute_bix(static_cache: &StaticBatteryMsgs, dynamic_cache: &DynamicBatteryMsgs) -> [u8; 16] {
    let mut bst = [0u8; 16];

    let bix_return = embedded_batteries_async::acpi::BixReturn {
        revision: 1,
        power_unit: if static_cache.battery_mode.capacity_mode() {
            PowerUnit::MilliWatts
        } else {
            PowerUnit::MilliAmps
        },
        design_capacity: static_cache.design_capacity_mwh,
        last_full_charge_capacity: dynamic_cache.full_charge_capacity_mwh,
        battery_technology: embedded_batteries_async::acpi::BatteryTechnology::Secondary,
        design_voltage: static_cache.design_voltage_mv.into(),
        design_cap_of_warning: 0, // TODO: read actual value
        design_cap_of_low: 0,     // TODO: read actual value
        cycle_count: dynamic_cache.cycle_count.into(),
        measurement_accuracy: u32::from(100 - dynamic_cache.max_error_pct) * 1000u32,
        max_sampling_time: 0xFFFFFFFF,      // TODO: read this from fg
        min_sampling_time: 0xFFFFFFFF,      // TODO: read this from fg
        max_averaging_interval: 0xFFFFFFFF, // TODO: read this from fg
        min_averaging_interval: 0xFFFFFFFF, // TODO: read this from fg
        battery_capacity_granularity_1: 1,
        battery_capacity_granularity_2: 1,
        model_number: &mut [],
        serial_number: &mut [],
        battery_type: &mut [],
        oem_info: &mut [],
        battery_swapping_capability: embedded_batteries_async::acpi::BatterySwapCapability::NonSwappable,
    };

    // Revision
    // bst[..4].copy_from_slice(&u32::to_le_bytes(bix_return.battery_state.bits()));
    // bst[4..8].copy_from_slice(&u32::to_le_bytes(bix_return.battery_present_rate));
    // bst[8..12].copy_from_slice(&u32::to_le_bytes(bix_return.battery_remaining_capacity));
    // bst[12..16].copy_from_slice(&u32::to_le_bytes(bix_return.battery_present_voltage));

    bst
}
