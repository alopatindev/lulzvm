use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use common::*;
use std::io::{Cursor, Result};

pub struct Memory {
    pub raw: Data,
    executable_size: Word,
    code_size: Word,
}

impl Memory {
    pub fn from_executable(mut executable: Data) -> Memory {
        let executable_size = executable.len() as Word;
        let new_size = executable_size + STACK_SIZE + EVENT_HANDLERS + EVENT_QUEUE_SIZE;
        executable.resize(new_size as usize, 0);

        let code_size = Self::read_word_from(&executable, CODE_SIZE_OFFSET);

        Memory {
            raw: executable,
            executable_size: executable_size,
            code_size: code_size,
        }
    }

    pub fn stack(&self, sp: Word) -> DataSlice {
        let sp = sp as usize;
        let stack_offset = self.executable_size as usize;
        &self.raw[sp..stack_offset]
    }

    pub fn data(&self) -> DataSlice {
        let offset = CODE_OFFSET + self.code_size;
        let data_size = self.executable_size - offset;
        let offset = offset as usize;
        let data_size = data_size as usize;
        &self.raw[offset..(offset + data_size)]
    }

    pub fn is_in_code(&self, index: Word) -> bool {
        index >= CODE_OFFSET && index <= CODE_OFFSET + self.code_size
    }

    pub fn get(&self, index: Word) -> u8 {
        self.raw[index as usize]
    }

    pub fn read_word(&self, index: Word) -> Word {
        Self::read_word_from(&self.raw, index)
    }

    fn read_word_from(data: DataSlice, index: Word) -> Word {
        let index = index as usize;
        let slice = &data[index..(index + WORD_SIZE as usize)];
        Endian::read_u16(slice)
    }
}
