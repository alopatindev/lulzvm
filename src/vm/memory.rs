use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use common::*;
use std::io::{Cursor, Result};

pub struct Memory {
    pub raw: Data,
    executable_size: usize,
    code_size: usize,
}

pub const CODE_OFFSET: usize = 2;

impl Memory {
    pub fn from_executable(mut executable: Data) -> Result<Memory> {
        let executable_size = executable.len();
        let new_size = executable_size + STACK_SIZE + INTERRUPT_HANDLERS + INTERRUPT_QUEUE_SIZE;
        executable.resize(new_size, 0);

        let code_size = try!(Cursor::new(&executable).read_u16::<LittleEndian>()) as usize;

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

    pub fn data(&self) -> DataSlice {
        let offset = CODE_OFFSET + self.code_size;
        let data_size = self.executable_size - offset;
        &self.raw[offset..(offset + data_size)]
    }

    pub fn stack(&self) -> DataSlice {
        let offset = self.executable_size;
        &self.raw[offset..(offset + STACK_SIZE)]
    }

    pub fn interrupt_handlers(&self) -> DataSlice {
        let stack_offset = self.executable_size;
        let offset = stack_offset + STACK_SIZE;
        &self.raw[offset..(offset + INTERRUPT_HANDLERS)]
    }

    pub fn interrupt_queue(&self) -> DataSlice {
        let stack_offset = self.executable_size;
        let interrupt_handlers_offset = stack_offset + STACK_SIZE;
        let offset = interrupt_handlers_offset + INTERRUPT_HANDLERS;
        &self.raw[offset..(offset + INTERRUPT_QUEUE_SIZE)]
    }
}
