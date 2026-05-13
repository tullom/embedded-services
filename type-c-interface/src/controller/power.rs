use embedded_usb_pd::{LocalPortId, PdError};

/// System power state related controller functionality
pub trait SystemPowerStateStatus {
    /// Set the system power state on the given port.
    ///
    /// This notifies the PD controller of the current system power state,
    /// which triggers Application Configuration updates (e.g., crossbar reconfiguration).
    fn set_system_power_state_status(
        &mut self,
        port: LocalPortId,
        state: crate::control::power::SystemPowerState,
    ) -> impl Future<Output = Result<(), PdError>>;
}
