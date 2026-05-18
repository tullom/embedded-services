use embedded_usb_pd::PdError;
use embedded_usb_pd::vdm::structured::command::discover_identity::{sop, sop_prime};

use crate::port::pd::Pd;

/// Functionality related to getting this port's Discover Identity responses.
pub trait DiscoverIdentity: Pd {
    /// Get the latest response from the Discover Identity command targeting SOP.
    fn get_discover_identity_sop_response(&mut self) -> impl Future<Output = Result<sop::ResponseVdos, PdError>>;

    /// Get the latest response from the Discover Identity command targeting SOP'.
    fn get_discover_identity_sop_prime_response(
        &mut self,
    ) -> impl Future<Output = Result<sop_prime::ResponseVdos, PdError>>;
}
