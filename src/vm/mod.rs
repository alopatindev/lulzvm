use common::*;
use std::io::{Read, Write, Result};

mod events;
mod memory;
mod modes;
mod opcodes;
mod registers;

use self::memory::*;
use self::modes::*;
use self::opcodes::*;
use self::registers::*;

pub struct VM<R: Read, W: Write> {
    input: R,
    output: W,

    registers: Registers,
    memory: Memory,
}

impl<R: Read, W: Write> VM<R, W> {
    pub fn new(input: R, output: W, executable: Data) -> Result<Self> {
        let memory = try!(Memory::from_executable(executable));

        let result = VM {
            input: input,
            output: output,
            registers: [0; REGISTERS],
            memory: memory,
        };

        Ok(result)
    }

    pub fn run(&mut self) {
        self.set_register(PC, CODE_OFFSET as Word);

        let mut args = vec![];
        while self.memory.is_in_code(self.get_register(PC)) {
            self.fetch()
                .decode(&mut args)
                .execute(&args)
                .process_events();
            args.clear();
        }
    }

    fn fetch(&mut self) -> &mut Self {
        let code_offset = self.get_register(PC);
        let opcode = self.memory.get(code_offset) as Word;
        self.set_register(IR, opcode);
        self.increment_register(PC);
        self
    }

    fn decode(&mut self, args: &mut Words) -> &mut Self {
        unimplemented!()
        // match self.get_register(IR) as u8 {
        //     MOV => {
        //         args.push(self.read_memory());
        //         args.push(self.read_memory());
        //     }
        //     _ => unimplemented!(),
        // }
        // self
    }

    fn execute(&mut self, args: WordsSlice) -> &mut Self {
        unimplemented!()
    }

    fn process_events(&mut self) {
        unimplemented!()
    }

    // fn read_memory(&mut self) -> Word {
    //     let code_offset = self.get_register(PC);
    //     let mode = self.memory.get(code_offset);
    //     self.increment_register(PC);
    //
    //     match mode {
    //         REG => {
    //             let code_offset = self.get_register(PC);
    //             self.increment_register(PC);
    //             let register_id = self.memory.get(code_offset);
    //             self.get_register(register_id)
    //         }
    //         PTR => unimplemented!(),
    //         PTR_WITH_OFFSET => unimplemented!(),
    //         VALUE => {
    //             let code_offset = self.get_register(PC) as usize;
    //             self.increment_register(PC);
    //             let value = self.memory.read_word(code_offset);
    //             for _ in 0..WORD_SIZE {
    //                 self.increment_register(PC);
    //             }
    //             value
    //         }
    //         _ => unreachable!(),
    //     }
    // }

    fn get_register(&self, id: u8) -> Word {
        self.registers[id as usize]
    }

    fn set_register(&mut self, id: u8, value: Word) {
        self.registers[id as usize] = value;
    }

    fn increment_register(&mut self, id: u8) {
        self.registers[id as usize] += 1;
    }
}

#[cfg(test)]
mod tests {
    use self::events::*;
    use std::io::{BufReader, BufWriter};
    use super::*;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn simple() {
        assert!(run(&[], vec![]).0.is_empty());

        {
            let mut executable = vec![0, 0,
                PUSH, VALUE, 5,
                PUSH, VALUE, 6,
                ADD, STACK, OFFSET, 0, 0];   // pop 2 bytes, add and push
            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let (output, registers, memory)  = run(&[], executable);
            // TODO: stack?

            // check 11
            assert!(output.is_empty());
        }

        // {
        //     let mut executable = vec![0, 0,
        //         PUSH, VALUE, 10,
        //         DEC, STACK, VALUE, 0, 0,
        //         INT, OUTPUT,
        //         JNZ,  STACK, VALUE, 0, 0,  DATA, 5, 0];
        //     let code_size = executable.len() as u8;
        //     executable[0] = code_size;
        //
        //     let (output, registers, memory) = run(&[], executable);
        //
        //     assert_eq!(0, registers[A as usize]);
        //     assert_eq!(0, registers[D as usize]);
        //     assert_eq!(&[9, 8, 7, 6, 5, 4, 3, 2, 1, 0], output.as_slice());
        // }
        //
        // {
        //     let mut executable = vec![0, 0,
        //         PUSH, VALUE, 25,
        //         PUSH, VALUE, 0,                  // data address
        //         LOAD, STACK,
        //         MOV, REG, D, PTR, REG, B,        // dereference B
        //         INT, OUTPUT,
        //         INC, REG, B,
        //         INC, REG, B,
        //         JNZ, REG, D, VALUE, 2, 0,
        //
        //         3, 0,                            // data with address 24
        //         2, 0,                            // data with address 26
        //         1, 0,
        //         0, 0];
        //     let data_length = 8;
        //     let code_size = executable.len() as u8 - data_length;
        //     executable[0] = code_size;
        //
        //     let (output, registers, memory) = run(&[], executable);
        //
        //     // assert_eq!(8, memory.data().len()); // FIXME
        //     assert_eq!(0, registers[A as usize]);
        //     assert_eq!(0, registers[D as usize]);
        //     assert_eq!(&[3, 0, 2, 0, 1, 0, 0, 0], output.as_slice());
        // }
        //
        // {
        //     let mut executable = vec![0, 0,
        //         INT, INPUT,
        //         INT, OUTPUT,
        //         JNZ, REG, D, VALUE, 2, 0];
        //     let code_size = executable.len() as u8;
        //     executable[0] = code_size;
        //
        //     let (output, registers, memory) = run(&[3, 0, 2, 0, 1, 0, 0, 0], executable);
        //
        //     assert_eq!(0, registers[A as usize]);
        //     assert_eq!(0, registers[D as usize]);
        //     assert_eq!(&[3, 0, 2, 0, 1, 0, 0, 0], output.as_slice());
        // }
        //
        // {
        //     let mut executable = vec![0, 0,
        //                                               // do
        //         INT, INPUT,                           // d = read
        //         SUB, REG, SP,  REG, SP,  VALUE, 2, 0, //   (stack allocation)
        //         PUSH, REG, D,
        //         CALL, VALUE, 38, 0,                   //   d = f(d)
        //         INT, OUTPUT,                          //   print a
        //         ADD, REG, SP,  REG, SP,  VALUE, 2, 0, //   (stack deallocation)
        //         JNZ, REG, D, VALUE, 2, 0,             // while d != 0 jmp addr 0
        //         JMP, VALUE, 66, 0,                    // exit
        //
        //         // label f
        //         PUSH, REG, BP,
        //         MOV, REG, BP, REG, SP,
        //
        //         POP, REG, D,
        //         MUL, REG, D, REG, D, VALUE, 2, 0,    // a = a * 2
        //
        //         MOV, REG, SP, REG, BP,
        //         POP, REG, BP,
        //         RET,
        //
        //         NOP                                  // optional
        //     ];
        //
        //     let code_size = executable.len() as u8;
        //     executable[0] = code_size;
        //
        //     let (output, registers, memory) = run(&[3, 0, 2, 0, 1, 0, 0, 0],
        //                                           executable);
        //
        //     assert_eq!(0, registers[A as usize]);
        //     assert_eq!(0, registers[D as usize]);
        //     assert_eq!(&[6, 0, 4, 0, 1, 0, 0, 0], output.as_slice());
        // }
        //
        // {
        //     let mut executable = vec![0, 0,
        //         DIV, REG, A, VALUE, 1, VALUE, 0,
        //         MOV, REG, A, 123,
        //     ];
        //     let code_size = executable.len() as u8;
        //     executable[0] = code_size;
        //
        //     let (output, registers, memory) = run(&[], executable);
        //
        //     assert_eq!(0, registers[A as usize]);
        //     assert_eq!(b"Unknown Error", output.as_slice());
        // }
        //
        // {
        //     let mut executable = vec![0, 0,
        //         MOV,  PTR, VALUE,  255, 255,  VALUE, 1,
        //         MOV, A, 123,
        //     ];
        //     let code_size = executable.len() as u8;
        //     executable[0] = code_size;
        //
        //     let (output, registers, memory) = run(&[], executable);
        //
        //     assert_eq!(0, registers[A as usize]);
        //     assert_eq!(b"Segfault", output.as_slice());
        // }
    }

    fn run(input: &[u8], executable: Data) -> (Data, Registers, Memory) {
        let input = BufReader::new(input);

        let output: Data = vec![];
        let output = BufWriter::new(output);

        let mut vm = VM::new(input, output, executable).unwrap();
        vm.run();

        let output = vm.output
            .get_mut()
            .by_ref()
            .iter()
            .map(|x| *x)
            .collect::<Data>();

        (output, vm.registers, vm.memory)
    }
}
