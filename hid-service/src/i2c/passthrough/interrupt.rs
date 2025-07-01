use embassy_sync::{mutex::Mutex, signal::Signal};
use embedded_hal::digital::OutputPin;
use embedded_hal_async::digital::Wait;
use embedded_services::{trace, GlobalRawMutex, SyncCell};

/// This struct manages interrupt signal passthrough
/// When an interrupt from the device occurs the interrupt to the host is assert
/// The interrupt will be deasserted when we receive a request from the host
/// We then ignore any further device interrupts until the response is sent to the host
pub struct InterruptSignal<IN: Wait, OUT: OutputPin> {
    state: SyncCell<InterruptState>,
    int_in: Mutex<GlobalRawMutex, IN>,
    int_out: Mutex<GlobalRawMutex, OUT>,
    signal: Signal<GlobalRawMutex, ()>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InterruptState {
    Idle,
    Asserted,
    Waiting,
    Reset,
}

impl<IN: Wait, OUT: OutputPin> InterruptSignal<IN, OUT> {
    pub fn new(int_in: IN, int_out: OUT) -> Self {
        Self {
            state: SyncCell::new(InterruptState::Idle),
            int_in: Mutex::new(int_in),
            int_out: Mutex::new(int_out),
            signal: Signal::new(),
        }
    }

    /// Deassert the interrupt signal
    pub fn deassert(&self) {
        if self.state.get() == InterruptState::Asserted {
            self.state.set(InterruptState::Waiting);
            self.signal.signal(());
        }
    }

    /// Release the interrupt signal, allowing device interrupts to passthrough again
    pub fn release(&self) {
        if self.state.get() == InterruptState::Waiting {
            self.state.set(InterruptState::Idle);
            self.signal.signal(());
        }
    }

    /// Deassert and release the interrupt signal
    pub fn reset(&self) {
        self.state.set(InterruptState::Reset);
        self.signal.signal(());
    }

    pub async fn process(&self) {
        let mut int_in = self.int_in.lock().await;
        let mut int_out = self.int_out.lock().await;

        trace!("Waiting for interrupt");

        int_in.wait_for_low().await.unwrap();

        int_out.set_low().unwrap();
        self.state.set(InterruptState::Asserted);
        trace!("Interrupt received");

        self.signal.wait().await;
        int_out.set_high().unwrap();
        trace!("Interrupt deasserted");

        if self.state.get() == InterruptState::Reset {
            self.state.set(InterruptState::Idle);
            return;
        }

        self.signal.wait().await;
        self.state.set(InterruptState::Idle);
        trace!("Interrupt cleared");
    }
}
