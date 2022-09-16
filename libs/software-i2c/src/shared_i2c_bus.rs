use core::cell::UnsafeCell;

use embedded_hal::{blocking::i2c::Write, digital::v2::StatefulOutputPin};

use super::{ProvideNanosecondDelay, I2C};

pub struct SharedI2CBus<SDA, SCL, DP>(UnsafeCell<I2C<SDA, SCL, DP>>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay;

pub struct SharedI2CBusAccessor<'a, SDA, SCL, DP>(&'a SharedI2CBus<SDA, SCL, DP>)
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay;

impl<SDA, SCL, DP> From<I2C<SDA, SCL, DP>> for SharedI2CBus<SDA, SCL, DP>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    fn from(bus: I2C<SDA, SCL, DP>) -> Self {
        Self(UnsafeCell::new(bus))
    }
}

impl<SDA, SCL, DP> SharedI2CBus<SDA, SCL, DP>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    pub fn make_accessor<'a>(&'a self) -> SharedI2CBusAccessor<'a, SDA, SCL, DP> {
        SharedI2CBusAccessor(self)
    }
}

impl<'a, SDA, SCL, DP> Write for SharedI2CBusAccessor<'a, SDA, SCL, DP>
where
    SDA: StatefulOutputPin,
    SCL: StatefulOutputPin,
    DP: ProvideNanosecondDelay,
{
    type Error = ();

    /// single thread guarantie by design
    #[inline]
    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let bus = self.0 .0.get();
        
        unsafe {
            (*bus).begin_transmission(address, true)?;
            (*bus).write(bytes)?;
            (*bus).end_transmission();
        }

        Ok(())
    }
}
