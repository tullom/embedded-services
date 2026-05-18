use embedded_usb_pd::PdError;

use crate::{control::svid::DiscoveredSvids, port::pd::Pd};

/// Functionality related to getting this port's discovered SVIDs.
pub trait SvidDiscovery: Pd {
    /// Get this port's discovered SVIDs
    fn get_discovered_svids(&mut self) -> impl Future<Output = Result<DiscoveredSvids, PdError>>;
}
