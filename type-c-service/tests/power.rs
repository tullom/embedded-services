#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]
use std::ptr;

use embassy_futures::join::join;
use embassy_time::{TimeoutError, with_timeout};
use embedded_usb_pd::{PowerRole, type_c::ConnectionState};
use power_policy_interface::{
    capability::{ConsumerDisconnect, ConsumerFlags, ConsumerPowerCapability, PsuType},
    service::event::Event as PowerPolicyEvent,
};
use type_c_interface::{
    control::pd::PortStatus,
    port::event::{PortEvent, PortStatusEventBitfield},
    port::max_sink_voltage::MaxSinkVoltage,
    util::POWER_CAPABILITY_5V_1A5,
};
use type_c_interface_mocks::controller::{
    FnCall as ControllerFnCall, max_sink_voltage::FnCall as MaxSinkVoltageFnCall, pd::FnCall as PdFnCall,
};
use type_c_service::controller::event::Event;

use crate::common::{
    DEFAULT_PER_CALL_TIMEOUT, DEFAULT_TEST_DURATION, PowerPolicyServiceReceiver, Test, TestPort, TypeCServiceReceiver,
};

mod common;

/// Test basic consumer attach flow
struct TestBasicConsumerFlow;

impl Test for TestBasicConsumerFlow {
    async fn run<'port, 'ch>(
        &mut self,
        type_c_receiver: TypeCServiceReceiver<'port, 'ch>,
        power_policy_receiver: PowerPolicyServiceReceiver<'port, 'ch>,
        port0: TestPort<'port, 'ch>,
        _port1: TestPort<'port, 'ch>,
        _port2: TestPort<'port, 'ch>,
    ) {
        {
            // Set up the mock to report a sink connection and allow enabling the sink path
            let mut mock0 = port0.mock.lock().await;

            mock0.next_result_get_port_status.push_back(Ok(PortStatus {
                available_sink_contract: Some(POWER_CAPABILITY_5V_1A5),
                connection_state: Some(ConnectionState::Attached),
                power_role: PowerRole::Sink,
                ..Default::default()
            }));
            mock0.next_result_enable_sink_path.push_back(Ok(()));
        }

        // Simulate a plug event and a new consumer contract
        let mut port_event = PortStatusEventBitfield::none();
        port_event.set_plug_inserted_or_removed(true);
        port_event.set_new_power_contract_as_consumer(true);
        port_event.set_sink_ready(true);

        port0
            .port
            .lock()
            .await
            .process_event(Event::PortEvent(PortEvent::StatusChanged(port_event)))
            .await
            .unwrap();

        let (type_c_result, power_policy_result) = join(
            with_timeout(DEFAULT_PER_CALL_TIMEOUT, type_c_receiver.receive()),
            with_timeout(DEFAULT_PER_CALL_TIMEOUT, power_policy_receiver.receive()),
        )
        .await;

        // Shouldn't get any Type-C service events in this flow
        assert_eq!(type_c_result.err(), Some(TimeoutError));

        // Power policy service should broadcast a consumer connect event
        match power_policy_result {
            Ok(PowerPolicyEvent::ConsumerConnected(psu, capability)) => {
                assert_eq!(
                    capability,
                    ConsumerPowerCapability {
                        capability: POWER_CAPABILITY_5V_1A5,
                        flags: ConsumerFlags::none().with_psu_type(PsuType::TypeC),
                    }
                );
                assert!(ptr::eq(psu, port0.port));
            }
            _ => panic!("Did not receive consumer connected event"),
        }

        {
            // Set up the mock to report an unplug
            let mut mock0 = port0.mock.lock().await;
            let port_status = Ok(Default::default());
            mock0.next_result_get_port_status.push_back(port_status);
        }

        // Simulate an unplug event
        let mut port_event = PortStatusEventBitfield::none();
        port_event.set_plug_inserted_or_removed(true);

        port0
            .port
            .lock()
            .await
            .process_event(Event::PortEvent(PortEvent::StatusChanged(port_event)))
            .await
            .unwrap();

        let (type_c_result, power_policy_result) = join(
            with_timeout(DEFAULT_PER_CALL_TIMEOUT, type_c_receiver.receive()),
            with_timeout(DEFAULT_PER_CALL_TIMEOUT, power_policy_receiver.receive()),
        )
        .await;

        // Type-C service currently shouldn't broadcast any events in this flow
        assert_eq!(type_c_result.err(), Some(TimeoutError));
        // Power policy service should broadcast a consumer disconnect event
        match power_policy_result {
            Ok(PowerPolicyEvent::ConsumerDisconnected(psu, _)) => {
                assert!(ptr::eq(psu, port0.port));
            }
            _ => panic!("Did not receive consumer disconnected event"),
        }
    }
}

/// Test that changing the max sink voltage while a consumer is connected disables the sink path and
/// notifies the power policy, which broadcasts a `ConsumerDisconnected` event with the renegotiation
/// flag set. Setting the same voltage should do neither.
struct TestSinkDisableOnVoltageChange;

impl Test for TestSinkDisableOnVoltageChange {
    async fn run<'port, 'ch>(
        &mut self,
        type_c_receiver: TypeCServiceReceiver<'port, 'ch>,
        power_policy_receiver: PowerPolicyServiceReceiver<'port, 'ch>,
        port0: TestPort<'port, 'ch>,
        _port1: TestPort<'port, 'ch>,
        _port2: TestPort<'port, 'ch>,
    ) {
        // Bring up a connected consumer at 5V.
        {
            let mut mock0 = port0.mock.lock().await;
            mock0.next_result_get_port_status.push_back(Ok(PortStatus {
                available_sink_contract: Some(POWER_CAPABILITY_5V_1A5),
                connection_state: Some(ConnectionState::Attached),
                power_role: PowerRole::Sink,
                ..Default::default()
            }));
            // Sink path is enabled when the power policy connects the consumer.
            mock0.next_result_enable_sink_path.push_back(Ok(()));
        }

        let mut port_event = PortStatusEventBitfield::none();
        port_event.set_plug_inserted_or_removed(true);
        port_event.set_new_power_contract_as_consumer(true);
        port_event.set_sink_ready(true);
        port0
            .port
            .lock()
            .await
            .process_event(Event::PortEvent(PortEvent::StatusChanged(port_event)))
            .await
            .unwrap();

        // Wait for the power policy to connect the consumer so the port is in the connected state.
        let (_type_c_result, power_policy_result) = join(
            with_timeout(DEFAULT_PER_CALL_TIMEOUT, type_c_receiver.receive()),
            with_timeout(DEFAULT_PER_CALL_TIMEOUT, power_policy_receiver.receive()),
        )
        .await;

        match power_policy_result {
            Ok(PowerPolicyEvent::ConsumerConnected(psu, _)) => assert!(ptr::eq(psu, port0.port)),
            _ => panic!("Did not receive consumer connected event"),
        }

        // Setting the same voltage as the active contract must not disable the sink path or disconnect.
        {
            let mut mock0 = port0.mock.lock().await;
            mock0.fn_calls.clear();
            mock0.next_result_set_max_sink_voltage.push_back(Ok(()));
        }
        port0.port.lock().await.set_max_sink_voltage(Some(5000)).await.unwrap();
        {
            let mut mock0 = port0.mock.lock().await;
            assert!(
                matches!(
                    mock0.fn_calls.pop_front(),
                    Some(ControllerFnCall::MaxSinkVoltage(
                        MaxSinkVoltageFnCall::SetMaxSinkVoltage(_, Some(5000))
                    ))
                ),
                "expected only the max sink voltage to be set without disabling the sink path"
            );
            assert!(mock0.fn_calls.is_empty());
        }
        // No disconnect should have been broadcast.
        assert!(matches!(
            with_timeout(DEFAULT_PER_CALL_TIMEOUT, power_policy_receiver.receive()).await,
            Err(TimeoutError)
        ));

        // Changing the max sink voltage should disable the sink path and notify the power policy.
        {
            let mut mock0 = port0.mock.lock().await;
            mock0.fn_calls.clear();
            mock0.next_result_enable_sink_path.push_back(Ok(()));
            mock0.next_result_set_max_sink_voltage.push_back(Ok(()));
        }
        port0.port.lock().await.set_max_sink_voltage(Some(9000)).await.unwrap();

        // The power policy should broadcast a consumer disconnect with the renegotiation flag set.
        match with_timeout(DEFAULT_PER_CALL_TIMEOUT, power_policy_receiver.receive()).await {
            Ok(PowerPolicyEvent::ConsumerDisconnected(psu, flags)) => {
                assert!(ptr::eq(psu, port0.port));
                assert_eq!(flags, ConsumerDisconnect::none().with_renegotiation(true));
            }
            _ => panic!("Did not receive consumer disconnected event"),
        }

        // The sink path should have been disabled before the new voltage was applied.
        {
            let mut mock0 = port0.mock.lock().await;
            assert!(
                matches!(
                    mock0.fn_calls.pop_front(),
                    Some(ControllerFnCall::Pd(PdFnCall::EnableSinkPath(_, false)))
                ),
                "expected the sink path to be disabled before the voltage change"
            );
            assert!(
                matches!(
                    mock0.fn_calls.pop_front(),
                    Some(ControllerFnCall::MaxSinkVoltage(
                        MaxSinkVoltageFnCall::SetMaxSinkVoltage(_, Some(9000))
                    ))
                ),
                "expected the max sink voltage to be set"
            );
            assert!(mock0.fn_calls.is_empty());
        }
    }
}

#[tokio::test]
async fn test_basic_consumer_flow() {
    common::run_test(
        DEFAULT_TEST_DURATION,
        Default::default(),
        Default::default(),
        TestBasicConsumerFlow,
    )
    .await;
}

#[tokio::test]
async fn test_sink_disable_on_voltage_change() {
    common::run_test(
        DEFAULT_TEST_DURATION,
        Default::default(),
        Default::default(),
        TestSinkDisableOnVoltageChange,
    )
    .await;
}
