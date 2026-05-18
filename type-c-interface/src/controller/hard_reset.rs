use embedded_usb_pd::{LocalPortId, PdError};

use crate::controller::pd::Pd;

/// Functionality related to executing a Hard Reset on a port.
pub trait HardReset: Pd {
    /// Execute a Hard Reset on the given port.
    fn hard_reset(&mut self, port: LocalPortId) -> impl Future<Output = Result<(), PdError>>;
}
