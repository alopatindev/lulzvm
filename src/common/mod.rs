pub type Word = u16;
pub type Words = Vec<Word>;
pub type WordsSlice<'a> = &'a [Word];

pub type Data = Vec<u8>;
pub type DataSlice<'a> = &'a [u8];

pub const STACK_SIZE: usize = 16 * 1024;
pub const REGISTERS: usize = 8;
pub const INTERRUPT_HANDLERS: usize = 8;
pub const INTERRUPT_QUEUE_SIZE: usize = 128;

pub type Registers = [Word; REGISTERS];
