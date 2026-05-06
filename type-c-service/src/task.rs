use embedded_services::{error, event::Receiver, info, sync::Lockable};
use power_policy_interface::service::event::EventData as PowerPolicyEventData;

use crate::service::{EventReceiver, Service};

/// Task to run the Type-C service, running the default event loop
pub async fn task<PowerReceiver: Receiver<PowerPolicyEventData>>(
    service: &'static impl Lockable<Inner = Service<'static>>,
    mut event_receiver: EventReceiver<'static, PowerReceiver>,
) {
    info!("Starting type-c task");

    loop {
        let event = event_receiver.wait_next().await;
        if let Err(e) = service.lock().await.process_event(event).await {
            error!("Type-C service processing error: {:#?}", e);
        }
    }
}
