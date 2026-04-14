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
    DRIVER_CONTROL = 0x01,
    GATE_VOLTAGE = 0x03,
    SOURCE_VOLTAGE = 0x04,
    DISPLAY_CONTROL = 0x07,
    NON_OVERLAP = 0x0B,
    BOOSTER_SOFT_START = 0x0C,
    GATE_SCAN_START = 0x0F,
    DEEP_SLEEP = 0x10,
    DATA_MODE = 0x11,
    SW_RESET = 0x12,
    TEMP_WRITE = 0x1A,
    TEMP_READ = 0x1B,
    TEMP_CONTROL = 0x1C,
    TEMP_LOAD = 0x1D,
    MASTER_ACTIVATE = 0x20,
    DISP_CTRL1 = 0x21,
    DISP_CTRL2 = 0x22,
    WRITE_RAM = 0x24,
    WRITE_ALTRAM = 0x26,
    READ_RAM = 0x25,
    VCOM_SENSE = 0x28,
    VCOM_DURATION = 0x29,
    WRITE_VCOM = 0x2C,
    READ_OTP = 0x2D,
    WRITE_LUT = 0x32,
    WRITE_DUMMY = 0x3A,
    WRITE_GATELINE = 0x3B,
    WRITE_BORDER = 0x3C,
    SET_RAMXPOS = 0x44,
    SET_RAMYPOS = 0x45,
    SET_RAMXCOUNT = 0x4E,
    SET_RAMYCOUNT = 0x4F,
    NOP = 0xFF,
}

fn as_u8(color: &Color) -> u8 {
    match color {
        Color::Black => 0,
        Color::White => 1,
         _ => todo!(),
    }
}

fn rotate<T>(v: &Vec<Vec<T>>) -> Vec<Vec<T>>
where
    T: Clone,
{
    assert!(!v.is_empty());
    (0..v[0].len())
        .map(|i| v.iter().rev().map(|inner| inner[i].clone()).collect::<Vec<T>>())
        .collect()
}

const lut_data : &[u8] = &[
          0x02, 0x02, 0x01, 0x11, 0x12, 0x12, 0x22, 0x22, 0x66, 0x69,
          0x69, 0x59, 0x58, 0x99, 0x99, 0x88, 0x00, 0x00, 0x00, 0x00,
          0xF8, 0xB4, 0x13, 0x51, 0x35, 0x51, 0x51, 0x19, 0x01, 0x00
];


const rows : u8 = 250;
const cols : u8 = 136;

add_inky_display_type!(InkyPhatSsd1608);

impl InkyDisplay for InkyPhatSsd1608 {
    fn new(eeprom: EEPROM) -> Result<Self> {
        ensure!(
            matches!(eeprom.display_variant(), DisplayVariant::PhatSsd1608),
            "Only the Inky Phat SSD1608 is supported!"
        );

        Ok(Self {
            connection: InkyConnection::new(eeprom)?,
        })
    }

    fn reset(&mut self) -> Result<()> {
        self.connection.reset.set_low();
        // Sleep time from inky library
        sleep(Duration::from_millis(500));
        self.connection.reset.set_high();
        sleep(Duration::from_millis(500));
        self.spi_send(SpiPacket::no_data(DisplayCommands::SW_RESET as u8))?;
        sleep(Duration::from_millis(1000));
        self.wait(None)?;
        Ok(())
    }

    fn update(&mut self, buf: Vec<u8>) -> Result<()> {
        self.reset()?;

        self.spi_send(SpiPacket::with_data(DisplayCommands::DRIVER_CONTROL as u8, vec![rows - 1, 0x00, 0x00]))?;
        //Set dummy line period
        self.spi_send(SpiPacket::with_data(DisplayCommands::WRITE_DUMMY as u8, vec![0x1B]))?;
        //Set Line Width
        self.spi_send(SpiPacket::with_data(DisplayCommands::WRITE_GATELINE as u8, vec![0x0B]))?;


        // Data entry sequence (scan direction leftward and downward)
        self.spi_send(SpiPacket::with_data(DisplayCommands::DATA_MODE as u8, vec![0x03]))?;
        // Set ram X start and end position
        let xpos = vec![0x00, (cols / 8 - 1) as u8];
        self.spi_send(SpiPacket::with_data(DisplayCommands::SET_RAMXPOS as u8, xpos))?;
        // Set ram Y start and end position
        let ypos = vec![0x00, 0x00, ((rows - 1) & 0xFF) as u8, 0x00];
        self.spi_send(SpiPacket::with_data(DisplayCommands::SET_RAMYPOS as u8, ypos))?;
        // VCOM Voltage
        self.spi_send(SpiPacket::with_data(DisplayCommands::WRITE_VCOM as u8, vec![0x70]))?;
        // Write LUT DATA
        self.spi_send(SpiPacket::with_data(DisplayCommands::WRITE_LUT as u8, lut_data.to_vec()))?;

        // Border colour
        self.spi_send(SpiPacket::with_data(DisplayCommands::WRITE_BORDER as u8, vec![0x00]))?;
        //For now we just set a default white border
        // if self.border_colour == self.BLACK:
        //     self._send_command(ssd1608.WRITE_BORDER, 0b00000000)
        //     // GS Transition + Waveform 00 + GSA 0 + GSB 0
        // elif self.border_colour == self.RED and self.colour == "red":
        //     self._send_command(ssd1608.WRITE_BORDER, 0b00000110)
        //     // GS Transition + Waveform 01 + GSA 1 + GSB 0
        // elif self.border_colour == self.YELLOW and self.colour == "yellow":
        //     self._send_command(ssd1608.WRITE_BORDER, 0b00001111)
        //     // GS Transition + Waveform 11 + GSA 1 + GSB 1
        // elif self.border_colour == self.WHITE:
        //     self._send_command(ssd1608.WRITE_BORDER, 0b00000001)
        //     // GS Transition + Waveform 00 + GSA 0 + GSB 1


        // Set RAM address to 0, 0
        self.spi_send(SpiPacket::with_data(DisplayCommands::SET_RAMXCOUNT as u8, vec![0x00]))?;
        self.spi_send(SpiPacket::with_data(DisplayCommands::SET_RAMYCOUNT as u8, vec![0x00, 0x00]))?;


        self.spi_send(SpiPacket::with_data(DisplayCommands::WRITE_RAM as u8, buf))?;
        // self.spi_send(SpiPacket::with_data(DisplayCommands::WRITE_ALTRAM as u8, buf))?;

        self.spi_send(SpiPacket::no_data(DisplayCommands::MASTER_ACTIVATE as u8))?;

        self.wait(None)?;

        self.spi_send(SpiPacket::no_data(DisplayCommands::MASTER_ACTIVATE as u8))?;

        Ok(())
    }

    fn wait(&mut self, timeout: Option<Duration>) -> Result<()> {
        if !self.connection.busy.is_high() {
            return Ok(());
        }

        self.connection.busy.set_interrupt(Trigger::FallingEdge)?;
        self.connection.busy.poll_interrupt(false, timeout)?;
        self.connection.busy.clear_interrupt()?;
        Ok(())
    }

    fn spi_send(&mut self, packet: SpiPacket) -> Result<()> {
        // First send the command
        self.connection.cs.set_low();
        self.connection.dc.set_low();

        self.connection.spi.write(&[packet.command])?;

        self.connection.cs.set_high();

        // Send the data if it exists
        if let Some(data) = packet.data {
            self.connection.cs.set_low();
            self.connection.dc.set_high();
            for chunk in data.chunks(4096) {
                self.connection.spi.write(chunk)?;
            }
            self.connection.cs.set_high();
        }

        Ok(())
    }

    fn convert(&self, buf: &Vec<Vec<Color>>) -> Result<Vec<u8>> {

        let rotated = rotate(buf);
        
        let mut result = Vec::new();
        let mut byte = 0u8;
        for row in rotated {
            result.push(0x00);
            let mut bit_count = 0;

            for pixel in row {
                byte = (byte << 1) | (as_u8(&pixel) & 0x01);
                bit_count += 1;
                if bit_count == 8 {
                    result.push(byte);
                    byte = 0;
                    bit_count = 0;
                }
            }
            if bit_count > 0 {
                byte <<= 8 - bit_count;
                result.push(byte);
            }
        }

        print!("Converted buffer to {} bytes\n", result.len());

        Ok(result)
    }
}
