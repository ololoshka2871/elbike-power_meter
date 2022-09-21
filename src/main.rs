#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

mod controller2bc_parcer;
mod uart0_cfg;

use core::{
    borrow::{Borrow, BorrowMut},
    cell::UnsafeCell,
    fmt::Write,
};

use arrayvec::ArrayString;
use controller2bc_parcer::{Controller2BCParcer, Message};
use embedded_graphics::{
    mono_font::{self, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::{Point, Size},
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
    Drawable,
};
use esp8266_hal::{prelude::*, target::Peripherals, time::MegaHertz};

use num::rational::Ratio;
use panic_halt as _;
use ssd1306::prelude::DisplayConfig;
use uart0_cfg::UART0Ex;
use xtensa_lx::mutex::{CriticalSectionMutex, Mutex};

mod logger;
mod nanosecond_delay_provider;

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

    let mut disp_reset_pin = pins.gpio2.into_open_drain_output();
    let _ = disp_reset_pin.set_high();
    timer2.delay_ms(1);
    let _ = disp_reset_pin.set_high();

    let i2c: esp8266_software_i2c::SharedI2CBus<_, _, _> = esp8266_software_i2c::I2C::new(
        pins.gpio5.into_open_drain_output(),
        pins.gpio4.into_open_drain_output(),
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

    //let mut display_reset_pin = pins.gpio5.into_push_pull_output();
    let display_interface = ssd1306::I2CDisplayInterface::new(i2c.make_accessor());

    writeln!(serial, "\nDisplay interface...").unwrap();

    // Font iso_8859_5 есть русские символы, вывод "приямо так".
    // Вычисление выравнивания не работает с русскими символами

    let mut disp = ssd1306::Ssd1306::new(
        display_interface,
        ssd1306::size::DisplaySize128x64,
        ssd1306::rotation::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    writeln!(serial, "\nDisplay....").unwrap();

    disp.init().unwrap();

    writeln!(serial, "\nDisplay Init.....").unwrap();

    //draw_initial_screen(&mut disp).expect("Failed to draw init screeen");
    draw_frame(&mut disp, Message::default()).expect("Failed to draw init screeen");

    writeln!(serial, "\nDisplay draw...").unwrap();

    (&PARCER).lock(|l| *l = Some(Controller2BCParcer::default()));
    let mut serial = serial.attach_interrupt(move |_serial| {
        if let Ok(b) = _serial.read() {
            (&PARCER).lock(|l| l.as_mut().unwrap().feed(b));
        }
    });

    writeln!(serial, "\nUart parcer...").unwrap();

    loop {
        /*
        let mut eeprom_data = [0u8; 128];
        match eeprom.read_data(0, &mut eeprom_data) {
            Ok(_) => writeln!(serial, "Eeprom data: {:?}", eeprom_data),
            Err(e) => writeln!(serial, "Failed to read eeprom: {:?}", e),
        }
        .unwrap();
        */

        // A_0__________
        if let Some(result) = (&PARCER).lock(|l| l.as_mut().unwrap().try_get()) {
            let _ = writeln!(serial, "Got message: {:?}", result);
            draw_frame(&mut disp, result).expect("Failed to draw frame");
        } 
        /*
        else {
            let c = (&PARCER).lock(|l| l.as_ref().unwrap().count());
            let _ = writeln!(serial, "count: {:08}\r", &c);
        }
        */

        /*
        let c = (&PARCER).lock(|l| l.as_ref().unwrap().count());
        let _ = writeln!(serial, "count: {:08}\r", &c);
        */

        //writeln!(serial, "Ping").unwrap();
        timer2.delay_ms(100);
    }
}

fn draw_frame<DI, SIZE>(
    disp: &mut ssd1306::Ssd1306<DI, SIZE, ssd1306::mode::BufferedGraphicsMode<SIZE>>,
    msg: Message,
) -> Result<(), display_interface::DisplayError>
where
    DI: display_interface::WriteOnlyDataCommand,
    SIZE: ssd1306::size::DisplaySize,
{
    let small_font_italic = MonoTextStyleBuilder::new()
        .font(&mono_font::iso_8859_5::FONT_6X13_ITALIC)
        .text_color(BinaryColor::On)
        .build();

    let display_dim = disp.dimensions();
    let _display_dim = (display_dim.0 as i32, display_dim.1 as i32);

    let mut buf: ArrayString<256> = ArrayString::new();

    write!(buf, "{}", msg).unwrap();

    Text::with_baseline(
        buf.as_str(),
        Point::new(18, 0),
        small_font_italic,
        Baseline::Top,
    )
    .draw(disp)?;

    disp.flush()?;

    Ok(())
}

/*
fn draw_initial_screen<DI, SIZE>(
    disp: &mut ssd1306::Ssd1306<DI, SIZE, ssd1306::mode::BufferedGraphicsMode<SIZE>>,
) -> Result<(), display_interface::DisplayError>
where
    DI: display_interface::WriteOnlyDataCommand,
    SIZE: ssd1306::size::DisplaySize,
{
    let big_font = MonoTextStyleBuilder::new()
        .font(&mono_font::iso_8859_5::FONT_10X20)
        .text_color(BinaryColor::On)
        .build();

    let small_font_italic = MonoTextStyleBuilder::new()
        .font(&mono_font::iso_8859_5::FONT_6X13_ITALIC)
        .text_color(BinaryColor::On)
        .build();

    let display_w = disp.dimensions().0 as i32;

    disp.flush().unwrap();

    Text::with_baseline("Измеритель", Point::new(18, -3), big_font, Baseline::Top).draw(disp)?;
    Text::with_baseline(
        "динамического",
        Point::new(
            0, //(display_h / 2).into(),
            big_font.font.character_size.height as i32 - 3 - 3,
        ),
        big_font,
        Baseline::Top,
    )
    .draw(disp)?;
    Text::with_baseline(
        "сопротивления",
        Point::new(
            0, //(display_h / 2).into(),
            (big_font.font.character_size.height as i32 - 3) * 2 - 3,
        ),
        big_font,
        Baseline::Top,
    )
    .draw(disp)?;

    Rectangle::new(
        Point::new(
            0,
            disp.dimensions().1 as i32 - small_font_italic.font.character_size.height as i32 + 1,
        ),
        Size::new(
            display_w as u32,
            small_font_italic.font.character_size.height - 1,
        ),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
    .draw(disp)?;

    Text::new(
        "СКТБ ЭлПА(c)",
        Point::new(
            (Ratio::<i32>::new(1, 4) * display_w).to_integer() as i32,
            disp.dimensions().1 as i32 - 2,
        ),
        MonoTextStyleBuilder::from(&small_font_italic)
            .background_color(BinaryColor::On)
            .text_color(BinaryColor::Off)
            .build(),
    )
    .draw(disp)?;

    disp.flush()?;

    Ok(())
}
*/
