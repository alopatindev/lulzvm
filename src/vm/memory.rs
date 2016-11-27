use common::*;

pub struct Memory<'a> {
    pub raw: Data,

    pub code: DataSlice<'a>,
    pub data: DataSlice<'a>,

    pub stack: DataSlice<'a>,
    pub interruption_handlers: DataSlice<'a>,
    pub interruptions_queue: DataSlice<'a>,
}

impl<'a> Memory<'a> {
    pub fn from_executable(executable: Data) -> Memory<'a> {
        unimplemented!()
    }

    pub fn stack_is_empty(&self, sp: Word) -> bool {
        unimplemented!()
    }
}
