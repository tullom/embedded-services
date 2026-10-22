# Battery service

The [battery service](src/lib.rs) answers ACPI queries from
caller-owned fuel-gauge caches. The caller drives the
[FuelGauge interface](../battery-service-interface/src/fuel_gauge.rs); the driver
caches successful reads in its `State`.

## Per-gauge power sources

Each fuel gauge's `State` stores a `PsuState` and an unconstrained-power override
from power-policy notifications.
It distinguishes an attached but idle source from an active consumer or a
detached source. PSU state is kept per gauge, with no service-wide copy.
Fuel gauges and ACPI queries do not hold PSU references or acquire PSU locks.
Fuel-gauge drivers need no extra associated type. Without power-policy
integration, the cache remains offline with unknown output capability.

`_PSR` and `_PIF` use the same `DeviceId` as the fuel gauge and read its cache.
Different gauges can track independent sources, share a source, or receive no
power updates. Reference-based queries are synchronous, like the battery queries:

```rust
let mut gauge = fuel_gauge.lock().await;
let status = service.is_psu_in_use(&mut *gauge)?;
```

- With default routing, `ConsumerConnected` records `ConnectedConsumer` with the
  routed source's capability. `ConsumerDisconnected` records `Detached` when the
  reason is `Detached`, or `Idle` otherwise. A source can stop supplying power
  while still attached. Provider events leave the cache unchanged.
- Unconstrained notifications set or clear the override without replacing the
  cached PSU state. Consumer events refresh it from the capability flags;
  disconnects clear it.
- `_PSR` reports online for `ConnectedConsumer` or unconstrained power.
- `_PIF` reports unlimited output as `0xFFFFFFFE`, a service-specific convention.
  Clearing unconstrained restores the cached consumer's negotiated output in mW,
  which may be below the adapter's rated maximum, or offline status if disconnected.
- Without a consumer or unconstrained override, output power is unknown
  (`0xFFFFFFFF`). Input rating is always unknown. Unassociated or not-yet-reported
  gauges start offline with unknown output. A consumer disconnect clears the
  previous negotiated capability.
- Notifications do not provide identity or redundancy/shared-source information,
  so identity strings are empty and those flags are clear.
- Unregistered IDs return `UnknownDeviceId`, including ID `0` when no gauges are
  registered. Cached external power can still be queried before the battery is
  present or initialized.

## Routing power-policy notifications

Use [`DefaultPowerPolicyRouting`](src/routing.rs) to route
payload-only power-policy notifications from every PSU to fuel gauge `0` without
defining a local routing type:

```rust
use battery_service::{ArrayRegistration, DefaultPowerPolicyRouting, Service};

let service = Service::new(ArrayRegistration {
    fuel_gauges: [fuel_gauge],
    routing: DefaultPowerPolicyRouting,
});
service.process_power_policy_event(&event).await?;
```

Every registration must define
[`Registration::Routing`](src/registration.rs) and provide its
router through `routing()`. `ArrayRegistration` owns the caller-provided `routing`
value. The service uses this router for every notification, so its event handler
accepts only the router's associated `Event` type and takes no per-notification
routing argument. The built-in router accepts `PowerPolicyEventData`; custom
routers can use another notification representation.

With default routing, fuel gauge `0` must be registered. The router applies
notifications to its cache in delivery order and does not aggregate
simultaneous PSU connections.

For custom mappings or event types, implement
[`battery_service::PowerPolicyRouting`](src/routing.rs). Define
only its associated `Event` type to use the trait's consumer-event policy and
gauge-`0` mapping. Override `route` to return `(DeviceId, PowerPolicyUpdate)` updates.
Return no updates to ignore an event, one for an independent source, or multiple
for gauges sharing a source. Each gauge can receive a different state.
Updates can replace a PSU state or set its unconstrained override. The router
chooses which gauges receive each update. The service validates the destinations,
then locks and updates each gauge.

To route by source identity, receive full power-policy `Event` notifications or
an application-tagged payload. The default implementation does not need source
identity, so payload-only `EventData` is enough. The associated event type must
implement `Clone + Into<EventData>` for the trait's default implementation. Route
using immutable source metadata without locking a PSU or the policy. Only state
and capability payloads go into gauge caches. The caches do not retain event
references.

[`battery_service::EventReceiver`](src/event_receiver.rs) wraps
the shared [`PowerPolicySubscriber`](../power-policy-interface/src/service/event.rs)
and supports either event representation. Await
`service.process_power_policy_event(&event)` for each event before processing the
next. The service rejects invalid target IDs before changing any cache.

Updates to multiple gauges run sequentially and are not atomic. Let processing
finish, or retain a cancelled event and retry it before applying newer events.
Cloned routing iterators must yield the same destination/update pairs without
side effects. Stateful routers must reproduce updates on retry without consuming
pending updates or applying an event twice to their policy state.

Power-policy routing does not need its own Embassy task. Like TCPM's event
receiver, `EventReceiver::wait_next_with_poll` can combine power notifications
with the existing battery task's polling deadline:

```rust
let mut events = battery_service::EventReceiver::new(power_policy_receiver);
let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_secs(1));
loop {
    match events.wait_next_with_poll(ticker.next()).await {
        battery_service::Event::Poll => {
            // Run the existing fuel-gauge polling here.
        }
        battery_service::Event::PowerPolicy(event) => {
            service.process_power_policy_event(&event).await?;
        }
    }
}
```

Keep the ticker outside the loop so incoming notifications do not restart the
polling interval. When both sources are ready, priority alternates, starting with
polling, so neither overdue polling nor notification traffic starves the other.
The method selects only the two waits. Finish hardware polling or notification
processing before waiting again to avoid cancelling either operation.
The [standalone battery example](../examples/std/src/bin/battery.rs) uses this
combined loop. Its power-policy channel is ready for a platform notifier, but
the example does not start a power-policy producer.

Register the notifier before starting power policy and deliver every connection
and disconnection in order: notifications are not replayed and the caches start
offline. Do not use a lossy immediate publisher for cache updates.

The [power-policy example](../examples/std/src/bin/power_policy.rs) stores immutable
source IDs alongside PSU mutexes and routes notifications to two independent
gauge caches in the task that polls them. Its `SenderNotifier` uses a bounded
channel; a full queue applies backpressure without dropping events. Routing,
cache updates, and ACPI reads do not acquire a PSU lock, avoiding a dependency
between that lock and the notification queue.
