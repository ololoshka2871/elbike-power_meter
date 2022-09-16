//! This module provides the ability to designate any two
//! gpio pins as SDA/SCL which allows you to introduce i2c
//! capabilities into your project.
//!
//! In order to use these pins, you must include a pull-up
//! resistor on both lines.
#![no_std]

#![allow(dead_code)]
#![allow(unused_variables)]

mod nanosecond_delay;
mod shared_i2c_bus;
mod single_client;
mod errors;

pub use shared_i2c_bus::SharedI2CBus;
pub use single_client::I2C;
pub use nanosecond_delay::ProvideNanosecondDelay;
pub use errors::Error;

#[derive(Clone, Copy)]
pub enum I2CSpeed {
    Fast400kHz = 1250,
    Normal100kHz = 2500,
}
