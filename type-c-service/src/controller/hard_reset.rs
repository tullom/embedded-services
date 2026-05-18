//! Hard Reset port trait implementation
use embedded_services::{event::Sender, sync::Lockable};
use embedded_usb_pd::PdError;
use type_c_interface::controller::hard_reset::HardReset;

use super::*;
use crate::controller::state::SharedState;

impl<
    'device,
    C: Lockable<Inner: Pd + HardReset>,
    Shared: Lockable<Inner = SharedState>,
    TypeCSender: Sender<type_c_interface::service::event::PortEventData>,
    PowerSender: Sender<power_policy_interface::psu::event::EventData>,
    LoopbackSender: Sender<event::Loopback>,
> type_c_interface::port::hard_reset::HardReset for Port<'device, C, Shared, TypeCSender, PowerSender, LoopbackSender>
{
    async fn hard_reset(&mut self) -> Result<(), PdError> {
        self.controller.lock().await.hard_reset(self.port).await
    }
}
