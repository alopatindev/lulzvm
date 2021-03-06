pub const CLOCK: u8 = 0x00;

pub const INPUT: u8 = 0x01;
pub const OUTPUT: u8 = 0x02;

pub const TERMINATE: u8 = 0x03;
pub const SEGFAULT: u8 = 0x04;
pub const UNKNOWN_ERROR: u8 = 0x05;

pub fn is_critical(id: u8) -> bool {
    id >= TERMINATE
}
