//! Max sink voltage port trait implementation
use embedded_services::{event::NonBlockingSender, sync::Lockable};
use embedded_usb_pd::PdError;
use type_c_interface::controller::max_sink_voltage::MaxSinkVoltage;

use super::*;
use crate::controller::state::SharedState;

impl<
    'device,
    C: Lockable<Inner: Pd + MaxSinkVoltage>,
    Shared: Lockable<Inner = SharedState>,
    TypeCSender: NonBlockingSender<type_c_interface::service::event::PortEventData>,
    PowerSender: NonBlockingSender<power_policy_interface::psu::event::EventData>,
    LoopbackSender: NonBlockingSender<event::Loopback>,
> type_c_interface::port::max_sink_voltage::MaxSinkVoltage
    for Port<'device, C, Shared, TypeCSender, PowerSender, LoopbackSender>
{
    async fn set_max_sink_voltage(&mut self, voltage_mv: Option<u16>) -> Result<(), PdError> {
        // A change in the maximum sink voltage can trigger a PD renegotiation. During that transition the
        // source may briefly output a voltage that does not match the active contract, which can cause an
        // overcurrent/overvoltage condition on the sink path. If we currently have a connected consumer and
        // the limit is actually changing (or being removed), disable the sink path before the renegotiation
        // to protect the system. The power policy re-enables the sink path when it connects the consumer to
        // the renegotiated contract.
        let disable_sink_path = match self.psu_state.psu_state {
            PsuState::ConnectedConsumer(capability) => {
                voltage_mv.is_none() || voltage_mv != Some(capability.capability.voltage_mv)
            }
            _ => false,
        };

        let mut controller = self.controller.lock().await;
        if disable_sink_path {
            debug!("({}): Disabling sink path before max sink voltage change", self.name);
            controller.enable_sink_path(self.port, false).await?;
        }
        controller.set_max_sink_voltage(self.port, voltage_mv).await
    }
}
