use embedded_usb_pd::{LocalPortId, PdError};

use crate::{control::svid::DiscoveredSvids, controller::pd::Pd};

/// Functionality related to getting a port's discovered SVIDs.
pub trait SvidDiscovery: Pd {
    /// Get a port's discovered SVIDs
    fn get_discovered_svids(&mut self, port: LocalPortId) -> impl Future<Output = Result<DiscoveredSvids, PdError>>;
}
