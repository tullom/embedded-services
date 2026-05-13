//! Comms service message definitions

use embedded_usb_pd::{GlobalPortId, ado::Ado};

use crate::{
    control::{dp::DpStatus, pd::PortStatus},
    port::event::{PortStatusEventBitfield, VdmData},
};

/// Struct containing data for a [`PortEventData::StatusChanged`] event
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StatusChangedData {
    /// Status changed event
    pub status_event: PortStatusEventBitfield,
    /// Previous port status
    pub previous_status: PortStatus,
    /// Current port status
    pub current_status: PortStatus,
}

/// Enum to contain all port event variants
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PortEventData {
    /// Port status change events
    StatusChanged(StatusChangedData),
    /// PD alert
    Alert(Ado),
    /// VDM
    Vdm(VdmData),
    /// Discover mode completed
    DiscoverModeCompleted,
    /// USB mux error recovery
    UsbMuxErrorRecovery,
    /// DP status update
    DpStatusUpdate(DpStatus),
}

/// Struct containing a complete port event
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PortEvent {
    pub port: GlobalPortId,
    pub event: PortEventData,
}

/// Message generated when a debug accessory is connected or disconnected
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DebugAccessory {
    /// Port
    pub port: GlobalPortId,
    /// Connected
    pub connected: bool,
}

/// UCSI connector change message
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UsciChangeIndicator {
    /// Port
    pub port: GlobalPortId,
    /// Notify OPM
    pub notify_opm: bool,
}

/// Top-level comms message
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Event {
    /// Debug accessory message
    DebugAccessory(DebugAccessory),
    /// UCSI CCI message
    UcsiCci(UsciChangeIndicator),
}
