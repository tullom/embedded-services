use embedded_services::{
    event::{Receiver, Sender},
    sync::Lockable,
};
use type_c_interface::port::event::PortEventBitfield;

use crate::controller::{event_receiver::EventReceiver, state};

pub const DEFAULT_POWER_POLICY_CHANNEL_SIZE: usize = 2;
pub const DEFAULT_LOOPBACK_CHANNEL_SIZE: usize = 1;
pub const DEFAULT_INTERRUPT_CHANNEL_SIZE: usize = 4;

/// Components returned from port creation
pub struct PortComponents<
    'a,
    Port,
    SharedState: Lockable<Inner = state::SharedState>,
    PowerPolicyReceveiver: Receiver<power_policy_interface::psu::event::EventData>,
    LoopbackReceiver: Receiver<crate::controller::event::Loopback>,
    InterruptReceiver: Receiver<PortEventBitfield>,
    InterruptSender: Sender<PortEventBitfield>,
> {
    /// Port instance
    pub port: &'a Port,
    /// Power policy event receiver
    pub power_policy_receiver: PowerPolicyReceveiver,
    /// Port event receiver
    pub event_receiver: EventReceiver<'a, SharedState, InterruptReceiver, LoopbackReceiver>,
    /// Interrupt sender
    pub interrupt_sender: InterruptSender,
}

/// Creates a module containing all state for a controller port, based on static cells and channels.
#[macro_export]
macro_rules! define_controller_port_static_cell_channel {
    ($vis:vis, $name:ident, $mutex:ty, $controller:ty) => {
        $vis mod $name {
            use super::*;

            // We prefix all aliases with 'Inner' to avoid potential name conflicts with user code when this macro is invoked
            // Unfortunately, super::$ty is not valid syntax in a macro, so we have to pull in everything with super::*.
            /// Type alias for the power policy sender
            pub type InnerPowerPolicySenderType =
                ::embassy_sync::channel::DynamicSender<'static, ::power_policy_interface::psu::event::EventData>;
            /// Type alias for the power policy receiver
            pub type InnerPowerPolicyReceiverType =
                ::embassy_sync::channel::DynamicReceiver<'static, ::power_policy_interface::psu::event::EventData>;

            /// Type alias for the loopback sender
            pub type InnerLoopbackSenderType =
                ::embassy_sync::channel::DynamicSender<'static, $crate::controller::event::Loopback>;
            /// Type alias for the loopback receiver
            pub type InnerLoopbackReceiverType =
                ::embassy_sync::channel::DynamicReceiver<'static, $crate::controller::event::Loopback>;

            /// Type alias for the interrupt sender
            pub type InnerInterruptReceiverType =
                ::embassy_sync::channel::DynamicReceiver<'static, ::type_c_interface::port::event::PortEventBitfield>;
            /// Type alias for the interrupt receiver
            pub type InnerInterruptSenderType =
                ::embassy_sync::channel::DynamicSender<'static, ::type_c_interface::port::event::PortEventBitfield>;

            /// Type alias for the shared state mutex
            pub type InnerSharedStateType =
                ::embassy_sync::mutex::Mutex<$mutex, $crate::controller::state::SharedState>;
            /// Type alias for the port
            pub type InnerPortType = ::embassy_sync::mutex::Mutex<
                $mutex,
                $crate::controller::Port<
                    'static,
                    // Controller type
                    $controller,
                    // Shared state type
                    InnerSharedStateType,
                    // Power policy event sender type
                    InnerPowerPolicySenderType,
                    // Loopback event sender type
                    InnerLoopbackSenderType,
                >,
            >;

            /// Channel to send events to the power policy service
            static POWER_POLICY_CHANNEL: ::static_cell::StaticCell<
                ::embassy_sync::channel::Channel<
                    $mutex,
                    ::power_policy_interface::psu::event::EventData,
                    { $crate::controller::macros::DEFAULT_POWER_POLICY_CHANNEL_SIZE },
                >,
            > = ::static_cell::StaticCell::new();
            /// Loopback channel
            static LOOPBACK_CHANNEL: ::static_cell::StaticCell<
                ::embassy_sync::channel::Channel<
                    $mutex,
                    $crate::controller::event::Loopback,
                    { $crate::controller::macros::DEFAULT_LOOPBACK_CHANNEL_SIZE },
                >,
            > = ::static_cell::StaticCell::new();
            /// Interrupt channel
            static INTERRUPT_CHANNEL: ::static_cell::StaticCell<
                ::embassy_sync::channel::Channel<
                    $mutex,
                    ::type_c_interface::port::event::PortEventBitfield,
                    { $crate::controller::macros::DEFAULT_INTERRUPT_CHANNEL_SIZE },
                >,
            > = ::static_cell::StaticCell::new();
            /// State shared between the port and event receiver
            static SHARED_STATE: ::static_cell::StaticCell<
                ::embassy_sync::mutex::Mutex<$mutex, $crate::controller::state::SharedState>,
            > = ::static_cell::StaticCell::new();
            /// Port instance
            static PORT: ::static_cell::StaticCell<InnerPortType> = ::static_cell::StaticCell::new();

            pub fn create(
                name: &'static str,
                port: ::embedded_usb_pd::LocalPortId,
                global_port: ::embedded_usb_pd::GlobalPortId,
                config: $crate::controller::config::Config,
                controller: &'static $controller,
                context: &'static type_c_interface::service::context::Context,
            ) -> $crate::controller::macros::PortComponents<
                'static,
                InnerPortType,
                InnerSharedStateType,
                InnerPowerPolicyReceiverType,
                InnerLoopbackReceiverType,
                InnerInterruptReceiverType,
                InnerInterruptSenderType,
            > {
                let shared_state = SHARED_STATE.init(::embassy_sync::mutex::Mutex::new(
                    $crate::controller::state::SharedState::new(),
                ));

                let power_policy_channel = POWER_POLICY_CHANNEL.init(::embassy_sync::channel::Channel::new());
                let power_policy_sender = power_policy_channel.dyn_sender();
                let power_policy_receiver = power_policy_channel.dyn_receiver();

                let loopback_channel = LOOPBACK_CHANNEL.init(::embassy_sync::channel::Channel::new());
                let loopback_sender = loopback_channel.dyn_sender();
                let loopback_receiver = loopback_channel.dyn_receiver();

                let interrupt_channel = INTERRUPT_CHANNEL.init(::embassy_sync::channel::Channel::new());
                let interrupt_sender = interrupt_channel.dyn_sender();
                let interrupt_receiver = interrupt_channel.dyn_receiver();

                let port = PORT.init(::embassy_sync::mutex::Mutex::new($crate::controller::Port::new(
                    name,
                    config,
                    port,
                    global_port,
                    controller,
                    shared_state,
                    power_policy_sender,
                    loopback_sender,
                    context,
                )));
                let event_receiver = $crate::controller::event_receiver::EventReceiver::new(
                    shared_state,
                    interrupt_receiver,
                    loopback_receiver,
                );
                $crate::controller::macros::PortComponents {
                    port,
                    power_policy_receiver,
                    event_receiver,
                    interrupt_sender,
                }
            }
        }
    };
}
