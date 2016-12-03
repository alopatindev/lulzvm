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
    pub fn new(input: R, output: W, executable: Data) -> Self {
        let memory = Memory::from_executable(executable);

        VM {
            input: input,
            output: output,
            registers: [0; REGISTERS],
            memory: memory,
        }
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

    pub fn stack(&self) -> DataSlice {
        let sp = self.get_register(SP);
        self.memory.stack(sp)
    }

    pub fn data(&self) -> DataSlice {
        self.memory.data()
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
        {
            let (output, vm) = run(&[], vec![]);

            assert!(output.is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
        }

        {
            let mut executable = vec![
                0, 0,
                PUSH, 3,                       // b
                PUSH, 4,                       // a
                ADD];                          // pop 2 bytes,
                                               // add (a + b) and push
            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert_eq!(&[7], vm.stack());
            assert!(output.is_empty());
        }

        {
            let mut executable = vec![
                0, 0,
                PUSH, 0,                       // b
                PUSH, 10,                      // a
                DEC,                           // a--
                INT, OUTPUT,
                JNE, 6, 0];                    // a != b
            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert_eq!(&[9, 8, 7, 6, 5, 4, 3, 2, 1, 0], output.as_slice());
            assert_eq!([0], vm.stack());
        }

        {
            let mut executable = vec![
                0, 0,
                PUSH, 0,                       // zero
                PUSH, 0,                       // offset

                // label loop
                LOAD, PTR_WITH_OFFSET, 21, 0,  // d = [data segment + offset]
                INT, OUTPUT,                   // print d
                JE, 20, 0,                     // if d == zero: goto end
                POP,                           // pop d
                INC,                           // offset++
                JMP, 6, 0,                     // goto loop

                // label end
                NOP,                           // optional
                3, 2, 1, 0];
            let data_size = 4;
            let code_size = executable.len() as u8 - data_size;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert_eq!(4, vm.data().len());
            assert_eq!(&[0], vm.stack());
            assert_eq!(&[3, 2, 1], output.as_slice());
        }

        {
            let mut executable = vec![
                0, 0,
                PUSH, 0,                       // b

                // label loop
                INT, INPUT,                    // a
                INT, OUTPUT,                   // print a
                JE, 15, 0,                     // if a == b: goto end
                POP,                           // remove a
                JMP, 4, 0                      // goto loop

                // label end
                // NOP                         // optional
                ];
            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let input = [3, 2, 1, 0];
            let (output, vm) = run(&input, executable);

            assert_eq!(&[0, 0], vm.stack());
            assert_eq!(&[3, 2, 1, 0], output.as_slice());
        }

        {
            let mut executable = vec![
                0, 0,
                PUSH, 4,                       // b
                PUSH, 12,                      // a
                DIV];                          // a / b
            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert!(output.is_empty());
            assert_eq!(&[3], vm.stack());
        }

        {
            let mut executable = vec![
                0, 0,
                PUSH, 0,                       // b
                PUSH, 1,                       // a
                DIV];                          // a / b
            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert_eq!(b"Unknown Error", output.as_slice());
            assert!(vm.stack().is_empty());
        }

        {
            let mut executable = vec![
                0, 0,
                POP];
            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert_eq!(b"Segfault", output.as_slice());
            assert!(vm.stack().is_empty());
        }

        {
            let mut executable = vec![
                0, 0,
                LOAD, PTR, 255, 255];          // access violation
            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert_eq!(b"Segfault", output.as_slice());
            assert!(vm.stack().is_empty());
        }

        {
            let mut executable = vec![
                0, 0,
                LOAD, PTR, 2, 0];              // try to load code segment
                                               // as data segment
            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert_eq!(b"Segfault", output.as_slice());
            assert!(vm.stack().is_empty());
        }

        {
            let mut executable = vec![
                0, 0,
                LOAD, PTR, 6, 0,
                123];
            let data_size = 1;
            let code_size = executable.len() as u8 - data_size;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert_eq!(&[123], vm.stack());
        }

        {
            let mut executable = vec![
                0, 0,
                PUSH, 0,                       // zero
                                               // loop:
                INT, INPUT,                    //   x = read
                CALL, PTR, 20, 0,              //   x = f(x)
                INT, OUTPUT,                   //   print x
                JE, 18, 0,                     // if x == zero: goto exit
                POP,                           // pop x
                JMP, 2, 0,                     // goto loop
                INT, TERMINATE,                // exit:

                                               // f:
                PUSH, 2,
                MUL,                           // a = a * 2
                RET];

            let code_size = executable.len() as u8;
            executable[0] = code_size;

            let input = [3, 2, 1, 0];
            let (output, vm) = run(&input, executable);

            assert_eq!(&[6, 4, 2, 0], output.as_slice());
            assert_eq!(&[0, 0], vm.stack());
        }

        {
            let mut executable = vec![
                0, 0,
                PUSH, 0,                       // i
                                               // loop:
                LOAD, 19, 0,                   // x
                JLE, 19, 0,                    // if x <= i goto: end
                DEC,
                STORE, 19, 0,                  // x
                POP,
                INC,
                JMP, 4, 0,                     // goto loop
                                               // end:
                5];                            // x
            let data_size = 1;
            let code_size = executable.len() as u8 - data_size;
            executable[0] = code_size;

            let (output, vm) = run(&[], executable);

            assert_eq!(&[3], vm.stack());
            assert_eq!(&[2], vm.data());
        }
    }

    fn run(input: DataSlice,
           executable: Data)
           -> (Data, VM<BufReader<DataSlice>, BufWriter<Data>>) {
        let input = BufReader::new(input);

        let output: Data = vec![];
        let output = BufWriter::new(output);

        let mut vm = VM::new(input, output, executable);
        vm.run();

        let output = vm.output
            .get_mut()
            .by_ref()
            .iter()
            .map(|x| *x)
            .collect::<Data>();

        (output, vm)
    }
}
