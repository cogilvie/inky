use crate::{
    core::colors::Color,
    eeprom::{DisplayVariant, EEPROM},
    hardware::display::{
        InkyConnection, InkyConnectionProvider, InkyDisplay, RefreshSpeed, SpiPacket,
    },
};

use rppal::gpio::Trigger;

use anyhow::{ensure, Context, Result};

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
    EL673_PARTIAL_WINDOW = 0x83,
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

pub struct InkyE673 {
    connection: InkyConnection,
    speed: RefreshSpeed,
}

impl InkyConnectionProvider for InkyE673 {
    fn connection(&mut self) -> &InkyConnection {
        &self.connection
    }
}

impl InkyE673 {
    /// Create an E673 driver using the requested refresh speed.
    pub fn new_with_speed(eeprom: EEPROM, speed: RefreshSpeed) -> Result<Self> {
        ensure!(
            matches!(eeprom.display_variant(), DisplayVariant::E673),
            "Only the Inky E673 is supported!"
        );

        let connection = InkyConnection::new(eeprom)?;
        connection.spi.set_clock_speed(10_000_000)?;

        Ok(Self { connection, speed })
    }

    fn write_data(&mut self, data: &[u8]) -> Result<()> {
        self.connection.cs.set_low();
        self.connection.dc.set_high();
        let result = (|| {
            for chunk in data.chunks(4096) {
                self.connection.spi.write(chunk)?;
            }
            Ok(())
        })();
        self.connection.cs.set_high();
        self.connection.dc.set_low();
        result
    }
}

impl InkyDisplay for InkyE673 {
    fn new(eeprom: EEPROM) -> Result<Self> {
        Self::new_with_speed(eeprom, RefreshSpeed::default())
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
            vec![match self.speed {
                RefreshSpeed::Normal => 0x08,
                RefreshSpeed::Medium => 0x04,
                RefreshSpeed::Fast => 0x07,
            }],
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

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_PSR as u8,
            vec![0x4F, 0x6E],
        ))?;
        self.wait(Some(Duration::from_millis(300)))?;

        Ok(())
    }

    fn update_region(&mut self, x: usize, y: usize, pixels: Vec<Vec<Color>>) -> Result<()> {
        ensure!(!pixels.is_empty(), "Partial update height must be non-zero");
        let width = pixels[0].len();
        ensure!(width > 0, "Partial update width must be non-zero");
        ensure!(
            pixels.iter().all(|row| row.len() == width),
            "Partial update rows must have equal widths"
        );

        let height = pixels.len();
        let x_end = x
            .checked_add(width)
            .context("Partial update x-coordinate overflow")?;
        let y_end = y
            .checked_add(height)
            .context("Partial update y-coordinate overflow")?;
        ensure!(
            x_end <= self.connection.eeprom.width() as usize
                && y_end <= self.connection.eeprom.height() as usize,
            "Partial update region is outside the display bounds"
        );

        let x_last = x_end - 1;
        let y_last = y_end - 1;
        let window = vec![
            ((x >> 8) & 0x03) as u8,
            (x & 0xFF) as u8,
            ((x_last >> 8) & 0x03) as u8,
            (x_last & 0xFF) as u8,
            ((y >> 8) & 0x03) as u8,
            (y & 0xFF) as u8,
            ((y_last >> 8) & 0x03) as u8,
            (y_last & 0xFF) as u8,
            0x01,
        ];

        self.reset()?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_PARTIAL_WINDOW as u8,
            window,
        ))?;
        self.wait(Some(Duration::from_millis(300)))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EL673_DTM1 as u8,
            Vec::new(),
        ))?;
        for row in &pixels {
            self.write_data(&encode_partial_row(row))?;
        }
        self.wait(Some(Duration::from_millis(300)))?;

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

fn encode_partial_row(row: &[Color]) -> Vec<u8> {
    row.chunks(2)
        .map(|pair| {
            let first = as_u8(&pair[0]);
            let second = pair.get(1).map(as_u8).unwrap_or(0);
            (first << 4) | second
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::encode_partial_row;
    use crate::core::colors::Color;

    #[test]
    fn encodes_even_partial_rows_as_color_nibbles() {
        let row = [Color::Black, Color::White, Color::Red, Color::Green];
        assert_eq!(encode_partial_row(&row), [0x01, 0x36]);
    }

    #[test]
    fn pads_the_low_nibble_of_odd_partial_rows() {
        let row = [Color::Red, Color::Yellow, Color::Blue];
        assert_eq!(encode_partial_row(&row), [0x32, 0x50]);
    }
}
