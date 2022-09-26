use embedded_hal::{
    blocking::i2c::{Write, WriteRead},
    digital::v2::StatefulOutputPin,
};
use esp8266_hal::{
    ram,
    time::{Nanoseconds, U32Ext},
};

use crate::{nanosecond_delay::ProvideNanosecondDelay, I2CSpeed};

type Result<T> = core::result::Result<T, crate::errors::Error>;

/// Represents a two-wire i2c controller.
pub struct I2C<SDA, SCL, DP>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    /// The pin referencing the sda line.
    sda_pin: SDA,
    /// The pin referencing the scl line.
    scl_pin: SCL,
    /// The speed at which to drive the clock signals.
    speed: I2CSpeed,
    /// provider for nanoseconds delay
    delay_provider: DP,
}

impl<SDA, SCL, DP> I2C<SDA, SCL, DP>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
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
    ///     MyNanodecondDelayProvider{...}
    /// );
    /// ```
    pub fn new(sda: SDA, scl: SCL, delay_provider: DP) -> Self {
        let mut res = Self {
            sda_pin: sda,
            scl_pin: scl,
            speed: I2CSpeed::Normal100kHz,
            delay_provider,
        };

        res.end_transmission();

        res
    }

    /// This method begins a new i2c transmission by sending
    /// the start condition signal and then transmitting
    /// the device select packet.
    ///
    /// If the write_mode parameter is true, the R/W bit will
    /// be 0, signalling to the downstream devices that
    /// a write operation will follow.
    #[ram]
    pub fn begin_transmission(&mut self, address: u8, write_mode: bool) -> Result<()> {
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
            return Err(crate::errors::Error::NoAck);
        }
    }

    /// This method terminates an existing i2c transmission by
    /// sending the stop condition signal.
    #[ram]
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
    /// let mut wire = I2C::begin(...);
    /// wire.begin_transmission(0x50, true)
    /// let result1 = wire.write(&[0, 0]);
    /// let result2 = wire.write(b"hello");
    /// wire.end_transmission();
    /// ```
    #[ram]
    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
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
                return Err(crate::errors::Error::NoAck);
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
    /// let mut wire = I2C::new(...);
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
    #[ram]
    pub fn read(&mut self, ack: bool) -> Result<u8> {
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

#[ram]
fn clock_high<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    let _ = i2c.scl_pin.set_high();
    i2c.delay_provider.delay_ns(500.ns());
}

#[ram]
fn clock_low<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    let _ = i2c.scl_pin.set_low();
    i2c.delay_provider.delay_ns(500.ns());
}

#[ram]
fn data_high<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    let _ = i2c.sda_pin.set_high();
    i2c.delay_provider.delay_ns(500.ns());
}

#[ram]
fn data_low<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    let _ = i2c.sda_pin.set_low();
    i2c.delay_provider.delay_ns(500.ns());
}

#[ram]
fn data_release<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    let _ = i2c.sda_pin.set_high();
    i2c.delay_provider.delay_ns(500.ns());
}

#[ram]
fn clock_release<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    let _ = i2c.scl_pin.set_high();
    i2c.delay_provider.delay_ns(500.ns());
}

#[ram]
fn i2c_start_condition<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    data_low(i2c);
    clock_low(i2c);
}

#[ram]
fn i2c_read_bit<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>) -> bool
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    clock_low(i2c);
    data_release(i2c);

    // **************
    // Pulse the clock
    // **************
    clock_release(i2c);

    let nanos = i2c.delay_provider.nanos();
    let timeout = wraping_add_nanos(nanos, Nanoseconds(i2c.speed as u32 * 4));
    let stretch_timeout = wraping_add_nanos(nanos, Nanoseconds(i2c.speed as u32 * 16));
    let mut res = true;

    loop {
        // Check for stretch condition
        let now = i2c.delay_provider.nanos();

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

        i2c.delay_provider.delay_ns(500.ns());
    }

    // Bring clock back down
    clock_low(i2c);
    data_low(i2c);

    res
}

#[ram]
fn i2c_write_bit<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>, high: bool)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    if high {
        data_high(i2c);
    } else {
        data_low(i2c);
    }

    // Wait
    i2c.delay_provider.delay_ns(Nanoseconds(i2c.speed as u32));

    // **************
    // Pulse the clock
    // **************
    clock_release(i2c);
    i2c.delay_provider
        .delay_ns(Nanoseconds(i2c.speed as u32 * 2));

    // Pull clock low
    clock_low(i2c);
    i2c.delay_provider.delay_ns(Nanoseconds(i2c.speed as u32));
}

#[ram]
fn i2c_end_condition<SDA, SCL, DP>(i2c: &mut I2C<SDA, SCL, DP>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    clock_release(i2c);
    i2c.delay_provider.delay_ns(500.ns());
    data_release(i2c);
    i2c.delay_provider.delay_ns(500.ns());
}

#[ram]
fn wraping_add_nanos(a: Nanoseconds, b: Nanoseconds) -> Nanoseconds {
    Nanoseconds(a.0.wrapping_add(b.0))
}

//-----------------------------------------------------------------------------

impl<SDA, SCL, DP> Write for I2C<SDA, SCL, DP>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    type Error = crate::errors::Error;

    #[ram]
    fn write(&mut self, address: u8, bytes: &[u8]) -> core::result::Result<(), Self::Error> {
        Self::begin_transmission(self, address, true)?;
        Self::write(self, bytes)?;
        Self::end_transmission(self);

        Ok(())
    }
}

impl<SDA, SCL, DP> WriteRead for I2C<SDA, SCL, DP>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    type Error = crate::errors::Error;

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> core::result::Result<(), Self::Error> {
        let mut res = Ok(());

        Self::begin_transmission(self, address, true)?;
        Self::write(self, bytes)?;
        for place in buffer.iter_mut() {
            match Self::read(self, true) {
                Ok(v) => *place = v,
                Err(e) => {
                    res = Err(e);
                    break;
                }
            }
        }
        Self::end_transmission(self);

        res
    }
}
