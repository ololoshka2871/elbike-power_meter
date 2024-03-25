#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

mod controller2bc_parcer;
mod display;
//mod logger;
mod nanosecond_delay_provider;
mod uart0_cfg;

mod config;

use core::{fmt::Write, ops::DerefMut};

use controller2bc_parcer::Controller2BCParcer;
use display::Display;
use display_interface::WriteOnlyDataCommand;

use esp8266_hal::{prelude::*, target::Peripherals, time::MegaHertz};
use xtensa_lx::mutex::{CriticalSectionMutex, Mutex};

use config::{CPU_SPEED_MHZ, UART_BOUD, UPDATE_EEPROM_EVERY};

use uart0_cfg::UART0Ex;

use panic_halt as _;

// если не сделать так то очему-то крашит стек
static PARCER: CriticalSectionMutex<Option<Controller2BCParcer>> = CriticalSectionMutex::new(None);

#[entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    let pins = dp.GPIO.split();

    let reset_result_pin = pins.gpio0.into_floating_input();

    let mut serial = dp
        .UART0
        .set_boud_devider(UART_BOUD, MegaHertz(CPU_SPEED_MHZ))
        .serial(pins.gpio1.into_uart(), pins.gpio3.into_uart());

    writeln!(serial, "Startup!\r").unwrap();

    let i2c: esp8266_software_i2c::SharedI2CBus<_, _, _> = esp8266_software_i2c::I2C::new(
        pins.gpio4.into_open_drain_output(),
        pins.gpio5.into_open_drain_output(),
        nanosecond_delay_provider::NanosecondDelayProvider {},
    )
    .set_speed(esp8266_software_i2c::I2CSpeed::Fast400kHz)
    .into();

    let mut storage = eeprom_log::EepromLog::<f32, _, _, _>::init(eeprom24x::Eeprom24x::new_24x08(
        i2c.make_accessor(),
        eeprom24x::SlaveAddr::default(),
    ));
    writeln!(serial, "Storage...\r").unwrap();

    let display_interface = ssd1306::I2CDisplayInterface::new(i2c.make_accessor());
    writeln!(serial, "Display interface...\r").unwrap();

    let mut display = display::Display::new(display_interface);
    writeln!(serial, "Display....\r").unwrap();

    {
        let mut disp_reset_pin = pins.gpio2.into_open_drain_output();
        let (_, mut timer2) = dp.TIMER.timers();
        display.reset(&mut disp_reset_pin, &mut timer2);
        writeln!(serial, "Display reset....\r").unwrap();
    }

    {
        use embedded_graphics::{
            geometry::Dimensions, pixelcolor::BinaryColor, prelude::Primitive,
            primitives::PrimitiveStyle, Drawable,
        };

        display
            .disp
            .bounding_box()
            .offset(-20)
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(&mut display.disp)
            .ok();
        display.disp.flush().ok();
        
        writeln!(serial, "Draw test rect....\r").unwrap();
    }

    {
        let last_work = storage.last().unwrap();
        display.set_total_work(last_work);
        writeln!(serial, "Load last work: {}....\r", last_work).unwrap();
    }

    (&PARCER).lock(|l| *l = Some(Controller2BCParcer::default()));
    let mut serial = serial.attach_interrupt(|_serial| {
        if let Ok(b) = _serial.read() {
            (&PARCER).lock(|l| l.as_mut().unwrap().feed(b));
        }
    });

    writeln!(serial, "Uart parser...\r").unwrap();

    let mut eeprom_update_counter = 0u32;
    let mut reset_pin_was_triggered = false;

    loop {
        if try_process_result(serial.deref_mut(), &mut display) {
            eeprom_update_counter += 1;
            if eeprom_update_counter == UPDATE_EEPROM_EVERY {
                eeprom_update_counter = 0;

                let total_power = if reset_pin_was_triggered && reset_result_pin.is_low().unwrap() {
                    reset_pin_was_triggered = false;
                    display.reset_accumulator();

                    0.0
                } else {
                    reset_pin_was_triggered = reset_result_pin.is_low().unwrap();
                    display.total_power()
                };

                let w_index = storage
                    .append(total_power)
                    .expect("Failed to store in EEPROM");
                writeln!(serial, "EEPROM_STORED: {}: {:.2} \r", w_index, total_power).unwrap();
            }
        }
    }
}

fn try_process_result<'a, SER, DI>(serial: &mut SER, display: &mut Display<'a, DI>) -> bool
where
    DI: WriteOnlyDataCommand,
    SER: core::fmt::Write,
{
    if let Some(result) = (&PARCER).lock(|l| l.as_mut().unwrap().try_get()) {
        display.draw_frame(result).expect("Failed to draw frame");
        let _ = writeln!(serial, "Got message: {:?}\r", result);
        return true;
    }
    false
}
