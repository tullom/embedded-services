use embassy_time::Instant;

/// State shared between the port and event receiver
#[derive(Copy, Clone)]
pub struct SharedState {
    /// Sink ready timeout
    pub(crate) sink_ready_timeout: Option<Instant>,
}

impl SharedState {
    /// Create a new instance with default values
    pub fn new() -> Self {
        Self {
            sink_ready_timeout: None,
        }
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}
