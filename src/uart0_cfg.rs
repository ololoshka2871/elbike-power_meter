use esp8266::UART0;
use esp8266_hal::time::Hertz;

pub trait UART0Ex {
    fn set_boud_devider<T: Into<Hertz>>(self, boud: u32, sys_clk: T) -> Self;
}

impl UART0Ex for UART0 {
    fn set_boud_devider<T: Into<Hertz>>(self, boud: u32, sys_clk: T) -> Self {
        unsafe {
            self.uart_clkdiv
                .write(|w| w.uart_clkdiv().bits(sys_clk.into().0 / boud));
        }

        self
    }
}
