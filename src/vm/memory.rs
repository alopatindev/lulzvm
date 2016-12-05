use byteorder::ByteOrder;
use common::*;
use std::cmp;

pub struct Memory {
    pub raw: Data,

    pub executable_size: Word,

    pub code_begin: Word,
    pub code_end: Word,

    pub data_begin: Word,
    pub data_end: Word,

    pub stack_begin: Word,
    pub stack_end: Word,

    pub event_handlers_begin: Word,
    pub event_handlers_end: Word,

    pub event_queue_begin: Word,
    pub event_queue_end: Word,
}

impl Memory {
    pub fn from_executable(mut executable: Data) -> Memory {
        let executable_size = executable.len() as Word;
        let new_size = executable_size + REGISTERS_SIZE + STACK_SIZE + EVENT_HANDLERS_SIZE +
                       EVENT_QUEUE_SIZE;
        executable.resize(new_size as usize, 0);

        let code_size = Self::read_word_from(&executable, CODE_SIZE_OFFSET);
        let code_begin = CODE_OFFSET;
        let code_end = CODE_OFFSET + code_size;

        let stack_begin = executable_size + REGISTERS_SIZE;
        let stack_end = stack_begin + STACK_SIZE;

        let data_begin = cmp::min(CODE_OFFSET + code_size, executable_size);
        let data_end = executable_size;

        let event_handlers_begin = stack_end;
        let event_handlers_end = stack_end + EVENT_HANDLERS_SIZE;

        let event_queue_begin = event_handlers_end;
        let event_queue_end = event_handlers_end + EVENT_QUEUE_SIZE;

        Memory {
            raw: executable,

            executable_size: executable_size,

            code_begin: code_begin,
            code_end: code_end,

            data_begin: data_begin,
            data_end: data_end,

            stack_begin: stack_begin,
            stack_end: stack_end,

            event_handlers_begin: event_handlers_begin,
            event_handlers_end: event_handlers_end,

            event_queue_begin: event_queue_begin,
            event_queue_end: event_queue_end,
        }
    }

    pub fn code(&self) -> DataSlice {
        let begin = self.code_begin as usize;
        let end = self.code_end as usize;
        &self.raw[begin..end]
    }

    pub fn data(&self) -> DataSlice {
        let begin = self.data_begin as usize;
        let end = self.data_end as usize;
        &self.raw[begin..end]
    }

    pub fn stack(&self, sp: Word) -> DataSlice {
        assert_ge!(sp, self.stack_begin);
        assert_le!(sp, self.stack_end);
        let sp = sp as usize;
        let stack_end = self.stack_end as usize;
        &self.raw[sp..stack_end]
    }

    pub fn get_event_handler(&self, event: u8) -> Word {
        let event = event as Word;
        assert_le!(0, event);
        assert_gt!(EVENT_HANDLERS, event);
        let offset = self.event_handlers_begin + event;
        self.read_word(offset)
    }

    pub fn event_queue(&self, ep: Word, ee: Word) -> DataSlice {
        assert_ge!(ep, self.event_queue_begin);
        assert_le!(ep, self.event_queue_end);
        assert_gt!(ee, self.event_queue_begin);
        assert_le!(ee, self.event_queue_end);
        let ep = ep as usize;
        let ee = ee as usize;
        let end = self.event_queue_end as usize;
        &self.raw[ep..ee]
    }

    pub fn is_in_code(&self, index: Word) -> bool {
        index >= self.code_begin && index < self.code_end
    }

    pub fn get(&self, index: Word) -> u8 {
        self.raw[index as usize]
    }

    pub fn put(&mut self, index: Word, value: u8) {
        debug!("put address={} value={}", to_hex!(index), to_hex!(value));
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
