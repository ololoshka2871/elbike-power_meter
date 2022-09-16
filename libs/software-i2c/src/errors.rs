#[derive(Copy, Clone, Debug)]
pub enum Error {
    NoAck,
    StrechTimeout,
}