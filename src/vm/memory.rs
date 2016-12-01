use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use common::*;
use std::io::{Cursor, Result};

pub struct Memory {
    pub raw: Data,
    executable_size: usize,
    code_size: usize,
}

impl Memory {
    pub fn from_executable(mut executable: Data) -> Result<Memory> {
        let executable_size = executable.len();
        let new_size = executable_size + STACK_SIZE + EVENT_HANDLERS + EVENT_QUEUE_SIZE;
        executable.resize(new_size, 0);

        let code_size = Self::read_word_from(&executable, CODE_SIZE_OFFSET) as usize;

        let result = Memory {
            raw: executable,
            executable_size: executable_size,
            code_size: code_size,
        };

        Ok(result)
    }

    pub fn stack_is_empty(&self, sp: Word) -> bool {
        let stack_offset = self.executable_size;
        sp as usize == stack_offset + STACK_SIZE - 1
    }

    pub fn code(&self) -> DataSlice {
        &self.raw[CODE_OFFSET..(CODE_OFFSET + self.code_size)]
    }

    pub fn is_in_code(&self, index: Word) -> bool {
        let index = index as usize;
        index >= CODE_OFFSET && index <= CODE_OFFSET + self.code_size
    }

    pub fn get(&self, index: Word) -> u8 {
        self.raw[index as usize]
    }

    pub fn read_word(&self, index: usize) -> Word {
        Self::read_word_from(&self.raw, index)
    }

    pub fn read_word_from(data: DataSlice, index: usize) -> Word {
        let slice = &data[index..(index + WORD_SIZE)];
        Endian::read_u16(slice)
    }
}
