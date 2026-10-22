use embassy_futures::select::{Either, select};
use embedded_services::event::Receiver;
use power_policy_interface::service::event::{EventData as PowerPolicyEventData, PowerPolicySubscriber};

/// Events handled by a battery loop that polls fuel gauges and receives power-policy notifications.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Event<E = PowerPolicyEventData> {
    /// The caller's polling deadline is ready.
    Poll,
    /// A power-policy notification to route through the service.
    PowerPolicy(E),
}

/// Receives power-policy notifications for the battery service.
///
/// The subscriber wraps a caller-provided receiver and is independent of
/// [`Service`](crate::Service). Use full events to route by source identity,
/// or payload events for a dedicated source. The receiver's event type
/// must match [`PowerPolicyRouting::Event`](crate::PowerPolicyRouting::Event)
/// for the registration's router.
///
/// Deliver every event in order to [`Service::process_power_policy_event`](crate::Service::process_power_policy_event).
/// The handler updates gauge caches without locking a PSU, so bounded channels
/// can apply backpressure independently of the PSU lock used by live queries.
///
/// Use [`Self::wait_next_with_poll`] to handle notifications and periodic
/// fuel-gauge polling in an existing task.
pub struct EventReceiver<PowerReceiver: Receiver<E>, E = PowerPolicyEventData> {
    power_policy_event_subscriber: PowerPolicySubscriber<E, PowerReceiver>,
    poll_first: bool,
}

impl<PowerReceiver: Receiver<E>, E> EventReceiver<PowerReceiver, E> {
    /// Create a battery event receiver from power-policy notifications.
    pub fn new(power_policy_event_receiver: PowerReceiver) -> Self {
        Self {
            power_policy_event_subscriber: PowerPolicySubscriber::new(power_policy_event_receiver),
            poll_first: true,
        }
    }

    /// Wait for a polling deadline or a power-policy notification.
    ///
    /// Pass a cancellation-safe wait such as [`embassy_time::Ticker::next`].
    /// Keep the ticker outside the event loop so notifications do not reset its
    /// deadline. When both sources are ready, priority alternates between polling
    /// and notifications, starting with polling, so neither a busy notification
    /// stream nor overdue sampling can starve the other source.
    ///
    /// The losing wait is dropped. Do not pass hardware updates or event
    /// processing as `poll`; finish those operations after this method returns.
    /// The underlying notification receiver must also support cancelled waits.
    pub async fn wait_next_with_poll(&mut self, poll: impl Future<Output = ()>) -> Event<E> {
        let event = if self.poll_first {
            match select(poll, self.power_policy_event_subscriber.wait_next()).await {
                Either::First(()) => Event::Poll,
                Either::Second(event) => Event::PowerPolicy(event),
            }
        } else {
            match select(self.power_policy_event_subscriber.wait_next(), poll).await {
                Either::First(event) => Event::PowerPolicy(event),
                Either::Second(()) => Event::Poll,
            }
        };
        self.poll_first = matches!(&event, Event::PowerPolicy(_));
        event
    }
}

impl<PowerReceiver: Receiver<E>, E> Receiver<E> for EventReceiver<PowerReceiver, E> {
    fn try_next(&mut self) -> Option<E> {
        self.power_policy_event_subscriber.try_next()
    }

    fn wait_next(&mut self) -> impl Future<Output = E> {
        self.power_policy_event_subscriber.wait_next()
    }
}
