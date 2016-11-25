use common::*;
use std::io::{Read, Write};

mod memory;
mod registers;
mod opcodes;
mod modes;

use self::memory::*;

pub struct VM<'a, R: Read, W: Write> {
    input: R,
    output: W,

    registers: Registers,
    memory: Memory<'a>,
}

impl<'a, R: Read, W: Write> VM<'a, R, W> {
    pub fn new(input: R, output: W) -> Self {
        unimplemented!()
    }

    pub fn run(&mut self, executable: Data) {
        unimplemented!()
    }

    fn parse_executable(executable: Data) -> Memory<'a> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use self::modes::*;
    use self::opcodes::*;
    use self::registers::*;
    use std::io::{BufReader, BufWriter};
    use super::*;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn simple() {
        assert!(run(&[], vec![]).0.is_empty());

        {
            let mut executable = vec![0,
                MOV, REG, A, VALUE, 123, 0,
                ADD, REG, B, REG, A, REG, A];
            let code_length = executable.len() as u8;
            executable[0] = code_length;

            let (output, registers, memory) = run(&[], executable);

            assert_eq!(123, registers[A as usize]);
            assert_eq!(246, registers[B as usize]);
            assert!(output.is_empty());
            assert!(memory.stack_is_empty(registers[SP as usize]));
        }

        {
            let mut executable = vec![0,
                SUB, REG, SP, 4, 0,  // stack allocation for 2 words
                PUSH, VALUE, 5, 0,
                PUSH, VALUE, 6, 0,
                ADD,   REG, A,   PTR, REG, SP,   PTR_WITH_OFFSET, REG, SP, 2, 0,
                // add a, [sp], [sp+2]
                POP, B,
                POP, C,
                ADD, 4, 0];
            let code_length = executable.len() as u8;
            executable[0] = code_length;

            let (output, registers, memory)  = run(&[], executable);

            assert_eq!(11, registers[A as usize]);
            assert_eq!(6, registers[B as usize]);
            assert_eq!(5, registers[C as usize]);
            assert!(output.is_empty());
            assert!(memory.stack_is_empty(registers[SP as usize]));
        }

        {
            let mut executable = vec![0,
                MOV, REG, A, VALUE, 10, 0,
                DEC, REG, A,
                WRITE, REG, A,
                JNZ, 0];
            let code_length = executable.len() as u8;
            executable[0] = code_length;

            let (output, registers, memory) = run(&[], executable);

            assert_eq!(0, registers[A as usize]);
            assert_eq!(&[10, 0, 9, 0, 8, 0, 7, 0, 6, 0, 5, 0,
                         4, 0, 3, 0, 2, 0, 1, 0, 0, 0], output.as_slice());
            assert!(memory.stack_is_empty(registers[SP as usize]));
        }

        {
            let mut executable = vec![0,
                MOV, REG, B, VALUE, 20, 0,       // data address
                MOV, REG, A, PTR, REG, B,        // dereference B
                WRITE, REG, A,
                INC, REG, B,
                INC, REG, B,
                JNZ, 0,

                3, 0,                            // data with address 23
                2, 0,                            // data with address 25
                1, 0,
                0, 0];
            let data_length = 8;
            let code_length = executable.len() as u8 - data_length;
            executable[0] = code_length;

            let (output, registers, memory) = run(&[], executable);

            assert_eq!(8, memory.data.len());
            assert_eq!(0, registers[A as usize]);
            assert_eq!(&[3, 0, 2, 0, 1, 0, 0, 0], output.as_slice());
            assert!(memory.stack_is_empty(registers[SP as usize]));
        }

        {
            let mut executable = vec![0,
                READ, REG, A,
                WRITE, REG, A,
                JNZ, 0];
            let code_length = executable.len() as u8;
            executable[0] = code_length;

            let (output, registers, memory) = run(&[3, 0, 2, 0, 1, 0, 0, 0], executable);
            assert_eq!(0, registers[A as usize]);
            assert_eq!(&[3, 0, 2, 0, 1, 0, 0, 0], output.as_slice());
            assert!(memory.stack_is_empty(registers[SP as usize]));
        }

        {
            let mut executable = vec![0,
                                                   // do
                READ, REG, A,                      //   a = read
                SUB, REG, SP, 2, 0,                //   (stack allocation)
                PUSH, REG, A,
                CALL, VALUE, 28, 0,                //   a = f(a)
                WRITE, REG, A,                     //   print a
                ADD,  REG, SP,  VALUE,  2, 0,      //   (stack deallocation)
                JNZ, 0,                            // while a != 0 goto addr 0
                JMP, 39,                           // exit

                // label f
                PUSH, REG, BP,
                MOV, REG, BP, REG, SP,

                POP, REG, A,
                MUL, REG, A, REG, A, VALUE, 2, 0,  // a = a * 2

                MOV, REG, SP, REG, BP,
                POP, REG, BP,
                RET,

                NOP                                // optional
            ];

            let code_length = executable.len() as u8;
            executable[0] = code_length;

            let (output, registers, memory) = run(&[3, 0, 2, 0, 1, 0, 0, 0],
                                                  executable);

            assert_eq!(0, registers[A as usize]);
            assert_eq!(&[6, 0, 4, 0, 1, 0, 0, 0], output.as_slice());
            assert!(memory.stack_is_empty(registers[SP as usize]));
        }
    }

    fn run<'a>(input: &[u8], executable: Data) -> (Data, Registers, Memory<'a>) {
        let input = BufReader::new(input);

        let output: Data = vec![];
        let output = BufWriter::new(output);

        let mut vm = VM::new(input, output);
        vm.run(executable);

        let output = vm.output
            .get_mut()
            .by_ref()
            .iter()
            .map(|x| *x)
            .collect::<Data>();

        (output, vm.registers, vm.memory)
    }
}
