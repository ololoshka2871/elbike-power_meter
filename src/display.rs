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

const CHART_START_H: i32 = 36;

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

    pub fn set_total_work(&mut self, initial_value: f32) {
        self.work_total = initial_value;
    }

    fn draw_comon(&mut self, p: Watts, ts: u32) -> Result<(), display_interface::DisplayError> {
        self.power_data_points[self.write_pos] = p;
        self.wrap_wp();

        {
            // ~23 ms
            self.disp.clear();

            self.draw_progress_bar(p)?;
            self.draw_total_power(p, ts)?;
            self.draw_chart()?;
        }

        self.disp.flush()?; // ~30ms

        Ok(())
    }

    pub fn draw_frame(&mut self, msg: Message) -> Result<(), display_interface::DisplayError> {
        self.draw_comon(msg.power, msg.end_timestamp)
    }

    pub fn total_power(&self) -> f32 {
        self.work_total
    }

    fn draw_progress_bar(&mut self, power: Watts) -> Result<(), display_interface::DisplayError> {
        let max_wigth = self.disp.dimensions().0 as u32;
        Rectangle::new(
            Point::zero(),
            Size::new(
                Self::transform_size(power.0, max_wigth, MAX_TORQUE),
                YELLOW_LINE_HEIGTH,
            ),
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
        let work = if let Some(prev_ts) = self.prev_timestamp {
            let len = end_ts.wrapping_sub(prev_ts);
            (power.0 * len) as f32 * CPU_CYCLE_TIME_S
        } else {
            0.0
        };
        self.prev_timestamp = Some(end_ts);
        self.work_total += work;

        let mut buf: ArrayString<32> = ArrayString::new();
        write!(buf, ">{:07.2}Вт*c", self.work_total).unwrap();

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
            Point::new(0, CHART_START_H - 1),
            Point::new(self.disp.dimensions().0 as i32, CHART_START_H),
        )
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(&mut self.disp)?;

        Ok(())
    }

    fn draw_chart(&mut self) -> Result<(), display_interface::DisplayError> {
        let last_pixel_h = self.disp.dimensions().1 as i32 - 1;
        let max_heigh = self.disp.dimensions().1 as u32 - CHART_START_H as u32;

        let mut line = Line::new(Point::new(0, last_pixel_h), Point::new(0, last_pixel_h));

        for x in 0..self.power_data_points.len() {
            let element =
                self.power_data_points[(self.write_pos + x) % self.power_data_points.len()];

            line.start.x = x as i32;
            line.end.x = x as i32;
            line.end.y =
                last_pixel_h - Self::transform_size(element.0, max_heigh, MAX_TORQUE) as i32;
            line.into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(&mut self.disp)?;
        }

        Ok(())
    }

    fn wrap_wp(&mut self) {
        self.write_pos = (self.write_pos + 1) % self.power_data_points.len()
    }

    fn transform_size(current: u32, target_max: u32, max_value: u32) -> u32 {
        (target_max * current) / max_value
    }
}
