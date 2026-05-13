//! UCSI LPM port trait implementation
use embedded_services::{event::Sender, sync::Lockable};
use embedded_usb_pd::PdError;
use type_c_interface::ucsi::Lpm as UcsiLpm;

use super::*;
use crate::controller::state::SharedState;

impl<
    'device,
    C: Lockable<Inner: Pd + UcsiLpm>,
    Shared: Lockable<Inner = SharedState>,
    PowerSender: Sender<power_policy_interface::psu::event::EventData>,
    LoopbackSender: Sender<event::Loopback>,
> type_c_interface::ucsi::Lpm for Port<'device, C, Shared, PowerSender, LoopbackSender>
{
    async fn execute_lpm_command(
        &mut self,
        command: embedded_usb_pd::ucsi::lpm::LocalCommand,
    ) -> Result<Option<embedded_usb_pd::ucsi::lpm::ResponseData>, PdError> {
        self.controller.lock().await.execute_lpm_command(command).await
    }
}
