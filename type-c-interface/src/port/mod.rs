//! PD controller related code
use embassy_sync::channel::{DynamicReceiver, DynamicSender};
use embedded_usb_pd::ucsi::lpm;
use embedded_usb_pd::{GlobalPortId, LocalPortId, PdError, ado::Ado};

use embedded_services::ipc::deferred;
use embedded_services::{GlobalRawMutex, intrusive_list};

pub mod electrical_disconnect;
pub mod event;
pub mod max_sink_voltage;
pub mod pd;
pub mod power;
pub mod retimer;
pub mod type_c;
use crate::control::dp::{DpConfig, DpStatus};
use crate::control::pd::PdStateMachineConfig;
use crate::control::retimer::RetimerFwUpdateState;
use crate::control::tbt::TbtConfig;
use crate::control::type_c::TypeCStateMachineState;
use crate::control::usb::UsbControlConfig;
use crate::control::vdm::{AttnVdm, OtherVdm, SendVdm};
use crate::controller::ControllerId;
use crate::service::event::PortEvent as ServicePortEvent;

/// Port-specific command data
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PortCommandData {
    /// Get retimer fw update state
    RetimerFwUpdateGetState,
    /// Set retimer fw update state
    RetimerFwUpdateSetState,
    /// Clear retimer fw update state
    RetimerFwUpdateClearState,
    /// Set retimer compliance
    SetRetimerCompliance,
    /// Reconfigure retimer
    ReconfigureRetimer,
    /// Set the maximum sink voltage in mV for the given port
    SetMaxSinkVoltage(Option<u16>),
    /// Set unconstrained power
    SetUnconstrainedPower(bool),
    /// Clear the dead battery flag for the given port
    ClearDeadBatteryFlag,
    /// Get other VDM
    GetOtherVdm,
    /// Get attention VDM
    GetAttnVdm,
    /// Send VDM
    SendVdm(SendVdm),
    /// Set USB control configuration
    SetUsbControl(UsbControlConfig),
    /// Get DisplayPort status
    GetDpStatus,
    /// Set DisplayPort configuration
    SetDpConfig(DpConfig),
    /// Execute DisplayPort reset
    ExecuteDrst,
    /// Set Thunderbolt configuration
    SetTbtConfig(TbtConfig),
    /// Set PD state-machine configuration
    SetPdStateMachineConfig(PdStateMachineConfig),
    /// Set Type-C state-machine configuration
    SetTypeCStateMachineConfig(TypeCStateMachineState),
    /// Execute the UCSI command
    ExecuteUcsiCommand(lpm::CommandData),
}

/// Port-specific commands
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PortCommand {
    /// Port ID
    pub port: GlobalPortId,
    /// Command data
    pub data: PortCommandData,
}

/// Port-specific response data
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PortResponseData {
    /// Command completed with no error
    Complete,
    /// Retimer Fw Update status
    RtFwUpdateStatus(RetimerFwUpdateState),
    /// PD alert
    PdAlert(Option<Ado>),
    /// Get other VDM
    OtherVdm(OtherVdm),
    /// Get attention VDM
    AttnVdm(AttnVdm),
    /// Get DisplayPort status
    DpStatus(DpStatus),
    /// UCSI response
    UcsiResponse(Result<Option<lpm::ResponseData>, PdError>),
}

impl PortResponseData {
    /// Helper function to convert to a result
    pub fn complete_or_err(self) -> Result<(), PdError> {
        match self {
            PortResponseData::Complete => Ok(()),
            _ => Err(PdError::InvalidResponse),
        }
    }
}

/// Port-specific command response
pub type PortResponse = Result<PortResponseData, PdError>;

/// PD controller command-specific data
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum InternalCommandData {
    /// Reset the PD controller
    Reset,
    /// Sync controller state
    SyncState,
}

/// PD controller command
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    /// Controller specific command
    Controller(InternalCommandData),
    /// Port command
    Port(PortCommand),
}

/// Controller-specific response data
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum InternalResponseData {
    /// Command complete
    Complete,
}

/// Response for controller-specific commands
pub type InternalResponse = Result<InternalResponseData, PdError>;

/// PD controller command response
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Response {
    /// Controller response
    Controller(InternalResponse),
    /// Port response
    Port(PortResponse),
}

/// Per-port registration info
pub struct PortRegistration {
    /// Global port ID of the port
    pub id: GlobalPortId,
    /// Event receiver for the type-C service
    pub receiver: DynamicReceiver<'static, ServicePortEvent>,
    /// Event sender for the type-C service
    pub sender: DynamicSender<'static, ServicePortEvent>,
}

/// PD controller
pub struct Device<'a> {
    node: intrusive_list::Node,
    id: ControllerId,
    pub ports: &'a [PortRegistration],
    num_ports: usize,
    command: deferred::Channel<GlobalRawMutex, Command, Response>,
}

impl intrusive_list::NodeContainer for Device<'static> {
    fn get_node(&self) -> &intrusive_list::Node {
        &self.node
    }
}

impl<'a> Device<'a> {
    /// Create a new PD controller struct
    pub fn new(id: ControllerId, ports: &'a [PortRegistration]) -> Self {
        Self {
            node: intrusive_list::Node::uninit(),
            id,
            ports,
            num_ports: ports.len(),
            command: deferred::Channel::new(),
        }
    }

    /// Get the controller ID
    pub fn id(&self) -> ControllerId {
        self.id
    }

    /// Send a command to this controller
    pub async fn execute_command(&self, command: Command) -> Response {
        self.command.execute(command).await
    }

    /// Check if this controller has the given port
    pub fn has_port(&self, port: GlobalPortId) -> bool {
        self.lookup_local_port(port).is_ok()
    }

    /// Convert a local port ID to a global port ID
    pub fn lookup_global_port(&self, port: LocalPortId) -> Result<GlobalPortId, PdError> {
        Ok(self.ports.get(port.0 as usize).ok_or(PdError::InvalidParams)?.id)
    }

    /// Convert a global port ID to a local port ID
    pub fn lookup_local_port(&self, port: GlobalPortId) -> Result<LocalPortId, PdError> {
        self.ports
            .iter()
            .position(|descriptor| descriptor.id == port)
            .map(|p| LocalPortId(p as u8))
            .ok_or(PdError::InvalidParams)
    }

    /// Create a command handler for this controller
    ///
    /// DROP SAFETY: Direct call to deferred channel primitive
    pub async fn receive(&self) -> deferred::Request<'_, GlobalRawMutex, Command, Response> {
        self.command.receive().await
    }

    /// Number of ports on this controller
    pub fn num_ports(&self) -> usize {
        self.num_ports
    }
}
