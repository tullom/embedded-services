use core::{future::Future, ops::DerefMut};

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel, mutex::Mutex};

use crate::intrusive_list;

use super::PowerCapability;

pub trait ChargeController: embedded_batteries_async::charger::Charger {
    type BusError;

    fn init_charger(&mut self) -> impl Future<Output = Result<(), Self::BusError>>;
    fn is_psu_attached(&mut self) -> impl Future<Output = Result<bool, Self::BusError>>;
}

/// Charger Device ID new type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChargerId(pub u8);

/// Charger Device ID new type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OemStateId(pub u8);

/// Data for a device request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChargerEvent {
    /// Charger finished initialization sequence
    Initialized,
    /// PSU attached and we want to switch to it
    PsuAttached(PowerCapability),
    /// PSU detached
    PsuDetached,
    /// A timeout of some sort was detected
    Timeout,
    /// An error occured on the bus
    BusError,
    /// OEM specific events
    Oem(OemStateId),
}

/// Current state of the charger
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum State {
    /// Device is initializing
    Init,
    /// Device is waiting for an event
    Idle,
    /// PSU is attached and device can charge if desired
    PsuAttached(PowerCapability),
    /// PSU is detached
    PsuDetached,
    /// Device is discharging battery
    Unpowered,
    // TODO: Dead battery revival?
    /// OEM specific state(s)
    Oem(OemStateId),
}

/// Channel size for device requests
pub const CHARGER_CHANNEL_SIZE: usize = 2;

/// Device struct
pub struct Device {
    /// Intrusive list node
    node: intrusive_list::Node,
    /// Device ID
    id: ChargerId,
    /// Current state of the device
    state: Mutex<NoopRawMutex, State>,
    /// Channel for requests to the device
    events: Channel<NoopRawMutex, ChargerEvent, CHARGER_CHANNEL_SIZE>,
    // /// Channel for responses from the device
    // response: Channel<NoopRawMutex, InternalResponseData, CHARGER_CHANNEL_SIZE>,
}

impl Device {
    /// Create a new device
    pub fn new(id: ChargerId) -> Self {
        Self {
            node: intrusive_list::Node::uninit(),
            id,
            state: Mutex::new(State::Init),
            events: Channel::new(),
            // response: Channel::new(),
        }
    }

    /// Get the device ID
    pub fn id(&self) -> ChargerId {
        self.id
    }

    /// Returns the current state of the device
    pub async fn state(&self) -> State {
        *self.state.lock().await
    }

    /// Set the state of the device
    pub async fn set_state(&self, new_state: State) {
        let mut lock = self.state.lock().await;
        let current_state = lock.deref_mut();
        *current_state = new_state;
    }

    /// Wait for an event
    pub async fn wait_event(&self) -> ChargerEvent {
        self.events.receive().await
    }

    /// Send a charger event, typically to change device state
    pub async fn send_event(&self, event: ChargerEvent) {
        self.events.send(event).await;
    }
}

impl intrusive_list::NodeContainer for Device {
    fn get_node(&self) -> &crate::Node {
        &self.node
    }
}

/// Trait for any container that holds a device
pub trait ChargerContainer {
    /// Get the underlying device struct
    fn get_charger(&self) -> &Device;
}

impl ChargerContainer for Device {
    fn get_charger(&self) -> &Device {
        self
    }
}
