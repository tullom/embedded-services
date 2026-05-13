use embassy_executor::{Executor, Spawner};
use embassy_sync::channel::Channel;
use embassy_sync::channel::DynamicReceiver;
use embassy_sync::channel::DynamicSender;
use embassy_sync::mutex::Mutex;
use embassy_sync::pubsub::{DynImmediatePublisher, DynSubscriber, PubSubChannel};
use embassy_time::Timer;
use embedded_services::GlobalRawMutex;
use embedded_services::event::MapSender;
use embedded_usb_pd::{GlobalPortId, LocalPortId};
use log::*;
use power_policy_interface::capability::PowerCapability;
use power_policy_interface::charger::mock::ChargerType;
use power_policy_interface::psu;
use power_policy_service::psu::PsuEventReceivers;
use power_policy_service::service::registration::ArrayRegistration;
use static_cell::StaticCell;
use std_examples::type_c::mock_controller::Port;
use std_examples::type_c::mock_controller::{self, InterruptReceiver};
use type_c_interface::controller::ControllerId;
use type_c_interface::port::Device;
use type_c_interface::port::PortRegistration;
use type_c_interface::port::event::PortEventBitfield;
use type_c_interface::service::event::PortEvent as ServicePortEvent;
use type_c_service::bridge::Bridge;
use type_c_service::bridge::event_receiver::EventReceiver as BridgeEventReceiver;
use type_c_service::controller::event_receiver::InterruptReceiver as _;
use type_c_service::controller::event_receiver::{EventReceiver as PortEventReceiver, PortEventSplitter};
use type_c_service::controller::macros::PortComponents;
use type_c_service::controller::state::SharedState;
use type_c_service::define_controller_port_static_cell_channel;
use type_c_service::service::{EventReceiver as ServiceEventReceiver, Service};

const CHANNEL_CAPACITY: usize = 4;

const CONTROLLER0_ID: ControllerId = ControllerId(0);
const PORT0_ID: GlobalPortId = GlobalPortId(0);

const CONTROLLER1_ID: ControllerId = ControllerId(1);
const PORT1_ID: GlobalPortId = GlobalPortId(1);

const CONTROLLER2_ID: ControllerId = ControllerId(2);
const PORT2_ID: GlobalPortId = GlobalPortId(2);

const DELAY_MS: u64 = 1000;

type ControllerType = Mutex<GlobalRawMutex, mock_controller::Controller<'static>>;
type PortType = Mutex<GlobalRawMutex, Port<'static>>;

type PowerPolicySenderType = MapSender<
    power_policy_interface::service::event::Event<'static, PortType>,
    power_policy_interface::service::event::EventData,
    DynImmediatePublisher<'static, power_policy_interface::service::event::EventData>,
    fn(
        power_policy_interface::service::event::Event<'static, PortType>,
    ) -> power_policy_interface::service::event::EventData,
>;

type PowerPolicyReceiverType = DynSubscriber<'static, power_policy_interface::service::event::EventData>;

type PowerPolicyServiceType = Mutex<
    GlobalRawMutex,
    power_policy_service::service::Service<
        'static,
        ArrayRegistration<'static, PortType, 3, PowerPolicySenderType, 1, ChargerType, 0>,
    >,
>;

type ServiceType = Service<'static>;
type SharedStateType = Mutex<GlobalRawMutex, SharedState>;
type PortEventReceiverType = PortEventReceiver<
    'static,
    SharedStateType,
    DynamicReceiver<'static, PortEventBitfield>,
    DynamicReceiver<'static, type_c_service::controller::event::Loopback>,
>;

#[embassy_executor::task(pool_size = 3)]
async fn bridge_task(
    mut event_receiver: BridgeEventReceiver,
    mut bridge: Bridge<'static, Mutex<GlobalRawMutex, mock_controller::Controller<'static>>>,
) -> ! {
    loop {
        let event = event_receiver.wait_next().await;
        let output = bridge.process_event(event).await;
        event_receiver.finalize(output);
    }
}

#[embassy_executor::task(pool_size = 3)]
async fn port_task(mut event_receiver: PortEventReceiverType, port: &'static PortType) {
    loop {
        let event = event_receiver.wait_event().await;
        let output = port.lock().await.process_event(event).await;
        if let Err(e) = output {
            error!("Error processing event: {e:?}");
        }
    }
}

#[embassy_executor::task(pool_size = 3)]
async fn interrupt_splitter_task(
    mut interrupt_receiver: InterruptReceiver<'static>,
    mut interrupt_splitter: PortEventSplitter<1, DynamicSender<'static, PortEventBitfield>>,
) -> ! {
    loop {
        let interrupts = interrupt_receiver.wait_interrupt().await;
        interrupt_splitter.process_interrupts(interrupts).await;
    }
}

#[embassy_executor::task]
async fn task(spawner: Spawner) {
    embedded_services::init().await;

    // Create power policy service
    static CONTROLLER_CONTEXT: StaticCell<type_c_interface::service::context::Context> = StaticCell::new();
    let controller_context = CONTROLLER_CONTEXT.init(type_c_interface::service::context::Context::new());

    static STATE0: StaticCell<mock_controller::ControllerState> = StaticCell::new();
    let state0 = STATE0.init(mock_controller::ControllerState::new());
    static CONTROLLER0: StaticCell<ControllerType> = StaticCell::new();
    let controller0 = CONTROLLER0.init(Mutex::new(mock_controller::Controller::new(state0)));

    static PORT_CHANNEL0: Channel<GlobalRawMutex, ServicePortEvent, CHANNEL_CAPACITY> = Channel::new();
    static PORT_REGISTRATION0: StaticCell<[PortRegistration; 1]> = StaticCell::new();
    let port_registration0 = PORT_REGISTRATION0.init([PortRegistration {
        id: PORT0_ID,
        sender: PORT_CHANNEL0.dyn_sender(),
        receiver: PORT_CHANNEL0.dyn_receiver(),
    }]);

    static PD_REGISTRATION0: StaticCell<Device<'static>> = StaticCell::new();
    let pd_registration0 = PD_REGISTRATION0.init(Device::new(CONTROLLER0_ID, port_registration0));

    controller_context.register_controller(pd_registration0).unwrap();

    define_controller_port_static_cell_channel!(pub(self), port0, GlobalRawMutex, Mutex<GlobalRawMutex, mock_controller::Controller<'static>>);
    let PortComponents {
        port: port0,
        power_policy_receiver: policy_receiver0,
        event_receiver: event_receiver0,
        interrupt_sender: port0_interrupt_sender,
    } = port0::create(
        "PD0",
        LocalPortId(0),
        PORT0_ID,
        Default::default(),
        controller0,
        controller_context,
    );
    let bridge_receiver0 = BridgeEventReceiver::new(pd_registration0);
    let bridge0 = Bridge::new(controller0, pd_registration0);

    static STATE1: StaticCell<mock_controller::ControllerState> = StaticCell::new();
    let state1 = STATE1.init(mock_controller::ControllerState::new());
    static CONTROLLER1: StaticCell<ControllerType> = StaticCell::new();
    let controller1 = CONTROLLER1.init(Mutex::new(mock_controller::Controller::new(state1)));

    static PORT1_CHANNEL: Channel<GlobalRawMutex, ServicePortEvent, CHANNEL_CAPACITY> = Channel::new();
    static PORT_REGISTRATION1: StaticCell<[PortRegistration; 1]> = StaticCell::new();
    let port_registration1 = PORT_REGISTRATION1.init([PortRegistration {
        id: PORT1_ID,
        sender: PORT1_CHANNEL.dyn_sender(),
        receiver: PORT1_CHANNEL.dyn_receiver(),
    }]);

    static PD_REGISTRATION1: StaticCell<Device<'static>> = StaticCell::new();
    let pd_registration1 = PD_REGISTRATION1.init(Device::new(CONTROLLER1_ID, port_registration1));

    controller_context.register_controller(pd_registration1).unwrap();

    define_controller_port_static_cell_channel!(pub(self), port1, GlobalRawMutex, Mutex<GlobalRawMutex, mock_controller::Controller<'static>>);
    let PortComponents {
        port: port1,
        power_policy_receiver: policy_receiver1,
        event_receiver: event_receiver1,
        interrupt_sender: port1_interrupt_sender,
    } = port1::create(
        "PD1",
        LocalPortId(0),
        PORT1_ID,
        Default::default(),
        controller1,
        controller_context,
    );
    let bridge_receiver1 = BridgeEventReceiver::new(pd_registration1);
    let bridge1 = Bridge::new(controller1, pd_registration1);

    static STATE2: StaticCell<mock_controller::ControllerState> = StaticCell::new();
    let state2 = STATE2.init(mock_controller::ControllerState::new());
    static CONTROLLER2: StaticCell<ControllerType> = StaticCell::new();
    let controller2 = CONTROLLER2.init(Mutex::new(mock_controller::Controller::new(state2)));

    static PORT2_CHANNEL: Channel<GlobalRawMutex, ServicePortEvent, CHANNEL_CAPACITY> = Channel::new();
    static PORT_REGISTRATION2: StaticCell<[PortRegistration; 1]> = StaticCell::new();
    let port_registration2 = PORT_REGISTRATION2.init([PortRegistration {
        id: PORT2_ID,
        sender: PORT2_CHANNEL.dyn_sender(),
        receiver: PORT2_CHANNEL.dyn_receiver(),
    }]);

    static PD_REGISTRATION2: StaticCell<Device<'static>> = StaticCell::new();
    let pd_registration2 = PD_REGISTRATION2.init(Device::new(CONTROLLER2_ID, port_registration2));

    controller_context.register_controller(pd_registration2).unwrap();

    define_controller_port_static_cell_channel!(pub(self), port2, GlobalRawMutex, Mutex<GlobalRawMutex, mock_controller::Controller<'static>>);
    let PortComponents {
        port: port2,
        power_policy_receiver: policy_receiver2,
        event_receiver: event_receiver2,
        interrupt_sender: port2_interrupt_sender,
    } = port2::create(
        "PD2",
        LocalPortId(0),
        PORT2_ID,
        Default::default(),
        controller2,
        controller_context,
    );
    let bridge_receiver2 = BridgeEventReceiver::new(pd_registration2);
    let bridge2 = Bridge::new(controller2, pd_registration2);

    // The service is the only receiver and we only use a DynImmediatePublisher, which doesn't take a publisher slot
    static POWER_POLICY_CHANNEL: StaticCell<
        PubSubChannel<GlobalRawMutex, power_policy_interface::service::event::EventData, 4, 1, 0>,
    > = StaticCell::new();

    let power_policy_channel = POWER_POLICY_CHANNEL.init(PubSubChannel::new());
    let power_policy_sender: PowerPolicySenderType =
        MapSender::new(power_policy_channel.dyn_immediate_publisher(), |e| e.into());
    // Guaranteed to not panic since we initialized the channel above
    let power_policy_subscriber = power_policy_channel.dyn_subscriber().unwrap();

    let power_policy_registration = ArrayRegistration {
        psus: [port0, port1, port2],
        service_senders: [power_policy_sender],
        chargers: [],
    };

    static POWER_SERVICE: StaticCell<PowerPolicyServiceType> = StaticCell::new();
    let power_service = POWER_SERVICE.init(Mutex::new(power_policy_service::service::Service::new(
        power_policy_registration,
        power_policy_service::service::config::Config::default(),
    )));

    // Create type-c service
    static TYPE_C_SERVICE: StaticCell<Mutex<GlobalRawMutex, ServiceType>> = StaticCell::new();
    let type_c_service = TYPE_C_SERVICE.init(Mutex::new(Service::create(Default::default(), controller_context)));

    spawner.spawn(
        power_policy_task(
            PsuEventReceivers::new(
                [port0, port1, port2],
                [policy_receiver0, policy_receiver1, policy_receiver2],
            ),
            power_service,
        )
        .expect("Failed to create power policy task"),
    );
    spawner.spawn(
        type_c_service_task(
            type_c_service,
            ServiceEventReceiver::new(controller_context, power_policy_subscriber),
        )
        .expect("Failed to create type-c service task"),
    );

    spawner.spawn(bridge_task(bridge_receiver0, bridge0).expect("Failed to create bridge0 task"));
    spawner.spawn(bridge_task(bridge_receiver1, bridge1).expect("Failed to create bridge1 task"));
    spawner.spawn(bridge_task(bridge_receiver2, bridge2).expect("Failed to create bridge2 task"));
    spawner.spawn(port_task(event_receiver0, port0).expect("Failed to create controller0 task"));
    spawner.spawn(
        interrupt_splitter_task(
            state0.create_interrupt_receiver(),
            PortEventSplitter::new([port0_interrupt_sender]),
        )
        .expect("Failed to create interrupt splitter 0 task"),
    );
    spawner.spawn(port_task(event_receiver1, port1).expect("Failed to create controller1 task"));
    spawner.spawn(
        interrupt_splitter_task(
            state1.create_interrupt_receiver(),
            PortEventSplitter::new([port1_interrupt_sender]),
        )
        .expect("Failed to create interrupt splitter 1 task"),
    );
    spawner.spawn(port_task(event_receiver2, port2).expect("Failed to create controller2 task"));
    spawner.spawn(
        interrupt_splitter_task(
            state2.create_interrupt_receiver(),
            PortEventSplitter::new([port2_interrupt_sender]),
        )
        .expect("Failed to create interrupt splitter 2 task"),
    );

    const CAPABILITY: PowerCapability = PowerCapability {
        voltage_mv: 20000,
        current_ma: 5000,
    };

    // Wait for controller to be registered
    Timer::after_secs(1).await;

    info!("Connecting port 0, unconstrained");
    state0.connect_sink(CAPABILITY, true).await;
    Timer::after_millis(DELAY_MS).await;

    info!("Connecting port 1, constrained");
    state1.connect_sink(CAPABILITY, false).await;
    Timer::after_millis(DELAY_MS).await;

    info!("Disconnecting port 0");
    state0.disconnect().await;
    Timer::after_millis(DELAY_MS).await;

    info!("Disconnecting port 1");
    state1.disconnect().await;
    Timer::after_millis(DELAY_MS).await;

    info!("Connecting port 0, unconstrained");
    state0.connect_sink(CAPABILITY, true).await;
    Timer::after_millis(DELAY_MS).await;

    info!("Connecting port 1, unconstrained");
    state1.connect_sink(CAPABILITY, true).await;
    Timer::after_millis(DELAY_MS).await;

    info!("Connecting port 2, unconstrained");
    state2.connect_sink(CAPABILITY, true).await;
    Timer::after_millis(DELAY_MS).await;

    info!("Disconnecting port 0");
    state0.disconnect().await;
    Timer::after_millis(DELAY_MS).await;

    info!("Disconnecting port 1");
    state1.disconnect().await;
    Timer::after_millis(DELAY_MS).await;

    info!("Disconnecting port 2");
    state2.disconnect().await;
    Timer::after_millis(DELAY_MS).await;
}

#[embassy_executor::task]
async fn power_policy_task(
    psu_events: PsuEventReceivers<'static, 3, PortType, DynamicReceiver<'static, psu::event::EventData>>,
    power_policy: &'static PowerPolicyServiceType,
) {
    power_policy_service::service::task::psu_task(psu_events, power_policy).await;
}

#[embassy_executor::task]
async fn type_c_service_task(
    service: &'static Mutex<GlobalRawMutex, ServiceType>,
    event_receiver: ServiceEventReceiver<'static, PowerPolicyReceiverType>,
) {
    info!("Starting type-c task");
    type_c_service::task::task(service, event_receiver).await;
}

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Trace).init();

    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(task(spawner).expect("Failed to create task"));
    });
}
