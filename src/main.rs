#![no_std]
#![no_main]

use embedded_graphics::{
    mono_font::{self, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::{Point, Size},
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
    Drawable,
};
use esp8266_hal::{prelude::*, target::Peripherals};

use num::rational::Ratio;
use panic_halt as _;
use ssd1306::prelude::DisplayConfig;

/*
extern crate esp_idf_alloc;

#[global_allocator]
static A: esp_idf_alloc::EspIdfAllocator = esp_idf_alloc::EspIdfAllocator;
*/

#[entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    let pins = dp.GPIO.split();

    let mut display_reset_pin = pins.gpio5.into_push_pull_output();
    let display_interface = display_interface_spi::SPIInterface::new(
        esp8266_hal::spi::SPI1Master::new(
            dp.SPI1,
            pins.gpio14.into_hspi(),
            pins.gpio12.into_hspi(),
            pins.gpio13.into_hspi(),
            esp8266_hal::spi::SpiClock::Spi2MHz,
        ),
        pins.gpio2.into_push_pull_output(),
        pins.gpio4.into_push_pull_output(),
    );
    let (_, mut timer2) = dp.TIMER.timers();

    // Font iso_8859_5 есть русские символы, вывод "приямо так".
    // Вычисление выравнивания не работает с русскими символами

    let mut disp = ssd1306::Ssd1306::new(
        display_interface,
        ssd1306::size::DisplaySize128x64,
        ssd1306::rotation::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    disp.reset(&mut display_reset_pin, &mut timer2).unwrap();
    disp.init().unwrap();

    draw_initial_screen(&mut disp).expect("Failed to draw init screeen");

    loop {
        timer2.delay_ms(1000u32);
    }
}

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
