use crate::intrusive_list;

/// Charger Device ID new type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DeviceId(pub u8);

enum State {
    /// Device is uninitialized, and will be initialized
    Init,
    /// TODO: Remove state?
    Idle,
    /// Device is charging battery from a power source
    Charging,
    /// Device is discharging battery
    Discharging,
    // TODO: Dead battery revival?
}

/// Device struct
pub struct Device {
    /// Intrusive list node
    node: intrusive_list::Node,
    /// Device ID
    id: DeviceId,
    /// Current state of the device
    state: State,
    // /// Channel for requests to the device
    // request: Channel<NoopRawMutex, RequestData, DEVICE_CHANNEL_SIZE>,
    // /// Channel for responses from the device
    // response: Channel<NoopRawMutex, InternalResponseData, DEVICE_CHANNEL_SIZE>,
}

impl intrusive_list::NodeContainer for Device {
    fn get_node(&self) -> &crate::Node {
        &self.node
    }
}
