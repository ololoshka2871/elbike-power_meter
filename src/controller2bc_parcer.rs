use core::fmt::Display;

use esp8266_hal::time::Milliseconds;

use num::FromPrimitive;
use num_derive::FromPrimitive;

const WATS_PER_UNIT: u32 = 13;

#[derive(Default)]
pub struct Controller2BCParcer {
    raw_data: [u8; 12],
    wp: usize,
}

#[derive(Debug, FromPrimitive)]
pub enum BatLevel {
    EmptyBox = 0,
    BorderFlashing = 1,
    Charging = 2,
    Empty = 3,
    Lvl1 = 4,
    Lvl2 = 5,
    Lvl3 = 6,
    Lvl4 = 7,
    Lvl5 = 8,
    Lvl6 = 9,
    Lvl7 = 10,
    Lvl8 = 11,
    Lvl9 = 12,
    Lvl10 = 13,
    Lvl11 = 14,
    Lvl12 = 15,
    Lvl13 = 16,
}

#[derive(Debug, FromPrimitive)]
pub enum Error {
    Info0 = 0x20,
    Info6 = 0x21,
    Info1 = 0x22,
    Info2 = 0x23,
    Info3 = 0x24,
    Info0_1 = 0x25,
    Info4 = 0x26,
    Info0_2 = 0x28,
}

#[derive(Debug, FromPrimitive)]
pub enum MovingMode {
    Idle = 0,
    AnimateThrottle = 1 << 0,
    Cruise = 1 << 3,
    Asist = 1 << 4,
}

#[derive(Debug)]
struct Watts(pub u32);

#[derive(Debug)]
struct Celsius(pub i8);

#[derive(Debug)]
pub struct Message {
    bat_lvl: BatLevel,
    wheel_rotation_period: Milliseconds,
    error: Error,
    crc: u8,
    moving_mode: MovingMode,
    power: Watts,
    motor_temperature: Celsius,
}

impl Controller2BCParcer {
    pub fn feed(&mut self, data: u8) {
        let ok = match self.wp {
            0 => data == 0x41,
            2 => data == 0x30,
            10 | 11 => data != 0,
            1 | (3..=9) => {
                self.raw_data[self.wp] = data;
                true
            }
            _ => return,
        };

        if ok {
            self.wp += 1;
        } else {
            self.wp = 0; // reset
        }
    }

    pub fn try_get(&mut self) -> Option<Message> {
        if self.wp == self.raw_data.len() {
            let res = Message {
                bat_lvl: FromPrimitive::from_u8(self.raw_data[1]).unwrap_or_default(),
                wheel_rotation_period: {
                    let mut tmp = [0u8; core::mem::size_of::<u16>()];
                    tmp.clone_from_slice(&self.raw_data[3..=4]);
                    Milliseconds(u16::from_be_bytes(tmp) as u32)
                },
                error: FromPrimitive::from_u8(self.raw_data[5]).unwrap_or_default(),
                crc: self.raw_data[6],
                moving_mode: FromPrimitive::from_u8(self.raw_data[7]).unwrap_or_default(),
                power: Watts(self.raw_data[8] as u32 * WATS_PER_UNIT),
                motor_temperature: Celsius(self.raw_data[9] as i8),
            };
            self.wp = 0;

            Some(res)
        } else {
            None
        }
    }
}

//-----------------------------------------------------------------------------

impl Default for MovingMode {
    fn default() -> Self {
        Self::Idle
    }
}

impl Default for BatLevel {
    fn default() -> Self {
        Self::EmptyBox
    }
}

impl Default for Error {
    fn default() -> Self {
        Self::Info0
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            bat_lvl: Default::default(),
            wheel_rotation_period: Milliseconds(0),
            error: Default::default(),
            crc: Default::default(),
            moving_mode: Default::default(),
            power: Watts(0),
            motor_temperature: Celsius(0),
        }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            r#"bat_lvl: {bat_lvl:?},
wheel_rp: {wheel_rotation_period}ms,
error: {error:?},
crc: 0x{crc:x},
moving_mode: {moving_mode:?},
power: {power}W,
T: {motor_temperature}*C"#,
            bat_lvl = self.bat_lvl,
            wheel_rotation_period = self.wheel_rotation_period.0,
            error = self.error,
            crc = self.crc,
            moving_mode = self.moving_mode,
            power = self.power.0,
            motor_temperature = self.motor_temperature.0,
        )
    }
}
