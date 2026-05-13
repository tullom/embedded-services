//! Temporary bridge between a controller and the type-C service

use embedded_services::{debug, sync::Lockable};
use embedded_usb_pd::{PdError, ucsi::lpm};
use type_c_interface::controller::{
    Controller,
    pd::{Pd, StateMachine as PdStateMachine},
    retimer::Retimer,
    type_c::StateMachine as TypeCStateMachine,
};
use type_c_interface::port::{self, InternalResponseData, Response};
use type_c_interface::ucsi::Lpm as UcsiLpm;

use crate::bridge::event_receiver::{ControllerCommand, OutputControllerCommand};
pub mod event_receiver;

pub struct Bridge<'device, C: Lockable>
where
    C::Inner: Controller + Pd + PdStateMachine + Retimer + TypeCStateMachine + UcsiLpm,
{
    controller: &'device C,
    registration: &'static port::Device<'static>,
}

impl<'device, C: Lockable> Bridge<'device, C>
where
    C::Inner: Controller + Pd + PdStateMachine + Retimer + TypeCStateMachine + UcsiLpm,
{
    pub fn new(controller: &'device C, registration: &'static port::Device<'static>) -> Self {
        Self {
            controller,
            registration,
        }
    }

    /// Handle a port command
    pub async fn process_port_command(&mut self, command: &port::PortCommand) -> Response {
        let local_port = if let Ok(port) = self.registration.lookup_local_port(command.port) {
            port
        } else {
            debug!("Invalid port: {:?}", command.port);
            return port::Response::Port(Err(PdError::InvalidPort));
        };

        let mut controller = self.controller.lock().await;
        port::Response::Port(match command.data {
            port::PortCommandData::RetimerFwUpdateGetState => controller
                .get_rt_fw_update_status(local_port)
                .await
                .map(port::PortResponseData::RtFwUpdateStatus),
            port::PortCommandData::RetimerFwUpdateSetState => controller
                .set_rt_fw_update_state(local_port)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::RetimerFwUpdateClearState => controller
                .clear_rt_fw_update_state(local_port)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::SetRetimerCompliance => controller
                .set_rt_compliance(local_port)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::ReconfigureRetimer => controller
                .reconfigure_retimer(local_port)
                .await
                .map(|_| port::PortResponseData::Complete),
            // This command isn't sent by the type-C service, disable it for the transition
            port::PortCommandData::SetMaxSinkVoltage(_) => Ok(port::PortResponseData::Complete),
            port::PortCommandData::SetUnconstrainedPower(unconstrained) => controller
                .set_unconstrained_power(local_port, unconstrained)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::ClearDeadBatteryFlag => controller
                .clear_dead_battery_flag(local_port)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::GetOtherVdm => controller.get_other_vdm(local_port).await.map(|vdm| {
                debug!("Port{}: Other VDM: {:?}", local_port.0, vdm);
                port::PortResponseData::OtherVdm(vdm)
            }),
            port::PortCommandData::GetAttnVdm => controller.get_attn_vdm(local_port).await.map(|vdm| {
                debug!("Port{}: Attention VDM: {:?}", local_port.0, vdm);
                port::PortResponseData::AttnVdm(vdm)
            }),
            port::PortCommandData::SendVdm(tx_vdm) => controller
                .send_vdm(local_port, tx_vdm)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::SetUsbControl(config) => controller
                .set_usb_control(local_port, config)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::GetDpStatus => controller.get_dp_status(local_port).await.map(|status| {
                debug!("Port{}: DP Status: {:?}", local_port.0, status);
                port::PortResponseData::DpStatus(status)
            }),
            port::PortCommandData::SetDpConfig(config) => controller
                .set_dp_config(local_port, config)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::ExecuteDrst => controller
                .execute_drst(local_port)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::SetTbtConfig(config) => controller
                .set_tbt_config(local_port, config)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::SetPdStateMachineConfig(config) => controller
                .set_pd_state_machine_config(local_port, config)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::SetTypeCStateMachineConfig(state) => controller
                .set_type_c_state_machine_config(local_port, state)
                .await
                .map(|_| port::PortResponseData::Complete),
            port::PortCommandData::ExecuteUcsiCommand(command_data) => Ok(port::PortResponseData::UcsiResponse(
                controller
                    .execute_lpm_command(lpm::Command::new(local_port, command_data))
                    .await,
            )),
        })
    }

    pub async fn process_controller_command(&mut self, command: &port::InternalCommandData) -> Response {
        let mut controller = self.controller.lock().await;
        match command {
            port::InternalCommandData::SyncState => port::Response::Controller(Ok(InternalResponseData::Complete)),
            port::InternalCommandData::Reset => {
                let result = controller.reset_controller().await;
                port::Response::Controller(result.map(|_| InternalResponseData::Complete))
            }
        }
    }

    /// Handle a PD controller command
    pub async fn process_event(&mut self, command: ControllerCommand<'static>) -> OutputControllerCommand<'static> {
        let response = match command.command {
            port::Command::Port(command) => self.process_port_command(&command).await,
            port::Command::Controller(command) => self.process_controller_command(&command).await,
        };
        OutputControllerCommand {
            request: command,
            response,
        }
    }
}
