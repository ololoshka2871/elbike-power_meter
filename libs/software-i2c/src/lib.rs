#![no_std]

//! This module provides the ability to designate any two
//! gpio pins as SDA/SCL which allows you to introduce i2c
//! capabilities into your project.
//!
//! In order to use these pins, you must include a pull-up
//! resistor on both lines.
#![allow(dead_code)]
#![allow(unused_variables)]

use embedded_hal::blocking::i2c::Write;
use embedded_hal::digital::v2::StatefulOutputPin;
use esp8266_hal::time::{Hertz, Nanoseconds, U32Ext};
use xtensa_lx::timer::{delay, get_cycle_count};

#[derive(Clone, Copy)]
pub enum I2CSpeed {
    Fast400kHz = 1250,
    Normal100kHz = 2500,
}

/// Represents a two-wire i2c controller.
pub struct I2C<SDA, SCL>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    /// The pin referencing the sda line.
    sda_pin: SDA,
    /// The pin referencing the scl line.
    scl_pin: SCL,
    /// The speed at which to drive the clock signals.
    speed: I2CSpeed,

    cpu_freq: Hertz,
}

impl<SDA, SCL> I2C<SDA, SCL>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    /// This method creates a new instance of an i2c controller.
    /// After specifying the pins on which sda and scl lines reside,
    /// the system will configure those pins as open-drain.
    ///
    /// This means you must have a pull-up resistor for each
    /// line on your circuit.
    ///
    /// ```
    /// let mut wire = I2C::Begin(        
    ///     pins.gpio2.into_open_drain_output(),
    ///     pins.gpio4.into_open_drain_output(),
    /// );
    /// ```
    pub fn new(sda: SDA, scl: SCL) -> Self {
        Self {
            sda_pin: sda,
            scl_pin: scl,
            speed: I2CSpeed::Normal100kHz,

            cpu_freq: 80_000_000.hz(),
        }
    }

    /// This method begins a new i2c transmission by sending
    /// the start condition signal and then transmitting
    /// the device select packet.
    ///
    /// If the write_mode parameter is true, the R/W bit will
    /// be 0, signalling to the downstream devices that
    /// a write operation will follow.
    pub fn begin_transmission(&mut self, address: u8, write_mode: bool) -> Result<(), ()> {
        // Start transmission
        i2c_start_condition(self);

        // Address frame
        let mut mask = 0x1 << 6;
        for _ in 0..=6 {
            let high = address & mask;
            i2c_write_bit(self, high > 0);
            mask >>= 1;
        }

        // R/W bit
        if write_mode {
            i2c_write_bit(self, false);
        } else {
            i2c_write_bit(self, true);
        }

        // Ack bit
        let ack = i2c_read_bit(self);
        if ack == false {
            // Success
            return Ok(());
        } else {
            // Transmissino not acknowledged. Terminate.
            i2c_end_condition(self);
            return Err(());
        }
    }

    /// This method terminates an existing i2c transmission by
    /// sending the stop condition signal.
    pub fn end_transmission(&mut self) {
        i2c_end_condition(self);
    }

    /// This method will write a series of bytes to
    /// the i2c bus. After each byte, the controller
    /// will expect an acknowledgement.
    ///
    /// In order to use this method successfully,
    /// you must first have invoked `i2c.begin_transmission()`
    ///
    /// ```
    /// let mut wire = I2C::begin(19, 18);
    /// wire.begin_transmission(0x50, true)
    /// let result1 = wire.write(&[0, 0]);
    /// let result2 = wire.write(b"hello");
    /// wire.end_transmission();
    /// ```
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        for byte in bytes {
            let mut mask = 0x1 << 7;
            for _ in 0..=7 {
                let high = byte & mask;
                i2c_write_bit(self, high > 0);
                mask >>= 1;
            }
            let ack = i2c_read_bit(self);
            if ack == false {
                // Success
            } else {
                // Not acknowledged
                i2c_end_condition(self);
                return Err(());
            }
        }
        return Ok(());
    }

    /// This method will read a single byte
    /// from the downstream device.
    ///
    /// If the ack parameter is true, after reading
    /// from the downstream device, the teensy will
    /// send an acknowledgement bit.
    ///
    /// In order to use this method successfully,
    /// you must first have invoked `i2c.begin_transmission()`
    ///
    /// ```
    /// let mut wire = I2C::begin(19, 18);
    /// wire.begin_transmission(0x50, true)
    /// let str = &[
    ///     wire.read(true).unwrap_or_default(),
    ///     wire.read(true).unwrap_or_default(),
    ///     wire.read(true).unwrap_or_default(),
    ///     wire.read(true).unwrap_or_default(),
    ///     wire.read(true).unwrap_or_default(),
    /// ];
    /// wire.end_transmission();
    /// ```
    pub fn read(&mut self, ack: bool) -> Result<u8, ()> {
        let mut byte: u8 = 0;
        let mut mask = 0x1 << 7;

        for _ in 0..8 {
            if i2c_read_bit(self) {
                byte |= mask;
            }
            mask >>= 1;
        }

        if ack {
            // Send the ack bit
            i2c_write_bit(self, false);
        }

        return Ok(byte);
    }

    /// This method will change the signal speed.
    /// By default, all signals are clocked at 100kHz
    /// but if you upgrade to fast mode, it'll be 400kHz.
    ///
    /// ```
    /// let mut wire = I2C::new(...);
    /// wire.set_speed(I2CSpeed::Fast400kHz);
    /// ```
    pub fn set_speed(mut self, speed: I2CSpeed) -> Self {
        self.speed = speed;
        self
    }
}

fn clock_high<SDA, SCL>(i2c: &mut I2C<SDA, SCL>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    let _ = i2c.scl_pin.set_high();
    wait_exact(500.ns(), i2c.cpu_freq);
}

fn clock_low<SDA, SCL>(i2c: &mut I2C<SDA, SCL>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    let _ = i2c.scl_pin.set_low();
    wait_exact(500.ns(), i2c.cpu_freq);
}

fn data_high<SDA, SCL>(i2c: &mut I2C<SDA, SCL>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    let _ = i2c.sda_pin.set_high();
    wait_exact(500.ns(), i2c.cpu_freq);
}

fn data_low<SDA, SCL>(i2c: &mut I2C<SDA, SCL>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    let _ = i2c.sda_pin.set_low();
    wait_exact(500.ns(), i2c.cpu_freq);
}

fn data_release<SDA, SCL>(i2c: &mut I2C<SDA, SCL>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    let _ = i2c.sda_pin.set_high();
    wait_exact(500.ns(), i2c.cpu_freq);
}

fn clock_release<SDA, SCL>(i2c: &mut I2C<SDA, SCL>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    let _ = i2c.scl_pin.set_high();
    wait_exact(500.ns(), i2c.cpu_freq);
}

fn i2c_start_condition<SDA, SCL>(i2c: &mut I2C<SDA, SCL>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    data_low(i2c);
    clock_low(i2c);
}

fn i2c_read_bit<SDA, SCL>(i2c: &mut I2C<SDA, SCL>) -> bool
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    clock_low(i2c);
    data_release(i2c);

    // **************
    // Pulse the clock
    // **************
    clock_release(i2c);
    let timeout = wraping_add_nanos(nanos(i2c.cpu_freq), Nanoseconds(i2c.speed as u32 * 4));
    let stretch_timeout =
        wraping_add_nanos(nanos(i2c.cpu_freq), Nanoseconds(i2c.speed as u32 * 16));
    let mut res = true;

    loop {
        // Check for stretch condition
        let now = nanos(i2c.cpu_freq);

        let clock_line = i2c.scl_pin.is_set_high().unwrap_or_default();
        let data_line = i2c.sda_pin.is_set_high().unwrap_or_default();

        if clock_line == false && now < stretch_timeout {
            // We are stretching the signal
            continue;
        } else if data_line == false {
            res = false;
        }

        if now > timeout {
            break;
        }

        wait_exact(500.ns(), i2c.cpu_freq);
    }

    // Bring clock back down
    clock_low(i2c);
    data_low(i2c);

    return res;
}

fn i2c_write_bit<SDA, SCL>(i2c: &mut I2C<SDA, SCL>, high: bool)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    if high {
        data_high(i2c);
    } else {
        data_low(i2c);
    }

    // Wait
    wait_exact(Nanoseconds(i2c.speed as u32), i2c.cpu_freq);

    // **************
    // Pulse the clock
    // **************
    clock_release(i2c);
    wait_exact(Nanoseconds(i2c.speed as u32 * 2), i2c.cpu_freq);

    // Pull clock low
    clock_low(i2c);
    wait_exact(Nanoseconds(i2c.speed as u32), i2c.cpu_freq);
}

fn i2c_end_condition<SDA, SCL>(i2c: &mut I2C<SDA, SCL>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    clock_release(i2c);
    wait_exact(500.ns(), i2c.cpu_freq);
    data_release(i2c);
    wait_exact(500.ns(), i2c.cpu_freq);
}

fn wait_exact<T: Into<Nanoseconds>, U: Into<Hertz>>(time: T, cpu_freq: U) {
    delay(((time.into().0 as u64 * cpu_freq.into().0 as u64) / 1_000_000_000) as u32);
}

fn nanos<U: Into<Hertz>>(cpu_freq: U) -> Nanoseconds {
    Nanoseconds((get_cycle_count() as u64 * 1_000_000_000 / cpu_freq.into().0 as u64) as u32)
}

fn wraping_add_nanos(a: Nanoseconds, b: Nanoseconds) -> Nanoseconds {
    Nanoseconds(a.0.wrapping_add(b.0))
}

impl<SDA, SCL> Write for I2C<SDA, SCL>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
{
    type Error = ();

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        Self::begin_transmission(self, address, true)?;
        Self::write(self, bytes)?;
        Self::end_transmission(self);

        Ok(())
    }
}
