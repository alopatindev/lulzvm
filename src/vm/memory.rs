use byteorder::ByteOrder;
use common::*;
use std::cmp;

pub struct Memory {
    pub raw: Data,
    pub executable_size: Word,
    pub code_size: Word,
    pub stack_offset: Word,
}

impl Memory {
    pub fn from_executable(mut executable: Data) -> Memory {
        let executable_size = executable.len() as Word;
        let new_size = executable_size + REGISTERS_SIZE + STACK_SIZE + EVENT_HANDLERS +
                       EVENT_QUEUE_SIZE;
        executable.resize(new_size as usize, 0);

        let code_size = Self::read_word_from(&executable, CODE_SIZE_OFFSET);
        let stack_offset = executable_size + REGISTERS_SIZE + STACK_SIZE;

        Memory {
            raw: executable,
            executable_size: executable_size,
            code_size: code_size,
            stack_offset: stack_offset,
        }
    }

    pub fn stack(&self, sp: Word) -> DataSlice {
        let sp = sp as usize;
        let stack_offset = self.stack_offset as usize;
        &self.raw[sp..stack_offset]
    }

    pub fn data(&self) -> DataSlice {
        let begin = CODE_OFFSET + self.code_size;
        let begin = cmp::min(begin, self.executable_size);
        let data_size = self.executable_size - begin;
        let begin = begin as usize;
        let data_size = data_size as usize;
        let end = begin + data_size;
        &self.raw[begin..end]
    }

    pub fn is_in_code(&self, index: Word) -> bool {
        index >= CODE_OFFSET && index < CODE_OFFSET + self.code_size
    }

    pub fn code(&self) -> DataSlice {
        let begin = CODE_OFFSET as usize;
        let end = (CODE_OFFSET + self.code_size) as usize;
        &self.raw[begin..end]
    }

    pub fn get(&self, index: Word) -> u8 {
        self.raw[index as usize]
    }

    pub fn put(&mut self, index: Word, value: u8) {
        self.raw[index as usize] = value;
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
