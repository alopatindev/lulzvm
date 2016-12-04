use common::*;
use std::fmt;
use std::io::{Read, Write};
use std::num::Wrapping;

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
            registers: [0; REGISTERS as usize],
            memory: memory,
        }
    }

    pub fn run(&mut self) {
        self.set_register(PC, CODE_OFFSET as Word);

        let stack_offset = self.memory.stack_offset;
        self.set_register(SP, stack_offset);

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

    pub fn code(&self) -> DataSlice {
        self.memory.code()
    }

    pub fn data(&self) -> DataSlice {
        self.memory.data()
    }

    fn fetch(&mut self) -> &mut Self {
        let opcode = self.next_code_byte() as Word;
        debug!("fetch {:?}", self);
        self.set_register(IR, opcode);
        self
    }

    fn decode(&mut self, args: &mut Data) -> &mut Self {
        let opcode = self.get_register(IR) as u8;
        debug!("decode {:?}", self);

        match opcode {
            PUSH => args.push(self.next_code_byte()),
            ADD => {
                args.push(self.stack_pop());
                args.push(self.stack_pop());
            }
            _ => unimplemented!(),
        }

        self
    }

    fn execute(&mut self, args: DataSlice) -> &mut Self {
        let opcode = self.get_register(IR) as u8;
        debug!("execute {:?}", self);

        match opcode {
            PUSH => {
                self.stack_push(args[0]);
            }
            ADD => {
                let value = Wrapping(args[0]) + Wrapping(args[1]);
                self.stack_push(value.0);
            }
            _ => unimplemented!(),
        }

        self
    }

    fn process_events(&mut self) {
        unimplemented!()
    }

    fn next_code_byte(&mut self) -> u8 {
        let code_offset = self.get_register(PC);
        let value = self.memory.get(code_offset);
        self.increment_register(PC);
        value
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

    fn decrement_register(&mut self, id: u8) {
        self.registers[id as usize] -= 1;
    }

    fn stack_pop(&mut self) -> u8 {
        let sp = self.get_register(SP);
        let value = self.memory.get(sp);
        self.increment_register(SP);
        value
    }

    fn stack_push(&mut self, value: u8) {
        self.decrement_register(SP);
        let sp = self.get_register(SP);
        self.memory.put(sp, value);
    }
}

impl<R: Read, W: Write> fmt::Debug for VM<R, W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "PC={} SP={} IR={} EP={} stack=[{}] code=[{}]",
               to_hex!(self.get_register(PC)),
               to_hex!(self.get_register(SP)),
               to_hex!(self.get_register(IR)),
               to_hex!(self.get_register(EP)),
               data_to_hex(self.stack()),
               data_to_hex(self.code()))
    }
}

#[cfg(test)]
mod tests {
    use byteorder::ByteOrder;
    use env_logger;
    use self::events::*;
    use std::io::{BufReader, BufWriter};
    use super::*;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn simple() {
        env_logger::init().unwrap();

        {
            let executable = vec![0, 0];
            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0, 0,
                PUSH, 0x55];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x55], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0, 0,
                PUSH, 0x02,                    // b
                PUSH, 0x03,                    // a
                ADD];                          // pop 2 bytes,
                                               // add (a + b) and push

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x05], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0, 0,
                PUSH, 0xff,                    // b
                PUSH, 1,                       // a
                ADD];                          // a + b

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0, 0,
                PUSH, 0,                       // b
                PUSH, 0x0a,                    // a
                DEC,                           // a--
                INT, OUTPUT,
                JNE, 6, 0];                    // a != b

            let (output, vm) = run(&[], executable, 0);

            assert_eq!([0], vm.stack());
            assert!(vm.data().is_empty());
            assert_eq!(&[9, 8, 7, 6, 5, 4, 3, 2, 1, 0], output.as_slice());
        }

        {
            let executable = vec![
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

            let (output, vm) = run(&[], executable, 4);

            assert_eq!(&[0], vm.stack());
            assert_eq!(&[3, 2, 1, 0], vm.data());
            assert_eq!(&[3, 2, 1], output.as_slice());
        }

        {
            let executable = vec![
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

            let input = [3, 2, 1, 0];
            let (output, vm) = run(&input, executable, 0);

            assert_eq!(&[0, 0], vm.stack());
            assert!(vm.data().is_empty());
            assert_eq!(&[3, 2, 1, 0], output.as_slice());
        }

        {
            let executable = vec![
                0, 0,
                PUSH, 4,                       // b
                PUSH, 12,                      // a
                DIV];                          // a / b

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[3], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0, 0,
                PUSH, 0,                       // b
                PUSH, 1,                       // a
                DIV];                          // a / b

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert_eq!(b"Unknown Error", output.as_slice());
        }

        {
            let executable = vec![
                0, 0,
                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0, 0,
                LOAD, PTR, 255, 255];          // access violation

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0, 0,
                LOAD, PTR, 2, 0];              // try to load code segment
                                               // as data segment

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0, 0,
                LOAD, PTR, 6, 0,
                123];

            let (output, vm) = run(&[], executable, 1);

            assert_eq!(&[123], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
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

            let input = [3, 2, 1, 0];
            let (output, vm) = run(&input, executable, 0);

            assert_eq!(&[0, 0], vm.stack());
            assert!(vm.data().is_empty());
            assert_eq!(&[6, 4, 2, 0], output.as_slice());
        }

        {
            let executable = vec![
                0, 0,
                PUSH, 0,                       // i
                                               // loop:
                LOAD, PTR, 21, 0,              // x
                JLE, 21, 0,                    // if x <= i goto: end
                DEC,
                STORE, PTR, 21, 0,             // x
                POP,
                INC,
                JMP, 4, 0,                     // goto loop
                                               // end:
                5];                            // x

            let (output, vm) = run(&[], executable, 1);

            assert_eq!(&[3], vm.stack());
            assert_eq!(&[2], vm.data());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0, 0,
                PUSH, 0,                       // i
                PUSH, 5,                       // x
                                               // loop:
                DEC,
                SWP,
                INC,
                SWP,
                JLE, 16, 0,                    // if x <= i: goto end
                JMP, 6, 0];                    // goto loop

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[2, 3], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }
    }

    fn run(input: DataSlice,
           mut executable: Data,
           data_size: Word)
           -> (Data, VM<BufReader<DataSlice>, BufWriter<Data>>) {
        let code_size = executable.len() as Word - CODE_OFFSET - data_size;
        Endian::write_u16(&mut executable, code_size);

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
