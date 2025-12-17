use crate::{
    core::colors::Color,
    eeprom::{DisplayVariant, EEPROM},
    hardware::display::{
        add_inky_display_type, InkyConnection, InkyConnectionProvider, InkyDisplay, SpiPacket,
    },
    lut::LUT_BLACK,
};

use rppal::gpio::Trigger;

use anyhow::{ensure, Result};

use std::{thread::sleep, time::Duration};

#[repr(u8)]
enum DisplayCommands {
    DataEntryMode = 0x11, // X/Y increment
    DisplayUpdateSequence = 0x22,
    DummyLinePeriod = 0x3a,
    EnterDeepSleep = 0x10,
    GSTransition = 0x3c,
    GateDrivingVoltage = 0x3,
    GateLineWidth = 0x3b,
    GateSetting = 0x1,
    SetAnalogBlockControl = 0x74,
    SetDigitalBlockControl = 0x7e,
    SetLUT = 0x32,
    SetRamXPointerStart = 0x4e,
    SetRamXStartEnd = 0x44,
    SetRamYPointerStart = 0x4f,
    SetRamYStartEnd = 0x45,
    SoftReset = 0x12,
    SourceDrivingVoltage = 0x4,
    TriggerDisplayUpdate = 0x20,
    VComRegister = 0x2c,
    SetBWBuffer = 0x24,
    SetRYBuffer = 0x26,
}

fn as_u8(color: &Color) -> u8 {
    if !matches!(color, Color::Black) {
        1
    } else {
        0
    }
}

add_inky_display_type!(InkyWhat);

impl InkyDisplay for InkyWhat {
    fn new(eeprom: EEPROM) -> Result<Self> {
        ensure!(
            matches!(eeprom.display_variant(), DisplayVariant::What),
            "Only the Inky What is supported!"
        );

        Ok(Self {
            connection: InkyConnection::new(eeprom)?,
        })
    }

    fn reset(&mut self) -> Result<()> {
        self.connection.reset.set_low();
        // Sleep time from inky library
        sleep(Duration::from_millis(100));
        self.connection.reset.set_high();
        sleep(Duration::from_millis(100));
        self.spi_send(SpiPacket::no_data(DisplayCommands::SoftReset as u8))?;
        self.wait(None)?;
        Ok(())
    }

    fn update(&mut self, buf: Vec<u8>) -> Result<()> {
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::SetAnalogBlockControl as u8,
            vec![0x54],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::SetDigitalBlockControl as u8,
            vec![0x3b],
        ))?;

        let mut gate_setting_data = (self.connection.eeprom.height() as u16).to_le_bytes().to_vec();
        gate_setting_data.push(0x00);

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::GateSetting as u8,
            gate_setting_data,
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::GateDrivingVoltage as u8,
            vec![0x17],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::SourceDrivingVoltage as u8,
            vec![0x41, 0xAC, 0x32],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::DummyLinePeriod as u8,
            vec![0x07],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::GateLineWidth as u8,
            vec![0x04],
        ))?;
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::DataEntryMode as u8,
            vec![0x03],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::VComRegister as u8,
            vec![0x3c],
        ))?;

        // TODO: Make this depend on color:
        // if self.border_colour == self.BLACK:
        //     self._send_command(0x3c, 0b00000000)  # GS Transition Define A + VSS + LUT0
        // elif self.border_colour == self.RED and self.colour == 'red':
        //     self._send_command(0x3c, 0b01110011)  # Fix Level Define A + VSH2 + LUT3
        // elif self.border_colour == self.YELLOW and self.colour == 'yellow':
        //     self._send_command(0x3c, 0b00110011)  # GS Transition Define A + VSH2 + LUT3
        // elif self.border_colour == self.WHITE:
        //     self._send_command(0x3c, 0b00110001)  # GS Transition Define A + VSH2 + LUT1
        self.spi_send(SpiPacket::with_data(
            DisplayCommands::GSTransition as u8,
            vec![0b00110001],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::SetLUT as u8,
            LUT_BLACK.to_vec(),
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::SetRamXStartEnd as u8,
            vec![0x00, ((self.connection.eeprom.width() / 8) - 1) as u8],
        ))?;

        let mut data = vec![0x00, 0x00];
        data.extend_from_slice(&(self.connection.eeprom.height() as u16).to_le_bytes());

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::SetRamYStartEnd as u8,
            data,
        ))?;

        // 0 because nothing == RED
        // let ry_buf = vec![0; bw_buf.len()];

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::SetRamXPointerStart as u8,
            vec![0x00],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::SetRamYPointerStart as u8,
            vec![0x00, 0x00],
        ))?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::SetBWBuffer as u8,
            buf,
        ))?;

        // TODO: Support additional displays
        // self.spi_send(
        //     SpiPacketBuilder::default()
        //         .command(DisplayCommands::SetRamXPointerStart)
        //         .data(vec![0x00])
        //         .build()?,
        // )?;

        // self.spi_send(
        //     SpiPacketBuilder::default()
        //         .command(DisplayCommands::SetRamYPointerStart)
        //         .data(vec![0x00, 0x00])
        //         .build()?,
        // )?;

        // self.spi_send(
        //     SpiPacketBuilder::default()
        //         .command(DisplayCommands::SetRYBuffer)
        //         .data(ry_buf)
        //         .build()?,
        // )?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::DisplayUpdateSequence as u8,
            vec![0xc7],
        ))?;

        self.spi_send(SpiPacket::no_data(
            DisplayCommands::TriggerDisplayUpdate as u8,
        ))?;

        // Defined by inky
        sleep(Duration::from_secs_f32(0.05));

        self.wait(None)?;

        self.spi_send(SpiPacket::with_data(
            DisplayCommands::EnterDeepSleep as u8,
            vec![0x01],
        ))?;

        Ok(())
    }

    fn wait(&mut self, timeout: Option<Duration>) -> Result<()> {
        self.connection.busy.set_interrupt(Trigger::FallingEdge)?;
        self.connection.busy.poll_interrupt(false, timeout)?;
        self.connection.busy.clear_interrupt()?;
        Ok(())
    }

    fn spi_send(&mut self, packet: SpiPacket) -> Result<()> {
        self.connection.dc.set_low();
        self.connection.spi.write(&[packet.command])?;

        if let Some(data) = packet.data {
            self.connection.dc.set_high();
            for chunk in data.chunks(4096) {
                self.connection.spi.write(chunk)?;
            }
        }

        Ok(())
    }

    fn convert(&self, buf: &Vec<Vec<Color>>) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut bit_pos: u8 = 0;
        let mut cur_byte: u8 = 0;
        for row in buf {
            for b in row {
                cur_byte |= (as_u8(b)) << bit_pos;
                bit_pos += 1;
                if bit_pos == 8 {
                    result.push(cur_byte);
                    cur_byte = 0;
                    bit_pos = 0;
                }
            }
        }
        if bit_pos != 0 {
            result.push(cur_byte);
        }
        Ok(result)
    }
}
