use core::cell::UnsafeCell;

use embedded_hal::{
    blocking::i2c::{Write, WriteRead},
    digital::v2::StatefulOutputPin,
};

use crate::{nanosecond_delay::ProvideNanosecondDelay, single_client::I2C};

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
    type Error = crate::errors::Error;

    /// single thread guarantie by design
    #[inline]
    fn write(&mut self, address: u8, bytes: &[u8]) -> core::result::Result<(), Self::Error> {
        let bus = self.0 .0.get();

        unsafe {
            (*bus).begin_transmission(address, true)?;
            (*bus).write(bytes)?;
            (*bus).end_transmission();
        }

        Ok(())
    }
}

impl<'a, SDA, SCL, DP> WriteRead for SharedI2CBusAccessor<'a, SDA, SCL, DP>
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

        let bus = self.0 .0.get();
        unsafe {
            (*bus).begin_transmission(address, true)?;
            (*bus).write(bytes)?;
            for place in buffer.iter_mut() {
                match (*bus).read(true) {
                    Ok(v) => *place = v,
                    Err(e) => {
                        res = Err(e);
                        break;
                    }
                }
            }
            (*bus).end_transmission();
        }

        res
    }
}
