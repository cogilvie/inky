use crate::{
    core::colors::Color,
    eeprom::{DisplayVariant, EEPROM},
    hardware::display::{
        add_inky_display_type, InkyConnection, InkyConnectionProvider, InkyDisplay, SpiPacket,
    },
};

use rppal::gpio::Trigger;

use anyhow::{ensure, Result};

use std::{thread::sleep, time::Duration};

#[repr(u8)]
enum DisplayCommands {
    EL673_PSR = 0x00,
    EL673_PWR = 0x01,
    EL673_POF = 0x02,
    EL673_POFS = 0x03,
    EL673_PON = 0x04,
    EL673_BTST1 = 0x05,
    EL673_BTST2 = 0x06,
    EL673_DSLP = 0x07,
    EL673_BTST3 = 0x08,
    EL673_DTM1 = 0x10,
    EL673_DSP = 0x11,
    EL673_DRF = 0x12,
    EL673_PLL = 0x30,
    EL673_CDI = 0x50,
    EL673_TCON = 0x60,
    EL673_TRES = 0x61,
    EL673_REV = 0x70,
    EL673_VDCS = 0x82,
    EL673_PWS = 0xE3,
}

fn as_u8(color: &Color) -> u8 {
    match color {
        Color::Black => 0,
        Color::White => 1,
        Color::Yellow => 2,
        Color::Red => 3,
        Color::Blue => 5,
        Color::Green => 6,
    }
}

add_inky_display_type!(InkyE673);

impl InkyDisplay for InkyE673 {
    fn new(eeprom: EEPROM) -> Result<Self> {
        ensure!(
            matches!(eeprom.display_variant(), DisplayVariant::E673),
            "Only the Inky E673 is supported!"
        );

        Ok(Self {
            connection: InkyConnection::new(eeprom)?,
        })
    }

    fn reset(&mut self) -> Result<()> {
        self.connection.reset.set_low();
        // Sleep time from inky library
        sleep(Duration::from_millis(30));
        self.connection.reset.set_high();
        sleep(Duration::from_millis(30));

        self.wait(Some(Duration::from_millis(300)))?;

        self.spi_send(SpiPacket::with_data(
            0xAA,
            vec![0x49, 0x55, 0x20, 0x08, 0x09, 0x18],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_PWR as u8,
            vec![0x3F],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_PSR as u8,
            vec![0x5F, 0x69],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_BTST1 as u8,
            vec![0x40, 0x1F, 0x1F, 0x2C],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_BTST3 as u8,
            vec![0x6F, 0x1F, 0x1F, 0x22],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_BTST2 as u8,
            vec![0x6F, 0x1F, 0x17, 0x17],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_POFS as u8,
            vec![0x00, 0x54, 0x00, 0x44],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_TCON as u8,
            vec![0x02, 0x00],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_PLL as u8,
            vec![0x08],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_CDI as u8,
            vec![0x3F],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_TRES as u8,
            vec![0x03, 0x20, 0x01, 0xE0],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_PWS as u8,
            vec![0x2F],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_VDCS as u8,
            vec![0x01],
        ))?;

        Ok(())
    }

    fn update(&mut self, buf: Vec<u8>) -> Result<()> {
        self.reset()?;

        self.spi_send(SpiPacket::with_data(DisplayCommands::EL673_DTM1 as u8, buf))?;
        self.spi_send(SpiPacket::no_data(DisplayCommands::EL673_PON as u8))?;
        self.wait(Some(Duration::from_millis(300)))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_BTST2 as u8,
            vec![0x6F, 0x1F, 0x17, 0x49],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_DRF as u8,
            vec![0x00],
        ))?;
        self.wait(Some(Duration::from_millis(32000)))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_POF as u8,
            vec![0x00],
        ))?;
        self.wait(Some(Duration::from_millis(300)))?;

        Ok(())
    }

    fn wait(&mut self, timeout: Option<Duration>) -> Result<()> {
        // If the busy_pin is *high* (pulled up by host)
        // then assume we're not getting a signal from inky
        // and wait the timeout period to be safe.
        if self.connection.busy.is_high() {
            sleep(timeout.unwrap_or(Duration::from_millis(100)));
            return Ok(());
        }

        self.connection.busy.set_interrupt(Trigger::RisingEdge)?;
        self.connection.busy.poll_interrupt(false, timeout)?;
        self.connection.busy.clear_interrupt()?;
        Ok(())
    }

    fn spi_send(&mut self, packet: SpiPacket) -> Result<()> {
        self.connection.cs.set_low();
        self.connection.dc.set_low();
        sleep(Duration::from_millis(300));
        self.connection.spi.write(&[packet.command])?;

        if let Some(data) = packet.data {
            self.connection.dc.set_high();
            for chunk in data.chunks(4096) {
                self.connection.spi.write(chunk)?;
            }
        }

        self.connection.cs.set_high();
        self.connection.dc.set_low();

        Ok(())
    }

    fn convert(&self, buf: &Vec<Vec<Color>>) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        for row in buf {
            ensure!(row.len() % 2 == 0, "Row length must be even!");
            // Take pairs of pixel values and packs them into single bytes
            for pair in row.chunks(2) {
                let pixel1 = as_u8(&pair[0]);
                let pixel2 = as_u8(&pair[1]);
                result.push(((pixel1 << 4) & 0xF0) | (pixel2 & 0x0F));
            }
        }
        Ok(result)
    }
}
