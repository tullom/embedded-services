#![no_std]

use core::any::TypeId;

use embassy_sync::once_lock::OnceLock;
use embedded_batteries_async::charger::{MilliAmps, MilliVolts};
use embedded_services::comms;

mod charger;
mod fuel_gauge;

/// Tasks breakdown:
/// Task to recv messages from other services (comms::MailboxDelegate::receive)
/// Task to send messages to other services (handle_charger_fuel_gauge_msg())

// TEMPORARILY COPY PASTED
// TODO: get from espi service
#[derive(Copy, Clone, Debug)]
enum ESpiMessage {
    // CAPS fields
    CapsFwVersion(u16),  // CAPS_FW_VERSION
    CapsSecureState(u8), // CAPS_SECURE_STATE
    CapsBootStatus(u8),  // CAPS_BOOT_STATUS
    CapsDebugMask(u16),  // CAPS_DEBUG_MASK
    CapsBatteryMask(u8), // CAPS_BATTERY_MASK
    CapsFanMask(u8),     // CAPS_FAN_MASK
    CapsTempMask(u8),    // CAPS_TEMP_MASK
    CapsHidMask(u8),     // CAPS_HID_MASK
    CapsKeyMask(u8),     // CAPS_KEY_MASK

    // BAT fields
    BatLastFullCharge(u32), // BAT_LAST_FULL_CHARGE (BIX)
    BatCycleCount(u32),     // BAT_CYCLE_COUNT (BIX)
    BatState(u32),          // BAT_STATE (BST)
    BatPresentRate(u32),    // BAT_PRESENT_RATE (BST)
    BatRemainCap(u32),      // BAT_REMAIN_CAP (BST)
    BatPresentVolt(u32),    // BAT_PRESENT_VOLT (BST)
    BatPsrState(u32),       // BAT_PSR_STATE (PSR/PIF)
    BatPsrMaxOut(u32),      // BAT_PSR_MAX_OUT (PIF)
    BatPsrMaxIn(u32),       // BAT_PSR_MAX_IN (PIF)
    BatPeakLevel(u32),      // BAT_PEAK_LEVEL (BPS)
    BatPeakPower(u32),      // BAT_PEAK_POWER (BPS/BPC)
    BatSusLevel(u32),       // BAT_SUS_LEVEL (BPS)
    BatSusPower(u32),       // BAT_SUS_POWER (BPS/PBC)
    BatPeakThres(u32),      // BAT_PEAK_THRES (BPT)
    BatSusThres(u32),       // BAT_SUS_THRES (BPT)
    BatTripThres(u32),      // BAT_TRIP_THRES (BTP)
    BatBmcData(u32),        // BAT_BMC_DATA (BMC)
    BatBmdStatus(u32),      // BAT_BMD_STATUS (BMD)
    BatBmdFlags(u32),       // BAT_BMD_FLAGS (BMD)
    BatBmdCount(u32),       // BAT_BMD_COUNT (BMD)
    BatChargeTime(u32),     // BAT_CHARGE_TIME (BCT)
    BatRunTime(u32),        // BAT_RUN_TIME (BTM)
    BatSampleTime(u32),     // BAT_SAMPLE_TIME (BMS/BMA)

    // MPTF fields
    MptfTmp1Val(u32),     // THM_TMP1_VAL (TMP)
    MptfTmp1Timeout(u32), // THM_TMP1_TIMEOUT (EC_THM_SET/GET_THRS)
    MptfTmp1Low(u32),     // THM_TMP1_LOW (EC_THM_SET/GET_THRS)
    MptfTmp1High(u32),    // THM_TMP1_HIGH (EC_THM_SET/GET_THRS)
    MptfCoolMode(u32),    // THM_COOL_MODE (EC_THM_SET_SCP)
    MptfFanOnTemp(u32),   // THM_FAN_ON_TEMP (GET/SET VAR)
    MptfFanRampTemp(u32), // THM_FAN_RAMP_TEMP (GET/SET VAR)
    MptfFanMaxTemp(u32),  // THM_FAN_MAX_TEMP (GET/SET VAR)
    MptfFanCrtTemp(u32),  // THM_FAN_CRT_TEMP (GET/SET VAR)
    MptfFanHotTemp(u32),  // THM_FAN_HOT_TEMP (GET/SET VAR PROCHOT notification)
    MptfFanMaxRpm(u32),   // THM_FAN_MAX_RPM (GET/SET VAR)
    MptfFanRpm(u32),      // THM_FAN_RPM (GET VAR)
    MptfDbaLimit(u32),    // THM_DBA_LIMIT (GET/SET VAR)
    MptfSonLimit(u32),    // THM_SON_LIMIT (GET/SET VAR)
    MptfMaLimit(u32),     // THM_MA_LIMIT (GET/SET VAR)

    // RTC fields
    RtcCapability(u32),  // TAS_CAPABILITY (GCP)
    RtcYear(u16),        // TAS_YEAR (GRT/SRT)
    RtcMonth(u8),        // TAS_MONTH (GRT/SRT)
    RtcDay(u8),          // TAS_DAY (GRT/SRT)
    RtcHour(u8),         // TAS_HOUR (GRT/SRT)
    RtcMinute(u8),       // TAS_MINUTE (GRT/SRT)
    RtcSecond(u8),       // TAS_SECOND (GRT/SRT)
    RtcValid(u8),        // TAS_VALID (GRT/SRT)
    RtcMs(u16),          // TAS_MS (GRT/SRT)
    RtcTimeZone(u16),    // TAS_TIME_ZONE (GRT/SRT)
    RtcDaylight(u8),     // TAS_DAYLIGHT (GRT/SRT)
    RtcAlarmStatus(u32), // TAS_ALARM_STATUS (GWS/CWS)
    RtcAcTimeVal(u32),   // TAS_AC_TIME_VAL (STV/TIV)
    RtcDcTimeVal(u32),   // TAS_DC_TIME_VAL (STV/TIV)
}

// TEMPORARILY COPY PASTED
// TODO: get from MFG service
#[derive(Copy, Clone, Debug)]
enum OemMessage {
    ChargeVoltage(MilliVolts),
    ChargeCurrent(MilliAmps),
}

/// Generic to hold OEM messages and standard ACPI messages
/// Can add more as more services have messages
#[derive(Copy, Clone, Debug)]
enum BatteryMsgs {
    Acpi(ESpiMessage),
    Oem(OemMessage),
}

/// Battery Service Errors
#[derive(Copy, Clone, Debug)]
enum BatteryServiceErrors {
    BufferFull,
}

pub struct Service<
    SmartCharger: embedded_batteries_async::charger::Charger, /*, SmartBattery: embedded_batteries_async::smart_battery::SmartBattery*/
> {
    pub endpoint: comms::Endpoint,
    pub charger: charger::Charger<SmartCharger>,
}

impl<SmartCharger: embedded_batteries_async::charger::Charger> Service<SmartCharger> {
    pub fn new(smart_charger: SmartCharger) -> Self {
        Service {
            endpoint: comms::Endpoint::uninit(comms::EndpointID::Internal(comms::Internal::Battery)),
            charger: charger::Charger::new(smart_charger),
        }
    }

    /// Maybe this should exist in app code?
    async fn broadcast_dynamic_acpi_msgs(&self, update_interval_us: u64, messages: &[ESpiMessage]) {
        embassy_time::Timer::after_micros(update_interval_us).await;
        for msg in messages {
            match msg {
                ESpiMessage::BatCycleCount(_) => self.charger.rx.send(BatteryMsgs::Acpi(*msg)).await,
                _ => todo!(),
            }
        }
    }

    fn handle_transport_msg(&self, msg: BatteryMsgs) -> Result<(), BatteryServiceErrors> {
        match msg {
            BatteryMsgs::Acpi(msg) => match msg {
                // Route to charger buffer or fuel gauge buffer
                _ => todo!(),
            },
            BatteryMsgs::Oem(msg) => match msg {
                // Route to charger buffer or fuel gauge buffer
                OemMessage::ChargeVoltage(_) => self
                    .charger
                    .rx
                    .try_send(BatteryMsgs::Oem(msg))
                    .map_err(|_| BatteryServiceErrors::BufferFull),
                _ => todo!(),
            },
        }
    }

    // Select between 2 futures or handle each future in a seperate task?
    async fn handle_charger_fuel_gauge_msg(&self) {
        let charger_fut = self.charger.tx.receive();
    }
}

impl<SmartCharger: embedded_batteries_async::charger::Charger> comms::MailboxDelegate for Service<SmartCharger> {
    /// Wrap the recv'd message in the correct battery message type and pass it on to the fuel gauge or charger
    /// Should this just be a signal? and then Service has an async fn to pass it along to the charger or fuel gauge
    /// Allows handle_transport_msg() to be async fn
    fn receive(&self, message: &comms::Message) {
        if let Some(msg) = message.data.get::<ESpiMessage>() {
            // Todo: Handle case where buffer is full.
            self.handle_transport_msg(BatteryMsgs::Acpi(*msg)).unwrap()
        }

        if let Some(msg) = message.data.get::<OemMessage>() {
            // Todo: error handling
            self.handle_transport_msg(BatteryMsgs::Oem(*msg)).unwrap()
        }
    }
}

static SERVICE: OnceLock<Service> = OnceLock::new();

pub async fn init() {
    let battery_service = SERVICE.get_or_init(|| Service::new(MockCharger {}));

    comms::register_endpoint(battery_service, &battery_service.endpoint)
        .await
        .unwrap();
}
