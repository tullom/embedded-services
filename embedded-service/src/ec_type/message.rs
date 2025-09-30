//! EC Internal Messages

use mctp_rs::OdpCommandCode;

use crate::ec_type::protocols::{acpi, debug, mptf};

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum CapabilitiesMessage {
    Events(u32),
    FwVersion(super::structure::Version),
    SecureState(u8),
    BootStatus(u8),
    FanMask(u8),
    BatteryMask(u8),
    TempMask(u16),
    KeyMask(u16),
    DebugMask(u16),
}

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeAlarmMessage {
    Events(u32),
    Capability(u32),
    Year(u16),
    Month(u8),
    Day(u8),
    Hour(u8),
    Minute(u8),
    Second(u8),
    Valid(u8),
    Daylight(u8),
    Res1(u8),
    Milli(u16),
    TimeZone(u16),
    Res2(u16),
    AlarmStatus(u32),
    AcTimeVal(u32),
    DcTimeVal(u32),
}

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BatteryMessage {
    Events(u32),
    Status(u32),
    LastFullCharge(u32),
    CycleCount(u32),
    State(u32),
    PresentRate(u32),
    RemainCap(u32),
    PresentVolt(u32),
    PsrState(u32),
    PsrMaxOut(u32),
    PsrMaxIn(u32),
    PeakLevel(u32),
    PeakPower(u32),
    SusLevel(u32),
    SusPower(u32),
    PeakThres(u32),
    SusThres(u32),
    TripThres(u32),
    BmcData(u32),
    BmdData(u32),
    BmdFlags(u32),
    BmdCount(u32),
    ChargeTime(u32),
    RunTime(u32),
    SampleTime(u32),
}

/// ACPI Message, compatible with comms system
#[derive(Debug, Clone, Copy)]
pub struct HostRequest<Command, Payload> {
    /// Command
    pub command: Command,
    /// Status code
    pub status: u8,
    /// Data payload
    pub payload: Payload,
}

/// Notification type to be sent to Host
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NotificationMsg {
    /// Interrupt offset
    pub offset: u8,
}

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ThermalMessage {
    Events(u32),
    CoolMode(u32),
    DbaLimit(u32),
    SonneLimit(u32),
    MaLimit(u32),
    Fan1OnTemp(u32),
    Fan1RampTemp(u32),
    Fan1MaxTemp(u32),
    Fan1CrtTemp(u32),
    Fan1HotTemp(u32),
    Fan1MaxRpm(u32),
    Fan1CurRpm(u32),
    Tmp1Val(u32),
    Tmp1Timeout(u32),
    Tmp1Low(u32),
    Tmp1High(u32),
}

/// Message type that services can send to communicate with the Host.
#[derive(Debug, Clone, Copy)]
pub enum HostMsg<Command, Payload> {
    /// Notification without data. After receivng a notification,
    /// typically the host will request some data from the EC
    Notification(NotificationMsg),
    /// Response to Host request.
    Response(HostRequest<Command, Payload>),
}

/// ODP specific command code that can come in from the host.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OdpCommand {
    /// Battery commands
    Battery(acpi::BatteryCmd),
    /// Thermal commands
    Thermal(mptf::ThermalCmd),
    /// Debug commands
    Debug(debug::DebugCmd),
}

/// Standard Host Request
pub type StdHostRequest = HostRequest<OdpCommand, mctp_rs::Odp>;
/// Standard Host Message
pub type StdHostMsg = HostMsg<OdpCommand, mctp_rs::Odp>;

impl From<OdpCommandCode> for OdpCommand {
    fn from(value: OdpCommandCode) -> Self {
        match value {
            OdpCommandCode::BatteryGetBix => OdpCommand::Battery(acpi::BatteryCmd::GetBix),
            OdpCommandCode::BatteryGetBst => OdpCommand::Battery(acpi::BatteryCmd::GetBst),
            OdpCommandCode::BatteryGetPsr => OdpCommand::Battery(acpi::BatteryCmd::GetPsr),
            OdpCommandCode::BatteryGetPif => OdpCommand::Battery(acpi::BatteryCmd::GetPif),
            OdpCommandCode::BatteryGetBps => OdpCommand::Battery(acpi::BatteryCmd::GetBps),
            OdpCommandCode::BatterySetBtp => OdpCommand::Battery(acpi::BatteryCmd::SetBtp),
            OdpCommandCode::BatterySetBpt => OdpCommand::Battery(acpi::BatteryCmd::SetBpt),
            OdpCommandCode::BatteryGetBpc => OdpCommand::Battery(acpi::BatteryCmd::GetBpc),
            OdpCommandCode::BatterySetBmc => OdpCommand::Battery(acpi::BatteryCmd::SetBmc),
            OdpCommandCode::BatteryGetBmd => OdpCommand::Battery(acpi::BatteryCmd::GetBmd),
            OdpCommandCode::BatteryGetBct => OdpCommand::Battery(acpi::BatteryCmd::GetBct),
            OdpCommandCode::BatteryGetBtm => OdpCommand::Battery(acpi::BatteryCmd::GetBtm),
            OdpCommandCode::BatterySetBms => OdpCommand::Battery(acpi::BatteryCmd::SetBms),
            OdpCommandCode::BatterySetBma => OdpCommand::Battery(acpi::BatteryCmd::SetBma),
            OdpCommandCode::BatteryGetSta => OdpCommand::Battery(acpi::BatteryCmd::GetSta),
            OdpCommandCode::ThermalGetTmp => OdpCommand::Thermal(mptf::ThermalCmd::GetTmp),
            OdpCommandCode::ThermalSetThrs => OdpCommand::Thermal(mptf::ThermalCmd::SetThrs),
            OdpCommandCode::ThermalGetThrs => OdpCommand::Thermal(mptf::ThermalCmd::GetThrs),
            OdpCommandCode::ThermalSetScp => OdpCommand::Thermal(mptf::ThermalCmd::SetScp),
            OdpCommandCode::ThermalGetVar => OdpCommand::Thermal(mptf::ThermalCmd::GetVar),
            OdpCommandCode::ThermalSetVar => OdpCommand::Thermal(mptf::ThermalCmd::SetVar),
            OdpCommandCode::DebugGetMsgs => OdpCommand::Debug(debug::DebugCmd::GetMsgs),
        }
    }
}
