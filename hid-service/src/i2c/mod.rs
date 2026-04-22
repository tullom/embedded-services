mod device;
mod host;
pub mod passthrough;

pub use device::{Config as DeviceConfig, Device};
pub use host::{Access, Config as HostConfig, Host};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    Probe,
    Write,
    Read,
}

// TODO: remove and use embedded hal trait when imxrt I2C implements it
#[allow(async_fn_in_trait)]
pub trait I2cSlaveAsync {
    type Error;

    async fn listen(&mut self) -> Result<Command, Self::Error>;
    async fn respond_to_write(&mut self, buf: &mut [u8]) -> Result<(), Self::Error>;
    async fn respond_to_read(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
}
