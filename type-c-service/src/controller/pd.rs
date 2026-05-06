//! PD functionality unrelated to power contracts and general port status
use embedded_services::{event::Sender, sync::Lockable};
use type_c_interface::port::{
    Controller,
    event::{VdmData, VdmNotification},
};
use type_c_interface::service::event::{PortEvent as ServicePortEvent, PortEventData as ServicePortEventData};

use super::*;
use crate::controller::state::SharedState;

impl<
    'device,
    C: Lockable<Inner: Controller>,
    Shared: Lockable<Inner = SharedState>,
    PowerSender: Sender<power_policy_interface::psu::event::EventData>,
    LoopbackSender: Sender<event::Loopback>,
> Port<'device, C, Shared, PowerSender, LoopbackSender>
{
    /// Process a VDM event by retrieving the relevant VDM data from the `controller` for the appropriate `port`.
    pub(super) async fn process_vdm_event(
        &mut self,
        event: VdmNotification,
    ) -> Result<ServicePortEventData, Error<<C::Inner as Controller>::BusError>> {
        debug!("({}): Processing VDM event: {:?}", self.name, event);
        let vdm_data = {
            let mut controller = self.controller.lock().await;
            match event {
                VdmNotification::Entered => VdmData::Entered(controller.get_other_vdm(self.port).await?),
                VdmNotification::Exited => VdmData::Exited(controller.get_other_vdm(self.port).await?),
                VdmNotification::OtherReceived => VdmData::ReceivedOther(controller.get_other_vdm(self.port).await?),
                VdmNotification::AttentionReceived => VdmData::ReceivedAttn(controller.get_attn_vdm(self.port).await?),
            }
        };

        let event = ServicePortEventData::Vdm(vdm_data);
        let _ = self
            .context
            .send_port_event(ServicePortEvent {
                port: self.global_port,
                event: ServicePortEventData::Vdm(vdm_data),
            })
            .await;
        Ok(event)
    }

    /// Process a DisplayPort status update by retrieving the current DP status from the `controller` for the appropriate `port`.
    pub(super) async fn process_dp_status_update(
        &mut self,
    ) -> Result<ServicePortEventData, Error<<C::Inner as Controller>::BusError>> {
        debug!("({}): Processing DP status update event", self.name);
        let status = self.controller.lock().await.get_dp_status(self.port).await?;
        let event = ServicePortEventData::DpStatusUpdate(status);
        let _ = self
            .context
            .send_port_event(ServicePortEvent {
                port: self.global_port,
                event,
            })
            .await;
        Ok(event)
    }

    pub(super) async fn process_pd_alert(
        &mut self,
    ) -> Result<Option<ServicePortEventData>, Error<<C::Inner as Controller>::BusError>> {
        let ado = self.controller.lock().await.get_pd_alert(self.port).await?;
        debug!("({}): PD alert: {:#?}", self.name, ado);
        if let Some(ado) = ado {
            let event = ServicePortEventData::Alert(ado);
            let _ = self
                .context
                .send_port_event(ServicePortEvent {
                    port: self.global_port,
                    event,
                })
                .await;
            Ok(Some(event))
        } else {
            // For some reason we didn't read an alert, nothing to do
            Ok(None)
        }
    }
}
