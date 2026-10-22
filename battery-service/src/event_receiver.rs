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

#[cfg(test)]
mod tests {
    use core::{
        future::{Future, pending, ready},
        pin::pin,
        task::{Context, Waker},
    };

    use embassy_futures::block_on;
    use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
    use power_policy_interface::capability::{DisconnectFlags, PowerCapability};

    use super::{Event, EventReceiver, PowerPolicyEventData, Receiver};

    #[test]
    fn combined_wait_polls_without_power_policy_notifications() {
        let channel = Channel::<NoopRawMutex, PowerPolicyEventData, 1>::new();
        let mut receiver = EventReceiver::new(channel.receiver());
        assert_eq!(block_on(receiver.wait_next_with_poll(ready(()))), Event::Poll);
        assert_eq!(receiver.try_next(), None);
    }

    #[test]
    fn ready_poll_preserves_queued_notifications_in_order() {
        let channel = Channel::<NoopRawMutex, PowerPolicyEventData, 2>::new();
        let mut receiver = EventReceiver::new(channel.receiver());
        let connected = PowerPolicyEventData::ConsumerConnected(
            PowerCapability {
                voltage_mv: 20_000,
                current_ma: 3250,
            }
            .into(),
        );
        let disconnected = PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default());
        assert!(channel.try_send(connected.clone()).is_ok());
        assert!(channel.try_send(disconnected.clone()).is_ok());

        assert_eq!(block_on(receiver.wait_next_with_poll(ready(()))), Event::Poll);
        assert_eq!(
            block_on(receiver.wait_next_with_poll(pending())),
            Event::PowerPolicy(connected)
        );
        assert_eq!(
            block_on(receiver.wait_next_with_poll(pending())),
            Event::PowerPolicy(disconnected)
        );
        assert_eq!(receiver.try_next(), None);
    }

    #[test]
    fn overdue_poll_does_not_starve_queued_notifications() {
        let channel = Channel::<NoopRawMutex, PowerPolicyEventData, 2>::new();
        let mut receiver = EventReceiver::new(channel.receiver());
        let events = [
            PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default()),
            PowerPolicyEventData::ProviderDisconnected,
        ];
        for event in &events {
            assert!(channel.try_send(event.clone()).is_ok());
        }

        for event in events {
            assert_eq!(block_on(receiver.wait_next_with_poll(ready(()))), Event::Poll);
            assert_eq!(
                block_on(receiver.wait_next_with_poll(ready(()))),
                Event::PowerPolicy(event)
            );
        }
        assert_eq!(receiver.try_next(), None);
    }

    #[test]
    fn combined_wait_preserves_notifications_after_cancellation() {
        let channel = Channel::<NoopRawMutex, PowerPolicyEventData, 1>::new();
        let mut receiver = EventReceiver::new(channel.receiver());
        {
            let mut wait = pin!(receiver.wait_next_with_poll(pending()));
            assert!(wait.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
        }

        let disconnected = PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default());
        assert!(channel.try_send(disconnected.clone()).is_ok());
        assert_eq!(
            block_on(receiver.wait_next_with_poll(pending())),
            Event::PowerPolicy(disconnected)
        );
        assert_eq!(block_on(receiver.wait_next_with_poll(ready(()))), Event::Poll);
        assert_eq!(receiver.try_next(), None);
    }

    #[test]
    fn receiver_preserves_events_after_cancelled_wait() {
        let channel = Channel::<NoopRawMutex, PowerPolicyEventData, 1>::new();
        let mut receiver = EventReceiver::new(channel.receiver());
        {
            let mut wait = pin!(receiver.wait_next());
            assert!(wait.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
        }

        let connected = PowerPolicyEventData::ConsumerConnected(
            PowerCapability {
                voltage_mv: 20_000,
                current_ma: 3250,
            }
            .into(),
        );
        assert!(channel.try_send(connected.clone()).is_ok());
        assert_eq!(block_on(receiver.wait_next()), connected);

        let disconnected = PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags::default());
        assert!(channel.try_send(disconnected.clone()).is_ok());
        assert_eq!(receiver.try_next(), Some(disconnected));
        assert_eq!(receiver.try_next(), None);
    }
}
