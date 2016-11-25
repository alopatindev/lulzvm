use common::*;

pub struct Memory<'a> {
    pub raw: Data,
    pub code: DataSlice<'a>,
    pub data: DataSlice<'a>,
    pub stack: DataSlice<'a>,
}

impl<'a> Memory<'a> {
    pub fn stack_is_empty(&self, sp: Word) -> bool {
        unimplemented!()
    }
}
