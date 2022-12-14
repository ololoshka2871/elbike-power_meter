use core::arch::asm;

use esp8266_hal::{ram, time::Nanoseconds};

pub(crate) struct NanosecondDelayProvider;

impl esp8266_software_i2c::ProvideNanosecondDelay for NanosecondDelayProvider {
    // ЖЭкспериментально подобрано
    #[ram]
    fn delay_ns(&self, _ns: Nanoseconds) {
        // xtensa_lx::timer::delay(ns.0.saturating_sub(self.minimal) / self.k)
        // ровно 400Khz
        unsafe {
            asm!("nop");
            asm!("nop");
            asm!("nop");
            //asm!("nop");
        }
    }

    fn nanos(&self) -> Nanoseconds {
        Nanoseconds(xtensa_lx::timer::get_cycle_count() * 50)
    }
}
