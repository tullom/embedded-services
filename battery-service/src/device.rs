use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embedded_services::{Node, NodeContainer};

enum FuelGaugeError {
    BusError,
}

enum Command {
    Initialize,
}

enum InternalResponse {
    Complete,
}

type Response = Result<InternalResponse, FuelGaugeError>;

pub struct Device {
    node: embedded_services::Node,
    id: u8,
    command: Channel<NoopRawMutex, Command, 1>,
    response: Channel<NoopRawMutex, Response, 1>,
}

impl Device {
    pub fn id(&self) -> u8 {
        self.id
    }
}

impl NodeContainer for Device {
    fn get_node(&self) -> &Node {
        &self.node
    }
}
