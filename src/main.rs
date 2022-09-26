#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

mod controller2bc_parcer;
mod display;
mod logger;
mod nanosecond_delay_provider;
mod uart0_cfg;

mod config;

use core::{fmt::Write, ops::DerefMut};

use config::MAX_CYCLE_TICKS;
use controller2bc_parcer::{Controller2BCParcer, Message};
use display::Display;
use display_interface::WriteOnlyDataCommand;

use esp8266_hal::{prelude::*, target::Peripherals, time::MegaHertz};
use xtensa_lx::{
    mutex::{CriticalSectionMutex, Mutex},
    timer::{delay, get_cycle_count},
};

use uart0_cfg::UART0Ex;

use panic_halt as _;

const UART_BOUD: u32 = 115200;
const CPU_SPEED_MHZ: u32 = 80;

// если не сделать так то очему-то крашит стек
static PARCER: CriticalSectionMutex<Option<Controller2BCParcer>> = CriticalSectionMutex::new(None);

#[entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    let pins = dp.GPIO.split();

    let mut serial = dp
        .UART0
        .set_boud_devider(UART_BOUD, MegaHertz(CPU_SPEED_MHZ))
        .serial(pins.gpio1.into_uart(), pins.gpio3.into_uart());

    writeln!(serial, "\nStartup!").unwrap();

    let (_, mut timer2) = dp.TIMER.timers();

    let i2c: esp8266_software_i2c::SharedI2CBus<_, _, _> = esp8266_software_i2c::I2C::new(
        pins.gpio4.into_open_drain_output(),
        pins.gpio5.into_open_drain_output(),
        nanosecond_delay_provider::NanosecondDelayProvider {
            minimal: 50,
            k: 640,
        },
    )
    .set_speed(esp8266_software_i2c::I2CSpeed::Fast400kHz)
    .into();

    /*
    let mut eeprom =
        eeprom24x::Eeprom24x::new_24x08(i2c.make_accessor(), eeprom24x::SlaveAddr::default());
    */

    let display_interface = ssd1306::I2CDisplayInterface::new(i2c.make_accessor());
    writeln!(serial, "\nDisplay interface...").unwrap();

    let mut display = display::Display::new(display_interface);
    writeln!(serial, "\nDisplay....").unwrap();

    let mut disp_reset_pin = pins.gpio2.into_open_drain_output();
    display.reset(&mut disp_reset_pin, &mut timer2);
    writeln!(serial, "\nDisplay reset....").unwrap();

    (&PARCER).lock(|l| *l = Some(Controller2BCParcer::default()));
    let mut serial = serial.attach_interrupt(move |_serial| {
        if let Ok(b) = _serial.read() {
            (&PARCER).lock(|l| l.as_mut().unwrap().feed(b));
        }
    });

    writeln!(serial, "\nUart parcer...").unwrap();

    let mut start = get_cycle_count();
    let mut end = start.wrapping_add(MAX_CYCLE_TICKS);

    //timeout_result(serial.deref_mut(), &mut display);

    'main: loop {
        /*
        let mut eeprom_data = [0u8; 128];
        match eeprom.read_data(0, &mut eeprom_data) {
            Ok(_) => writeln!(serial, "Eeprom data: {:?}", eeprom_data),
            Err(e) => writeln!(serial, "Failed to read eeprom: {:?}", e),
        }
        .unwrap();
        */

        if end < start {
            // wrap
            let mut now = get_cycle_count();
            while now > end && now < start {
                if try_process_result(serial.deref_mut(), &mut display, &mut start, &mut end) {
                    continue 'main;
                }
                now = get_cycle_count();
            }
        } else {
            // normal
            while get_cycle_count() < end {
                if try_process_result(serial.deref_mut(), &mut display, &mut start, &mut end) {
                    continue 'main;
                }
            }
        }

        /*
        {
            disp_reset_pin.set_low();
            timer2.delay_us(1);
            disp_reset_pin.set_high();
        }
        */

        timeout_result(serial.deref_mut(), &mut display);
        start = end.wrapping_add(MAX_CYCLE_TICKS);
        end = start.wrapping_add(MAX_CYCLE_TICKS);

        /*
        if let Some(result) = (&PARCER).lock(|l| l.as_mut().unwrap().try_get()) {
            let _ = writeln!(serial, "Got message: {:?}", result);
            display.draw_frame(result).expect("Failed to draw frame");
        }
        */
    }
}

fn try_process_result<'a, SER, DI>(
    serial: &mut SER,
    display: &mut Display<'a, DI>,
    start: &mut u32,
    end: &mut u32,
) -> bool
where
    DI: WriteOnlyDataCommand,
    SER: core::fmt::Write,
{
    if let Some(result) = (&PARCER).lock(|l| l.as_mut().unwrap().try_get()) {
        let _ = writeln!(serial, "Got message: {:?}", result);
        display.draw_frame(result).expect("Failed to draw frame");

        delay(end.wrapping_sub(get_cycle_count()));
        *start = end.wrapping_add(MAX_CYCLE_TICKS);
        *end = start.wrapping_add(MAX_CYCLE_TICKS);

        return true;
    }
    false
}

fn timeout_result<'a, SER: core::fmt::Write, DI: WriteOnlyDataCommand>(
    serial: &mut SER,
    display: &mut Display<'a, DI>,
) {
    //let _ = writeln!(serial, "Message timeout: {}ticks", MAX_CYCLE_TICKS);
    display
        .duplucate_current_frame(MAX_CYCLE_TICKS)
        .expect("Failed to draw dule frame");
}
