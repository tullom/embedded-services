use crate::device;
use crate::device::Device;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::once_lock::OnceLock;
use embedded_services::buffer;
use embedded_services::error;
use embedded_services::intrusive_list;
use embedded_services::IntrusiveList;

pub enum State {
    Init,

    Polling,

    NotPresent,

    NotOperational,
}

pub enum Event {
    DoInit,
    Oem(u8, &'static [u8]),
}

// buffer::define_static_buffer!(battery_cache, BatteryMsgs);

pub trait BatterySequence {
    // TODO: make a macro, similar to power sequence
    async fn init_oem_override(state: State, event: Event) -> bool;
    async fn init_oem(devices: &mut [Device], event: Event);

    async fn polling_oem_override(state: State, event: Event) -> bool;
    async fn polling_oem(devices: &mut [Device], event: Event);
}

pub struct Context<B: BatterySequence> {
    fuel_gauges: IntrusiveList,
    state: State,
    oem: B,
    battery_request: Channel<NoopRawMutex, u8, 1>,
    battery_response: Channel<NoopRawMutex, u8, 1>,
}

impl<B: BatterySequence> Context<B> {
    pub fn new(oem: B) -> Self {
        Self {
            fuel_gauges: IntrusiveList::new(),
            state: State::Init,
            oem,
            battery_request: Channel::new(),
            battery_response: Channel::new(),
        }
    }

    fn do_state_machine(&mut self, event: Event) {
        match self.state {
            State::Init => {
                // Check if OEM wants to run custom logic

                // if true, OEM stuff and bypass std stuff

                // else, Std stuff
            }
            State::Polling => todo!(),
            State::NotPresent => todo!(),
            State::NotOperational => todo!(),
        }
    }
}

// static CONTEXT: OnceLock<Context<B>> = OnceLock::new();

// /// Init battery service
// pub fn init() {
//     CONTEXT.get_or_init(Context::new);
// }

// async fn get_fuel_gauge(id: u8) -> Option<&'static Device> {
//     for device in &CONTEXT.get().await.fuel_gauges {
//         if let Some(data) = device.data::<Device>() {
//             if data.id() == id {
//                 return Some(data);
//             }
//         } else {
//             error!("Non-device located in devices list");
//         }
//     }
//     None
// }

// pub async fn register_fuel_gauge(device: &'static Device) -> Result<(), intrusive_list::Error> {
//     if get_fuel_gauge(device.id()).await.is_some() {
//         return Err(embedded_services::Error::NodeAlreadyInList);
//     }

//     CONTEXT.get().await.fuel_gauges.push(device)
// }
