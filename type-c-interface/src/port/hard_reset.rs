use embedded_usb_pd::PdError;

use crate::port::pd::Pd;

/// Functionality related to executing a Hard Reset on this port.
pub trait HardReset: Pd {
    /// Execute a Hard Reset on this port.
    fn hard_reset(&mut self) -> impl Future<Output = Result<(), PdError>>;
}
