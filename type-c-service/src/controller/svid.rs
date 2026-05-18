//! SvidDiscovery port trait implementation
use embedded_services::{event::Sender, sync::Lockable};
use embedded_usb_pd::PdError;
use type_c_interface::{control::svid::DiscoveredSvids, controller::svid::SvidDiscovery};

use super::*;
use crate::controller::state::SharedState;

impl<
    'device,
    C: Lockable<Inner: Pd + SvidDiscovery>,
    Shared: Lockable<Inner = SharedState>,
    TypeCSender: Sender<type_c_interface::service::event::PortEventData>,
    PowerSender: Sender<power_policy_interface::psu::event::EventData>,
    LoopbackSender: Sender<event::Loopback>,
> type_c_interface::port::svid::SvidDiscovery for Port<'device, C, Shared, TypeCSender, PowerSender, LoopbackSender>
{
    async fn get_discovered_svids(&mut self) -> Result<DiscoveredSvids, PdError> {
        self.controller.lock().await.get_discovered_svids(self.port).await
    }
}
