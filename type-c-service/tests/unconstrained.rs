//! Test unconstrained logic of type-C service

use embassy_sync::{mutex::Mutex, pubsub::DynSubscriber};
use embassy_time::Timer;
use embedded_services::{
    GlobalRawMutex, info,
    power::{self, policy::PowerCapability},
    type_c,
};
use embedded_usb_pd::LocalPortId;

use crate::common::{
    DEFAULT_PER_CALL_TIMEOUT, DEFAULT_TEST_DURATION, Test,
    mock::{self, FnCall},
};

mod common;

const CAPABILITY: PowerCapability = PowerCapability {
    voltage_mv: 20000,
    current_ma: 5000,
};

/// Prepares the `set_unconstrained_power` function calls for the given port mocks.
async fn prepare_set_unconstrained_calls(
    port0: &'static Mutex<GlobalRawMutex, mock::ControllerState<'static>>,
    port1: &'static Mutex<GlobalRawMutex, mock::ControllerState<'static>>,
    port2: &'static Mutex<GlobalRawMutex, mock::ControllerState<'static>>,
) {
    port0.lock().await.next_result_set_unconstrained_power.push_back(Ok(()));
    port1.lock().await.next_result_set_unconstrained_power.push_back(Ok(()));
    port2.lock().await.next_result_set_unconstrained_power.push_back(Ok(()));
}

/// Verifies that no `set_unconstrained` function calls have been made on the port.
async fn assert_no_unconstrained_calls(port: &'static Mutex<GlobalRawMutex, mock::ControllerState<'static>>) {
    for call in port.lock().await.fn_calls.drain(..) {
        assert!(!matches!(call, FnCall::SetUnconstrainedPower(_, _)));
    }
}

struct TestUnconstrained;

impl Test for TestUnconstrained {
    async fn run(
        &mut self,
        _type_c_receiver: DynSubscriber<'static, type_c::comms::CommsMessage>,
        _power_policy_event_receiver: DynSubscriber<'static, power::policy::CommsMessage>,
        port0: &'static Mutex<GlobalRawMutex, mock::ControllerState<'static>>,
        port1: &'static Mutex<GlobalRawMutex, mock::ControllerState<'static>>,
        port2: &'static Mutex<GlobalRawMutex, mock::ControllerState<'static>>,
    ) {
        // A single unconstrained port should unconstrain the other ports, except itself
        info!("Connecting port 0, unconstrained");
        prepare_set_unconstrained_calls(port0, port1, port2).await;
        port0.lock().await.next_result_enable_sink_path.push_back(Ok(()));
        port0.lock().await.connect_sink(CAPABILITY, true).await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(
            port0.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), false))
        );
        assert_eq!(
            port1.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );
        assert_eq!(
            port2.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );

        // `set_unconstrained` should not be called on any port
        info!("Connecting port 1, constrained");
        port1.lock().await.connect_sink(CAPABILITY, false).await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(port0.lock().await.fn_calls.drain(..).next_back(), None);
        assert_no_unconstrained_calls(port1).await;
        assert_eq!(port2.lock().await.fn_calls.drain(..).next_back(), None);

        // All ports should become constrained
        info!("Disconnecting port 0");
        prepare_set_unconstrained_calls(port0, port1, port2).await;
        port1.lock().await.next_result_enable_sink_path.push_back(Ok(()));
        port0.lock().await.disconnect().await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(
            port0.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), false))
        );
        assert_eq!(
            port1.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), false))
        );
        assert_eq!(
            port2.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), false))
        );

        // `set_unconstrained` should not be called on any port
        info!("Disconnecting port 1");
        port1.lock().await.disconnect().await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(port0.lock().await.fn_calls.drain(..).next_back(), None);
        assert_no_unconstrained_calls(port1).await;
        assert_eq!(port2.lock().await.fn_calls.drain(..).next_back(), None);

        info!("Connecting port 0, unconstrained");
        prepare_set_unconstrained_calls(port0, port1, port2).await;
        port0.lock().await.next_result_enable_sink_path.push_back(Ok(()));
        port0.lock().await.connect_sink(CAPABILITY, true).await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(
            port0.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), false))
        );
        assert_eq!(
            port1.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );
        assert_eq!(
            port2.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );

        // With multiple unconstrained ports, all ports should have their unconstrained flag set
        info!("Connecting port 1, unconstrained");
        prepare_set_unconstrained_calls(port0, port1, port2).await;
        port1.lock().await.connect_sink(CAPABILITY, true).await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(
            port0.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );
        assert_eq!(
            port1.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );
        assert_eq!(
            port2.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );

        // Still multiple unconstrained attached, all should have unconstrained flag set
        info!("Connecting port 2, unconstrained");
        prepare_set_unconstrained_calls(port0, port1, port2).await;
        port2.lock().await.connect_sink(CAPABILITY, true).await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(
            port0.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );
        assert_eq!(
            port1.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );
        assert_eq!(
            port2.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );

        // Still have multiple unconstrained ports
        info!("Disconnecting port 0");
        prepare_set_unconstrained_calls(port0, port1, port2).await;
        port2.lock().await.next_result_enable_sink_path.push_back(Ok(()));
        port0.lock().await.disconnect().await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(
            port0.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );
        assert_eq!(
            port1.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );
        assert_eq!(
            port2.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );

        // Go down to a single unconstrained port, everything else should have unconstrained set
        info!("Disconnecting port 2");
        prepare_set_unconstrained_calls(port0, port1, port2).await;
        port1.lock().await.next_result_enable_sink_path.push_back(Ok(()));
        port2.lock().await.disconnect().await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(
            port0.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );
        assert_eq!(
            port1.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), false))
        );
        assert_eq!(
            port2.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), true))
        );

        // Nothing left, everything should have unconstrained flag set to false
        info!("Disconnecting port 1");
        prepare_set_unconstrained_calls(port0, port1, port2).await;
        port1.lock().await.disconnect().await;
        Timer::after(DEFAULT_PER_CALL_TIMEOUT).await;
        assert_eq!(
            port0.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), false))
        );
        assert_eq!(
            port1.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), false))
        );
        assert_eq!(
            port2.lock().await.fn_calls.drain(..).next_back(),
            Some(FnCall::SetUnconstrainedPower(LocalPortId(0), false))
        );
    }
}

#[tokio::test]
async fn unconstrained() {
    common::run_test(
        DEFAULT_TEST_DURATION,
        type_c_service::service::config::Config::default(),
        power_policy_service::config::Config::default(),
        TestUnconstrained,
    )
    .await;
}
