#[cfg(target_os = "linux")]
pub mod ftdi_linux;

#[cfg(not(target_os = "linux"))]
pub mod ftdi_stub;

pub mod ftdi;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bits {
    Bits7,
    Bits8,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StopBits {
    StopBits1,
    StopBits15,
    StopBits2,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Parity {
    None,
    Odd,
    Even,
    Mark,
    Space,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Configuration {
    pub baud_rate: usize,
    pub bits: Bits,
    pub stop_bits: StopBits,
    pub parity: Parity,
}
