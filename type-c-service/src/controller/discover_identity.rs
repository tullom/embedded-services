//! Discover Identity port trait implementation
use embedded_services::{event::Sender, sync::Lockable};
use embedded_usb_pd::PdError;
use embedded_usb_pd::vdm::structured::command::discover_identity::{sop, sop_prime};
use type_c_interface::controller::discover_identity::DiscoverIdentity;

use super::*;
use crate::controller::state::SharedState;

impl<
    'device,
    C: Lockable<Inner: Pd + DiscoverIdentity>,
    Shared: Lockable<Inner = SharedState>,
    TypeCSender: Sender<type_c_interface::service::event::PortEventData>,
    PowerSender: Sender<power_policy_interface::psu::event::EventData>,
    LoopbackSender: Sender<event::Loopback>,
> type_c_interface::port::discover_identity::DiscoverIdentity
    for Port<'device, C, Shared, TypeCSender, PowerSender, LoopbackSender>
{
    async fn get_discover_identity_sop_response(&mut self) -> Result<sop::ResponseVdos, PdError> {
        self.controller
            .lock()
            .await
            .get_discover_identity_sop_response(self.port)
            .await
    }

    async fn get_discover_identity_sop_prime_response(&mut self) -> Result<sop_prime::ResponseVdos, PdError> {
        self.controller
            .lock()
            .await
            .get_discover_identity_sop_prime_response(self.port)
            .await
    }
}
