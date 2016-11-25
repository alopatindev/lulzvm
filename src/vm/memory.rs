use common::*;

pub struct Memory<'a> {
    pub raw: Data,
    pub code: DataSlice<'a>,
    pub data: DataSlice<'a>,
    pub stack: DataSlice<'a>,
}
