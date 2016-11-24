use std::io::{Read, Write};

const STACK_LEN: usize = 128;
const REGISTERS_LEN: usize = 8;

pub type Word = u64;

pub struct VM<R: Read, W: Write> {
    input: R,
    output: W,

    registers: Registers,
    memory: Memory,
}

pub type Registers = [Word; REGISTERS_LEN];

pub struct Memory {
    stack: [StackFrame; STACK_LEN],
    code: Vec<u8>,
    data: Vec<u8>,
}

pub struct StackFrame {
    data: Vec<u8>,
    return_address: Option<Word>,
}

impl<R: Read, W: Write> VM<R, W> {
    // TODO
}

#[cfg(test)]
mod tests {
    #[test]
    fn simple() {
        assert!(false)
    }
}
