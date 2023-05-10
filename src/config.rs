pub const MAX_TORQUE: u32 = 1200;

pub const YELLOW_LINE_HEIGTH: u32 = 16;

// set to 110ms
pub const MAX_CYCLE_TICKS: u32 = 4400000;

// MAX_CYCLE_TICKS -> 110ms => cycle_time = 110E-3 / MAX_CYCLE_TICKS
pub const CPU_CYCLE_TIME_S: f32 = 110E-3f32 / MAX_CYCLE_TICKS as f32;

pub const UART_BOUD: u32 = 9600;
pub const CPU_SPEED_MHZ: u32 = 80;

// write current work into eeptom every results 
pub const UPDATE_EEPROM_EVERY: u32 = 10;