pub type Word = u16;
pub type Data = Vec<u8>;
pub type DataSlice<'a> = &'a [u8];

pub const STACK_SIZE: usize = 16 * 1024;
pub const REGISTERS_LEN: usize = 8;

pub type Registers = [Word; REGISTERS_LEN];
