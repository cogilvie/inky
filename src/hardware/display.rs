use crate::{
    eeprom::{EEPROM},
    core::colors::Color,
};

use rppal::{
    gpio::{Gpio, InputPin, OutputPin},
    spi::{Bus, Mode, SlaveSelect as SecondarySelect, Spi},
};

use anyhow::Result;
use std::time::Duration;

pub struct SpiPacket {
    pub command: u8,
    pub data: Option<Vec<u8>>,
}

impl SpiPacket {
    pub fn with_data(command: u8, data: Vec<u8>) -> Self {
        Self { command, data: Some(data) }
    }
    pub fn no_data(command: u8) -> Self {
        Self { command, data: None }
    }
}

pub struct InkyConnection {
    pub spi: Spi,
    pub cs: OutputPin,
    pub dc: OutputPin,
    pub reset: OutputPin,
    pub busy: InputPin,
    pub eeprom: EEPROM,
}

impl InkyConnection {
    pub fn new(
        eeprom: EEPROM,
    ) -> Result<Self> {
        let gpio = Gpio::new()?;

        Ok(Self {
            spi: Spi::new(
                Bus::Spi0,
                SecondarySelect::Ss0,
                488_000,
                Mode::Mode0,
            )?,
            cs: gpio.get(8)?.into_output_high(),
            dc: gpio.get(22)?.into_output_low(),
            reset: gpio.get(27)?.into_output_high(),
            busy: gpio.get(17)?.into_input(),
            eeprom: eeprom,
        })
    }
}

pub trait InkyConnectionProvider {
    fn connection(&mut self) -> &InkyConnection;
}

pub trait InkyDisplay : InkyConnectionProvider {
    fn new(eeprom: EEPROM) -> Result<Self> where Self: Sized;
    fn reset(&mut self) -> Result<()>;
    fn convert(&self, buf: &Vec<Vec<Color>>) -> Result<Vec<u8>>;
    fn update(&mut self, buf: Vec<u8>) -> Result<()>;
    fn wait(&mut self, timeout: Option<Duration>) -> Result<()>;
    fn spi_send(&mut self, packet: SpiPacket) -> Result<()>;
}

macro_rules! add_inky_display_type {
    ( $type:ident )=> {
        pub struct $type {
            connection: InkyConnection,
        }

        impl InkyConnectionProvider for $type {
            fn connection(&mut self) -> &InkyConnection {
                &self.connection
            }
        }
    };
}

pub(crate) use add_inky_display_type;
