use core::fmt::Write;

use arrayvec::ArrayString;
use display_interface::WriteOnlyDataCommand;
use embedded_graphics::{
    mono_font::{self, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::{Point, Size},
    primitives::{Line, Primitive, PrimitiveStyle, Rectangle},
    text::{Baseline, Text, TextStyleBuilder},
    Drawable,
};
use embedded_hal::{blocking::delay::DelayMs, digital::v2::OutputPin};
use ssd1306::{mode::BufferedGraphicsMode, prelude::DisplayConfig, Ssd1306};

use crate::{
    config::{CPU_CYCLE_TIME_S, MAX_TORQUE, YELLOW_LINE_HEIGTH},
    controller2bc_parcer::{Message, Watts},
};

// Font iso_8859_5 есть русские символы, вывод "приямо так".
// Вычисление выравнивания не работает с русскими символами
pub struct Display<'a, DI> {
    disp: Ssd1306<
        DI,
        ssd1306::size::DisplaySize128x64,
        BufferedGraphicsMode<ssd1306::size::DisplaySize128x64>,
    >,
    progress_bar_font: MonoTextStyle<'a, BinaryColor>,
    big_font: MonoTextStyle<'a, BinaryColor>,

    power_data_points: [Watts; 128],
    write_pos: usize,

    prev_timestamp: Option<u32>,
    work_total: f32,
}

impl<'a, DI> Display<'a, DI>
where
    DI: WriteOnlyDataCommand,
{
    pub fn new(interface: DI) -> Self {
        Self {
            disp: ssd1306::Ssd1306::new(
                interface,
                ssd1306::size::DisplaySize128x64,
                ssd1306::rotation::DisplayRotation::Rotate0,
            )
            .into_buffered_graphics_mode(),
            progress_bar_font: MonoTextStyleBuilder::new()
                .font(&mono_font::iso_8859_5::FONT_7X13_BOLD)
                .text_color(BinaryColor::On)
                .build(),
            big_font: MonoTextStyleBuilder::new()
                .font(&mono_font::iso_8859_5::FONT_10X20)
                .text_color(BinaryColor::On)
                .build(),
            power_data_points: [Watts(0); 128],
            write_pos: 0,
            prev_timestamp: None,
            work_total: 0.0,
        }
    }

    pub fn reset<O: OutputPin, D: DelayMs<u32>>(
        &mut self,
        reset_pin: &mut O,
        delay_provider: &mut D,
    ) {
        let _ = reset_pin.set_low();
        delay_provider.delay_ms(1);
        let _ = reset_pin.set_high();
        delay_provider.delay_ms(1);

        self.disp.init().expect("failed to init display");
    }

    pub fn draw_frame(&mut self, msg: Message) -> Result<(), display_interface::DisplayError> {
        self.disp.clear();

        let display_dim = self.disp.dimensions();
        let _display_dim = (display_dim.0 as i32, display_dim.1 as i32);

        self.power_data_points[self.write_pos] = msg.power;
        self.work_total += msg.power.0 as f32;
        //let work_fragment = self.power_data_points.iter().fold(0u32, |acc, p| acc + p.0);
        self.wrap_wp();

        self.draw_progress_bar(msg.power)?;
        self.draw_total_power(msg.power, msg.end_timestamp)?;

        self.disp.flush()?;

        Ok(())
    }

    pub fn duplucate_current_frame(&mut self, ts: u32) -> Result<(), display_interface::DisplayError> {
        self.disp.clear();

        // duplicate last point
        let diplicate_result = if self.write_pos == 0 {
            self.power_data_points[self.power_data_points.len() - 1]
        } else {
            self.power_data_points[self.write_pos - 1]
        };

        self.power_data_points[self.write_pos] = diplicate_result;
        self.work_total += diplicate_result.0 as f32;

        self.wrap_wp();

        self.draw_progress_bar(diplicate_result)?;
        self.draw_total_power(diplicate_result, ts)?;

        self.disp.flush()?;

        Ok(())
    }

    fn draw_progress_bar(&mut self, power: Watts) -> Result<(), display_interface::DisplayError> {
        let max_wigth = self.disp.dimensions().0 as u32;
        Rectangle::new(
            Point::zero(),
            Size::new((max_wigth * power.0) / MAX_TORQUE, YELLOW_LINE_HEIGTH),
        )
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(&mut self.disp)?;

        let mut buf: ArrayString<16> = ArrayString::new();
        write!(buf, "{}", power).unwrap();

        Text::with_text_style(
            &buf,
            Point::new(max_wigth as i32 / 2, 2),
            MonoTextStyleBuilder::from(&self.progress_bar_font)
                .background_color(BinaryColor::On)
                .text_color(BinaryColor::Off)
                .build(),
            TextStyleBuilder::new()
                .alignment(embedded_graphics::text::Alignment::Center)
                .baseline(Baseline::Top)
                .build(),
        )
        .draw(&mut self.disp)?;

        Ok(())
    }

    fn draw_total_power(
        &mut self,
        power: Watts,
        end_ts: u32,
    ) -> Result<(), display_interface::DisplayError> {
        let total_power = if let Some(prev_ts) = self.prev_timestamp {
            let len = end_ts.wrapping_sub(prev_ts);
            (power.0 * len) as f32 * CPU_CYCLE_TIME_S
        } else {
            0.0
        };
        self.prev_timestamp = Some(end_ts);

        let mut buf: ArrayString<32> = ArrayString::new();
        write!(buf, "> {:06.2}Вт*ч", total_power).unwrap();

        Text::with_text_style(
            &buf,
            Point::new(0, 17),
            self.big_font,
            TextStyleBuilder::new()
                .alignment(embedded_graphics::text::Alignment::Left)
                .baseline(Baseline::Top)
                .build(),
        )
        .draw(&mut self.disp)?;

        Line::new(
            Point::new(0, 35),
            Point::new(self.disp.dimensions().0 as i32, 35),
        )
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(&mut self.disp)?;

        Ok(())
    }

    fn wrap_wp(&mut self) {
        self.write_pos = if self.write_pos == self.power_data_points.len() - 1 {
            0
        } else {
            self.write_pos + 1
        }
    }
}
