//! Routing of power-policy notifications to fuel gauges.

use battery_service_interface::{DeviceId, fuel_gauge::PsuState};
use power_policy_interface::{capability::DisconnectReason, service::event::EventData as PowerPolicyEventData};

/// A routed update to a fuel gauge's external-power cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PowerPolicyUpdate {
    /// Replace the PSU state and refresh its unconstrained flag.
    PsuState(PsuState),
    /// Set or clear the unconstrained override, preserving the cached PSU state.
    Unconstrained(bool),
}

/// Maps power-policy events to fuel-gauge cache updates.
///
/// Use full or application-tagged [`Self::Event`] values to route by source
/// identity without PSU locks. The service validates all IDs before updating
/// caches under gauge locks.
pub trait PowerPolicyRouting {
    /// Notification type; [`Self::route`] uses its payload by default.
    type Event: Clone + Into<PowerPolicyEventData>;

    /// Returns per-gauge cache updates. An empty iterator ignores the event.
    ///
    /// The default updates registered gauge `0`: consumer connections set
    /// [`PsuState::ConnectedConsumer`]; consumer disconnects set [`PsuState::Detached`]
    /// for [`DisconnectReason::Detached`], otherwise [`PsuState::Idle`].
    /// Unconstrained events set or clear the power override. Other events are ignored.
    /// Override to choose gauges and their updates.
    ///
    /// Iteration must have no side effects; clones must yield identical updates in order.
    /// Retries must reproduce all updates, even if cancellation left some applied.
    /// Retry the same event before processing newer events. Stateful routers must not consume
    /// pending updates or apply an event twice to their policy state on retry.
    fn route(&self, event: &Self::Event) -> impl Iterator<Item = (DeviceId, PowerPolicyUpdate)> + Clone {
        let update = match event.clone().into() {
            PowerPolicyEventData::ConsumerConnected(capability) => {
                Some(PowerPolicyUpdate::PsuState(PsuState::ConnectedConsumer(capability)))
            }
            PowerPolicyEventData::ConsumerDisconnected(disconnect) => {
                Some(PowerPolicyUpdate::PsuState(match disconnect.reason {
                    Some(DisconnectReason::Detached) => PsuState::Detached,
                    _ => PsuState::Idle,
                }))
            }
            PowerPolicyEventData::Unconstrained(state) => Some(PowerPolicyUpdate::Unconstrained(state.unconstrained)),
            _ => None,
        };
        update.map(|update| (DeviceId(0), update)).into_iter()
    }
}

/// Payload-only routing for single-battery controllers.
///
/// Uses [`PowerPolicyRouting::route`] to update gauge `0` in delivery order from
/// every PSU. Assumes only one PSU is attached; simultaneous connections are not aggregated.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultPowerPolicyRouting;

impl PowerPolicyRouting for DefaultPowerPolicyRouting {
    type Event = PowerPolicyEventData;
}

#[cfg(test)]
mod tests {
    use power_policy_interface::{
        capability::{DisconnectFlags, PowerCapability},
        service::UnconstrainedState,
    };

    use super::*;

    #[test]
    fn default_routing_connects_only_gauge_zero() {
        let routing = DefaultPowerPolicyRouting;
        let capability = PowerCapability {
            voltage_mv: 20_000,
            current_ma: 3250,
        }
        .into();
        let event = PowerPolicyEventData::ConsumerConnected(capability);
        let expected = (
            DeviceId(0),
            PowerPolicyUpdate::PsuState(PsuState::ConnectedConsumer(capability)),
        );
        let mut updates = routing.route(&event);
        let cloned = updates.clone();
        assert_eq!(updates.next(), Some(expected));
        assert_eq!(updates.next(), None);
        assert!(cloned.eq([expected]));
    }

    #[test]
    fn default_routing_distinguishes_detachment_from_other_disconnects() {
        let routing = DefaultPowerPolicyRouting;
        for reason in [
            None,
            Some(DisconnectReason::Detached),
            Some(DisconnectReason::Switching),
            Some(DisconnectReason::AutoRenegotiation),
            Some(DisconnectReason::ManualRenegotiation),
            Some(DisconnectReason::RoleSwap),
            Some(DisconnectReason::Reset),
        ] {
            let event = PowerPolicyEventData::ConsumerDisconnected(DisconnectFlags { reason });
            let expected = if reason == Some(DisconnectReason::Detached) {
                PsuState::Detached
            } else {
                PsuState::Idle
            };
            assert!(
                routing
                    .route(&event)
                    .eq([(DeviceId(0), PowerPolicyUpdate::PsuState(expected))])
            );
        }
    }

    #[test]
    fn default_routing_ignores_provider_events() {
        let routing = DefaultPowerPolicyRouting;
        for event in [
            PowerPolicyEventData::ProviderConnected(
                PowerCapability {
                    voltage_mv: 5000,
                    current_ma: 1500,
                }
                .into(),
            ),
            PowerPolicyEventData::ProviderDisconnected,
        ] {
            assert!(routing.route(&event).next().is_none());
        }
    }

    #[test]
    fn default_routing_updates_unconstrained_override() {
        let routing = DefaultPowerPolicyRouting;
        for state in [
            UnconstrainedState::new(true, 2),
            UnconstrainedState::new(true, 0),
            UnconstrainedState::new(false, 2),
            UnconstrainedState::new(false, 0),
        ] {
            let event = PowerPolicyEventData::Unconstrained(state);
            let expected = [(DeviceId(0), PowerPolicyUpdate::Unconstrained(state.unconstrained))];
            let updates = routing.route(&event);
            assert!(updates.clone().eq(expected));
            assert!(updates.eq(expected));
        }
    }
}
