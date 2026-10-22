use core::{future::ready, marker::PhantomData};

use embedded_services::{
    event::{NonBlockingSender, Receiver, Sender},
    sync::Lockable,
};

use crate::{
    capability::{ConsumerPowerCapability, DisconnectFlags, ProviderPowerCapability},
    psu::Psu,
    service::UnconstrainedState,
};

/// Event data broadcast from the service.
///
/// This enum doesn't contain a reference to the device and is suitable
/// for receivers that don't need to know which device triggered the event
/// and allows for receivers that don't need to be generic over the device type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum EventData {
    /// Consumer disconnected
    ConsumerDisconnected(DisconnectFlags),
    /// Consumer connected
    ConsumerConnected(ConsumerPowerCapability),
    /// Provider disconnected
    ProviderDisconnected,
    /// Provider connected
    ProviderConnected(ProviderPowerCapability),
    /// Unconstrained state changed
    Unconstrained(UnconstrainedState),
}

impl<'device, PSU: Lockable> From<Event<'device, PSU>> for EventData
where
    PSU::Inner: Psu,
{
    fn from(value: Event<'device, PSU>) -> Self {
        match value {
            Event::ConsumerDisconnected(_, disconnect) => EventData::ConsumerDisconnected(disconnect),
            Event::ConsumerConnected(_, capability) => EventData::ConsumerConnected(capability),
            Event::ProviderDisconnected(_) => EventData::ProviderDisconnected,
            Event::ProviderConnected(_, capability) => EventData::ProviderConnected(capability),
            Event::Unconstrained(unconstrained) => EventData::Unconstrained(unconstrained),
        }
    }
}

/// Receives power-policy notifications.
///
/// Wraps any [`Receiver`] and forwards its events unchanged.
pub struct PowerPolicySubscriber<E, R: Receiver<E>> {
    receiver: R,
    // Track E without owning it or imposing its Send/Sync requirements.
    _phantom: PhantomData<fn() -> E>,
}

impl<E, R: Receiver<E>> PowerPolicySubscriber<E, R> {
    /// Wrap an event receiver.
    pub fn new(receiver: R) -> Self {
        Self {
            receiver,
            _phantom: PhantomData,
        }
    }
}

impl<E, R: Receiver<E>> Receiver<E> for PowerPolicySubscriber<E, R> {
    fn try_next(&mut self) -> Option<E> {
        self.receiver.try_next()
    }

    fn wait_next(&mut self) -> impl Future<Output = E> {
        self.receiver.wait_next()
    }
}

/// Events broadcast from the service.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Event<'device, PSU: Lockable>
where
    PSU::Inner: Psu,
{
    /// Consumer disconnected
    ConsumerDisconnected(&'device PSU, DisconnectFlags),
    /// Consumer connected
    ConsumerConnected(&'device PSU, ConsumerPowerCapability),
    /// Provider disconnected
    ProviderDisconnected(&'device PSU),
    /// Provider connected
    ProviderConnected(&'device PSU, ProviderPowerCapability),
    /// Unconstrained state changed
    Unconstrained(UnconstrainedState),
}

impl<'device, PSU> Clone for Event<'device, PSU>
where
    PSU: Lockable,
    PSU::Inner: Psu,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'device, PSU> Copy for Event<'device, PSU>
where
    PSU: Lockable,
    PSU::Inner: Psu,
{
}

/// New-type that implements the [`crate::service::notification::Notifier`] trait for any [`NonBlockingSender<Event>`].
///
/// This allows the user to choose blocking/non-blocking behavior when a type supports both.
pub struct NonBlockingSenderNotifier<
    'device,
    PSU: Lockable<Inner: Psu> + 'device,
    S: NonBlockingSender<Event<'device, PSU>>,
> {
    pub sender: S,
    _phantom: PhantomData<&'device PSU>,
}

impl<'device, PSU: Lockable<Inner: Psu>, S: NonBlockingSender<Event<'device, PSU>>>
    NonBlockingSenderNotifier<'device, PSU, S>
{
    /// Create a new [`NonBlockingSenderNotifier`]
    pub fn new(sender: S) -> Self {
        Self {
            sender,
            _phantom: PhantomData,
        }
    }
}

impl<'device, PSU: Lockable<Inner: Psu>, S: NonBlockingSender<Event<'device, PSU>>>
    crate::service::notification::Notifier<'device> for NonBlockingSenderNotifier<'device, PSU, S>
{
    type Psu = PSU;

    fn notify_consumer_disconnected(
        &mut self,
        psu: &'device Self::Psu,
        disconnect: DisconnectFlags,
    ) -> impl Future<Output = Result<(), crate::service::notification::Error>> {
        ready(
            self.sender
                .try_send(Event::ConsumerDisconnected(psu, disconnect))
                .ok_or(crate::service::notification::Error::WouldBlock),
        )
    }

    fn notify_consumer_connected(
        &mut self,
        psu: &'device Self::Psu,
        capability: ConsumerPowerCapability,
    ) -> impl Future<Output = Result<(), crate::service::notification::Error>> {
        ready(
            self.sender
                .try_send(Event::ConsumerConnected(psu, capability))
                .ok_or(crate::service::notification::Error::WouldBlock),
        )
    }

    fn notify_provider_disconnected(
        &mut self,
        psu: &'device Self::Psu,
    ) -> impl Future<Output = Result<(), crate::service::notification::Error>> {
        ready(
            self.sender
                .try_send(Event::ProviderDisconnected(psu))
                .ok_or(crate::service::notification::Error::WouldBlock),
        )
    }

    fn notify_provider_connected(
        &mut self,
        psu: &'device Self::Psu,
        capability: ProviderPowerCapability,
    ) -> impl Future<Output = Result<(), crate::service::notification::Error>> {
        ready(
            self.sender
                .try_send(Event::ProviderConnected(psu, capability))
                .ok_or(crate::service::notification::Error::WouldBlock),
        )
    }

    fn notify_unconstrained(
        &mut self,
        unconstrained: UnconstrainedState,
    ) -> impl Future<Output = Result<(), crate::service::notification::Error>> {
        ready(
            self.sender
                .try_send(Event::Unconstrained(unconstrained))
                .ok_or(crate::service::notification::Error::WouldBlock),
        )
    }
}

impl<'device, PSU: Lockable<Inner: Psu>, S: NonBlockingSender<Event<'device, PSU>>> From<S>
    for NonBlockingSenderNotifier<'device, PSU, S>
{
    fn from(sender: S) -> Self {
        Self {
            sender,
            _phantom: PhantomData,
        }
    }
}

/// New-type that implements the [`crate::service::notification::Notifier`] trait for any [`Sender<Event>`].
///
/// This allows the user to choose blocking/non-blocking behavior when a type supports both.
pub struct SenderNotifier<'device, PSU: Lockable<Inner: Psu> + 'device, S: Sender<Event<'device, PSU>>> {
    pub sender: S,
    _phantom: PhantomData<&'device PSU>,
}

impl<'device, PSU: Lockable<Inner: Psu> + 'device, S: Sender<Event<'device, PSU>>> SenderNotifier<'device, PSU, S> {
    /// Create a new [`SenderNotifier`]
    pub fn new(sender: S) -> Self {
        Self {
            sender,
            _phantom: PhantomData,
        }
    }
}

impl<'device, PSU: Lockable<Inner: Psu> + 'device, S: Sender<Event<'device, PSU>>>
    crate::service::notification::Notifier<'device> for SenderNotifier<'device, PSU, S>
{
    type Psu = PSU;

    async fn notify_consumer_disconnected(
        &mut self,
        psu: &'device Self::Psu,
        disconnect: DisconnectFlags,
    ) -> Result<(), crate::service::notification::Error> {
        self.sender.send(Event::ConsumerDisconnected(psu, disconnect)).await;
        Ok(())
    }

    async fn notify_consumer_connected(
        &mut self,
        psu: &'device Self::Psu,
        capability: ConsumerPowerCapability,
    ) -> Result<(), crate::service::notification::Error> {
        self.sender.send(Event::ConsumerConnected(psu, capability)).await;
        Ok(())
    }

    async fn notify_provider_disconnected(
        &mut self,
        psu: &'device Self::Psu,
    ) -> Result<(), crate::service::notification::Error> {
        self.sender.send(Event::ProviderDisconnected(psu)).await;
        Ok(())
    }

    async fn notify_provider_connected(
        &mut self,
        psu: &'device Self::Psu,
        capability: ProviderPowerCapability,
    ) -> Result<(), crate::service::notification::Error> {
        self.sender.send(Event::ProviderConnected(psu, capability)).await;
        Ok(())
    }

    async fn notify_unconstrained(
        &mut self,
        unconstrained: UnconstrainedState,
    ) -> Result<(), crate::service::notification::Error> {
        self.sender.send(Event::Unconstrained(unconstrained)).await;
        Ok(())
    }
}

impl<'device, PSU: Lockable<Inner: Psu>, S: Sender<Event<'device, PSU>>> From<S> for SenderNotifier<'device, PSU, S> {
    fn from(sender: S) -> Self {
        Self {
            sender,
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::{
        future::{Future, poll_fn},
        pin::pin,
        ptr,
        task::{Context, Poll, Waker},
    };

    use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel, mutex::Mutex};

    use super::{Event, EventData, PowerPolicySubscriber, Receiver, UnconstrainedState};
    use crate::{
        capability::{
            ConsumerPowerCapability, DisconnectFlags, DisconnectReason, PowerCapability, ProviderPowerCapability,
        },
        psu,
    };

    #[derive(Default)]
    struct TestPsu(psu::State);

    impl psu::Psu for TestPsu {
        async fn disconnect(&mut self) -> Result<(), psu::Error> {
            self.0.disconnect(false)
        }

        async fn connect_provider(&mut self, capability: ProviderPowerCapability) -> Result<(), psu::Error> {
            self.0.connect_provider(capability)
        }

        async fn connect_consumer(&mut self, capability: ConsumerPowerCapability) -> Result<(), psu::Error> {
            self.0.connect_consumer(capability)
        }

        fn state(&self) -> &psu::State {
            &self.0
        }

        fn state_mut(&mut self) -> &mut psu::State {
            &mut self.0
        }
    }

    impl embedded_services::named::Named for TestPsu {
        fn name(&self) -> &'static str {
            "Test PSU"
        }
    }

    fn poll_once<E>(receiver: &mut impl Receiver<E>) -> Poll<E> {
        let mut future = pin!(receiver.wait_next());
        future.as_mut().poll(&mut Context::from_waker(Waker::noop()))
    }

    #[test]
    fn subscriber_forwards_event_payloads_in_order() {
        let events = [
            EventData::ConsumerConnected(
                PowerCapability {
                    voltage_mv: 5000,
                    current_ma: 1500,
                }
                .into(),
            ),
            EventData::ConsumerDisconnected(DisconnectFlags {
                reason: Some(DisconnectReason::Switching),
            }),
            EventData::ProviderConnected(
                PowerCapability {
                    voltage_mv: 20_000,
                    current_ma: 5000,
                }
                .into(),
            ),
            EventData::ProviderDisconnected,
            EventData::Unconstrained(UnconstrainedState::new(true, 2)),
        ];
        let channel = Channel::<NoopRawMutex, EventData, 5>::new();
        for event in &events {
            assert!(channel.try_send(event.clone()).is_ok());
        }

        let mut subscriber = PowerPolicySubscriber::new(channel.dyn_receiver());
        for event in events {
            assert_eq!(poll_once(&mut subscriber), Poll::Ready(event));
        }
        assert_eq!(poll_once(&mut subscriber), Poll::Pending);
    }

    #[test]
    fn subscriber_forwards_full_events_with_psu_references() {
        type Psu = Mutex<NoopRawMutex, TestPsu>;
        let first_psu = Psu::new(TestPsu::default());
        let second_psu = Psu::new(TestPsu::default());
        let channel = Channel::<NoopRawMutex, Event<'_, Psu>, 2>::new();
        let capability = PowerCapability {
            voltage_mv: 20_000,
            current_ma: 3250,
        }
        .into();
        let disconnect = DisconnectFlags {
            reason: Some(DisconnectReason::Switching),
        };
        assert!(
            channel
                .try_send(Event::ConsumerConnected(&first_psu, capability))
                .is_ok()
        );
        assert!(
            channel
                .try_send(Event::ConsumerDisconnected(&second_psu, disconnect))
                .is_ok()
        );

        let mut subscriber = PowerPolicySubscriber::new(channel.dyn_receiver());
        assert!(matches!(
            poll_once(&mut subscriber),
            Poll::Ready(Event::ConsumerConnected(psu, received))
                if ptr::eq(psu, &first_psu) && received == capability
        ));
        assert!(matches!(
            subscriber.try_next(),
            Some(Event::ConsumerDisconnected(psu, received))
                if ptr::eq(psu, &second_psu) && received == disconnect
        ));
        assert!(subscriber.try_next().is_none());
    }

    #[test]
    fn subscriber_forwards_non_clone_events() {
        struct NonCloneEvent(u8);

        let channel = Channel::<NoopRawMutex, NonCloneEvent, 1>::new();
        let mut subscriber = PowerPolicySubscriber::new(channel.receiver());
        assert!(channel.try_send(NonCloneEvent(42)).is_ok());
        assert!(matches!(poll_once(&mut subscriber), Poll::Ready(NonCloneEvent(42))));
        assert!(subscriber.try_next().is_none());
    }

    #[test]
    fn subscriber_forwards_nonblocking_receives() {
        let channel = Channel::<NoopRawMutex, EventData, 1>::new();
        let mut subscriber = PowerPolicySubscriber::new(channel.receiver());
        assert_eq!(subscriber.try_next(), None);

        let event = EventData::ConsumerDisconnected(DisconnectFlags::default());
        assert!(channel.try_send(event.clone()).is_ok());
        assert_eq!(subscriber.try_next(), Some(event));
        assert_eq!(subscriber.try_next(), None);
    }

    #[test]
    fn subscriber_preserves_events_after_cancelled_pending_wait() {
        let channel = Channel::<NoopRawMutex, EventData, 1>::new();
        let mut subscriber = PowerPolicySubscriber::new(channel.receiver());
        assert_eq!(poll_once(&mut subscriber), Poll::Pending);

        let event = EventData::ConsumerDisconnected(DisconnectFlags::default());
        assert!(channel.try_send(event.clone()).is_ok());
        assert_eq!(poll_once(&mut subscriber), Poll::Ready(event));
        assert_eq!(subscriber.try_next(), None);
    }

    #[test]
    fn subscriber_preserves_receiver_send_and_sync() {
        struct EmptyReceiver;

        impl Receiver<*const ()> for EmptyReceiver {
            fn try_next(&mut self) -> Option<*const ()> {
                None
            }

            fn wait_next(&mut self) -> impl Future<Output = *const ()> {
                core::future::pending()
            }
        }

        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<PowerPolicySubscriber<*const (), EmptyReceiver>>();
    }

    #[test]
    fn subscriber_accepts_custom_receivers() {
        struct CustomReceiver(Option<EventData>);

        impl Receiver<EventData> for CustomReceiver {
            fn try_next(&mut self) -> Option<EventData> {
                self.0.take()
            }

            fn wait_next(&mut self) -> impl Future<Output = EventData> {
                poll_fn(|_| match self.try_next() {
                    Some(event) => Poll::Ready(event),
                    None => Poll::Pending,
                })
            }
        }

        let event = EventData::ProviderDisconnected;
        let mut subscriber = PowerPolicySubscriber::new(CustomReceiver(Some(event.clone())));
        assert_eq!(poll_once(&mut subscriber), Poll::Ready(event));
        assert_eq!(subscriber.try_next(), None);
    }
}
