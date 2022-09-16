use esp8266_hal::time::Nanoseconds;

pub trait ProvideNanosecondDelay {
    fn delay_ns(&self, ns: Nanoseconds);
    fn nanos(&self) -> Nanoseconds;
}