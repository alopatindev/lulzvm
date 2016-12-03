use byteorder::LittleEndian;

macro_rules! to_hex {
    ($data:expr) => {
        format!("{:02x}", $data)
    }
}

pub type Word = u16;
pub type Words = Vec<Word>;
pub type WordsSlice<'a> = &'a [Word];

pub type Endian = LittleEndian;

pub type Data = Vec<u8>;
pub type DataSlice<'a> = &'a [u8];
pub type DataMutSlice<'a> = &'a mut [u8];

pub const REGISTERS: Word = 4;
pub const REGISTERS_SIZE: Word = REGISTERS * WORD_SIZE;

// FIXME: https://github.com/rust-lang/rfcs/pull/253
// pub const WORD_SIZE: usize = mem::size_of::<Word>();
pub const WORD_SIZE: Word = 2;
pub const STACK_SIZE: Word = 16 * 1024;
pub const EVENT_HANDLERS: Word = 8;
pub const EVENT_QUEUE_SIZE: Word = 128;

pub const CODE_SIZE_OFFSET: Word = 0x0;
pub const CODE_OFFSET: Word = CODE_SIZE_OFFSET + WORD_SIZE;

pub type Registers = [Word; REGISTERS as usize];

pub fn data_to_hex(data: DataSlice) -> String {
    data.iter()
        .map(|i| to_hex!(i))
        .collect::<Vec<String>>()
        .join(" ")
}
