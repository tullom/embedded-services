use embassy_futures::yield_now;
use embassy_sync::pubsub::WaitResult;
use embedded_services::debug;
use embedded_services::type_c::controller::{InternalResponseData, Response};
use embedded_usb_pd::ucsi::lpm;

use super::*;

impl<'a, const N: usize, C: Controller, BACK: Backing<'a>, V: FwOfferValidator> ControllerWrapper<'a, N, C, BACK, V> {
    async fn process_get_pd_alert(&self, local_port: LocalPortId) -> Result<Option<Ado>, PdError> {
        let mut backing = self.backing.lock().await;
        let mut channel = match backing.pd_alert_channel_mut(local_port.0 as usize).await {
            Some(channel) => channel,
            None => return Err(PdError::InvalidPort),
        };

        loop {
            match channel.1.try_next_message() {
                Some(WaitResult::Message(alert)) => return Ok(Some(alert)),
                None => return Ok(None),
                Some(WaitResult::Lagged(count)) => {
                    warn!("Port{}: Lagged PD alert channel: {}", local_port.0, count);
                    // Yield to avoid starving other tasks since we're in a loop and try_next_message isn't async
                    yield_now().await;
                }
            }
        }
    }

    /// Handle a port command
    async fn process_port_command(
        &self,
        controller: &mut C,
        state: &mut InternalState,
        command: &controller::PortCommand,
    ) -> Response<'static> {
        if state.fw_update_state.in_progress() {
            debug!("FW update in progress, ignoring port command");
            return controller::Response::Port(Err(PdError::Busy));
        }

        let local_port = self.pd_controller.lookup_local_port(command.port);
        if local_port.is_err() {
            return controller::Response::Port(Err(PdError::InvalidPort));
        }

        let local_port = local_port.unwrap();
        controller::Response::Port(match command.data {
            controller::PortCommandData::PortStatus(cached) => {
                match controller.get_port_status(local_port, cached).await {
                    Ok(status) => Ok(controller::PortResponseData::PortStatus(status)),
                    Err(e) => match e {
                        Error::Bus(_) => Err(PdError::Failed),
                        Error::Pd(e) => Err(e),
                    },
                }
            }
            controller::PortCommandData::ClearEvents => {
                let event = self.active_events[0].get();
                self.active_events[0].set(PortEvent::none());
                Ok(controller::PortResponseData::ClearEvents(event))
            }
            controller::PortCommandData::RetimerFwUpdateGetState => {
                match controller.get_rt_fw_update_status(local_port).await {
                    Ok(status) => Ok(controller::PortResponseData::RtFwUpdateStatus(status)),
                    Err(e) => match e {
                        Error::Bus(_) => Err(PdError::Failed),
                        Error::Pd(e) => Err(e),
                    },
                }
            }
            controller::PortCommandData::RetimerFwUpdateSetState => {
                match controller.set_rt_fw_update_state(local_port).await {
                    Ok(()) => Ok(controller::PortResponseData::Complete),
                    Err(e) => match e {
                        Error::Bus(_) => Err(PdError::Failed),
                        Error::Pd(e) => Err(e),
                    },
                }
            }
            controller::PortCommandData::RetimerFwUpdateClearState => {
                match controller.clear_rt_fw_update_state(local_port).await {
                    Ok(()) => Ok(controller::PortResponseData::Complete),
                    Err(e) => match e {
                        Error::Bus(_) => Err(PdError::Failed),
                        Error::Pd(e) => Err(e),
                    },
                }
            }
            controller::PortCommandData::SetRetimerCompliance => match controller.set_rt_compliance(local_port).await {
                Ok(()) => Ok(controller::PortResponseData::Complete),
                Err(e) => match e {
                    Error::Bus(_) => Err(PdError::Failed),
                    Error::Pd(e) => Err(e),
                },
            },
            controller::PortCommandData::GetPdAlert => match self.process_get_pd_alert(local_port).await {
                Ok(alert) => Ok(controller::PortResponseData::PdAlert(alert)),
                Err(e) => Err(e),
            },
        })
    }

    async fn process_controller_command(
        &self,
        controller: &mut C,
        state: &mut InternalState,
        command: &controller::InternalCommandData,
    ) -> Response<'static> {
        if state.fw_update_state.in_progress() {
            debug!("FW update in progress, ignoring controller command");
            return controller::Response::Controller(Err(PdError::Busy));
        }

        match command {
            controller::InternalCommandData::Status => {
                let status = controller.get_controller_status().await;
                controller::Response::Controller(status.map(InternalResponseData::Status).map_err(|_| PdError::Failed))
            }
            controller::InternalCommandData::SyncState => {
                let result = controller.sync_state().await;
                controller::Response::Controller(
                    result
                        .map(|_| InternalResponseData::Complete)
                        .map_err(|_| PdError::Failed),
                )
            }
            _ => controller::Response::Controller(Err(PdError::UnrecognizedCommand)),
        }
    }

    /// Handle a PD controller command
    pub(super) async fn process_pd_command(
        &self,
        controller: &mut C,
        state: &mut InternalState,
        command: &controller::Command,
    ) -> Response<'static> {
        match command {
            controller::Command::Port(command) => self.process_port_command(controller, state, command).await,
            controller::Command::Controller(command) => {
                self.process_controller_command(controller, state, command).await
            }
            controller::Command::Lpm(_) => controller::Response::Lpm(lpm::Response::Err(PdError::UnrecognizedCommand)),
        }
    }
}
