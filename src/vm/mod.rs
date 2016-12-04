use common::*;
use env_logger;
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
        let _ = env_logger::init();

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
    use self::events::*;
    use std::io::{BufReader, BufWriter};
    use super::*;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn simple() {
        {
            let executable = vec![0, 0];
            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x03,
                PUSH, 0x04,
                SWP];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x03, 0x04], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // b
                PUSH, 0x0a,                    // a
                DEC,                           // a--
                INT, OUTPUT,
                JNE, 0x06, 0];                 // a != b

            let (output, vm) = run(&[], executable, 0);

            assert_eq!([0], vm.stack());
            assert!(vm.data().is_empty());
            assert_eq!(&[9, 8, 7, 6, 5, 4, 3, 2, 1, 0], output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // zero
                PUSH, 0x00,                    // offset

                // label loop
                                               // d = [data segment + offset]
                LOAD, PTR_WITH_OFFSET, 0x15, 0x00,
                INT, OUTPUT,                   // print d
                JE, 0x14, 0x00,                // if d == zero: goto end
                POP,                           // pop d
                INC,                           // offset++
                JMP, 0x06, 0x00,               // goto loop

                // label end
                NOP,                           // optional
                0x03, 0x02, 0x01, 0x00];

            let (output, vm) = run(&[], executable, 4);

            assert_eq!(&[0x00], vm.stack());
            assert_eq!(&[0x03, 0x02, 0x01, 0x00], vm.data());
            assert_eq!(&[0x03, 0x02, 0x01], output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // b

                // label loop
                INT, INPUT,                    // a
                INT, OUTPUT,                   // print a
                JE, 0x0f, 0x00,                // if a == b: goto end
                POP,                           // remove a
                JMP, 0x04, 0x00                // goto loop

                // label end
                // NOP                         // optional
                ];

            let input = [0x03, 0x02, 0x01, 0x00];
            let (output, vm) = run(&input, executable, 0);

            assert_eq!(&[0x00, 0x00], vm.stack());
            assert!(vm.data().is_empty());
            assert_eq!(&[0x03, 0x02, 0x01, 0x00], output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // const zero
                                               // loop:
                INT, INPUT,                    //   x = read
                CALL, PTR, 0x14, 0x00,         //   x = f(x)
                INT, OUTPUT,                   //   print x
                JE, 0x12, 0x00,                // if x == zero: goto exit
                POP,                           // pop x
                JMP, 0x02, 0x00,               // goto loop
                INT, TERMINATE,                // exit:

                                               // f:
                PUSH, 0x02,
                MUL,                           // a = a * 2
                RET];

            let input = [0x03, 0x02, 0x01, 0x00];
            let (output, vm) = run(&input, executable, 0);

            assert_eq!(&[0x00, 0x00], vm.stack());
            assert!(vm.data().is_empty());
            assert_eq!(&[0x06, 0x04, 0x02, 0x00], output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // i
                                               // loop:
                LOAD, PTR, 0x15, 0x00,         // x
                JLE, 0x15, 0x00,               // if x <= i goto: end
                DEC,
                STORE, PTR, 0x15, 0x00,        // x
                POP,
                INC,
                JMP, 0x04, 0x00,               // goto loop
                                               // end:
                0x05];                         // x

            let (output, vm) = run(&[], executable, 1);

            assert_eq!(&[0x03], vm.stack());
            assert_eq!(&[0x02], vm.data());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,                    // i
                PUSH, 0x05,                    // x
                                               // loop:
                DEC,
                SWP,
                INC,
                SWP,
                JLE, 0x10, 0,                  // if x <= i: goto end
                JMP, 0x06, 0];                 // goto loop

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x02, 0x03], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn stack() {
        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x55], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                PUSH, 0x77];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x77, 0x55], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                PUSH, 0x77,
                POP,
                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                PUSH, 0x77,
                POP];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x55], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable_size = WORD_SIZE + STACK_SIZE * 2;
            let executable_size = executable_size as usize;
            let mut executable = Vec::with_capacity(executable_size);
            executable.resize(executable_size, 0x00);

            let mut i = 3;
            while i < executable_size {
                executable[i] = PUSH;
                i += 2;
            }

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(STACK_SIZE, vm.stack().len() as Word);
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable_size = WORD_SIZE + (STACK_SIZE + 1) * 2;
            let executable_size = executable_size as usize;
            let mut executable = Vec::with_capacity(executable_size);
            executable.resize(executable_size, 0x00);

            let mut i = 3;
            while i < executable_size {
                executable[i] = PUSH;
                i += 2;
            }

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(STACK_SIZE, vm.stack().len() as Word);
            assert!(vm.data().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn load_store() {
        {
            let executable = vec![
                0x00, 0x00,

                LOAD, PTR, 0x06, 0x00,
                0x7b];

            let (output, vm) = run(&[], executable, 1);

            assert_eq!(&[0x7b], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                LOAD, PTR, 0xff, 0xff];        // access violation

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                LOAD, PTR, 0x02, 0x00];        // try to load code segment
                                               // as data segment

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn arithmetic() {
        {
            let executable = vec![
                0x00, 0x00,

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
                0x00, 0x00,

                PUSH, 0xff,
                PUSH, 0x01,
                ADD];                          // overflow

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x02,
                PUSH, 0x03,
                SUB];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x01], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x03,
                PUSH, 0x02,
                SUB];                          // overflow

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0xff], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x03,
                PUSH, 0x02,
                MUL];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x06], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x99,
                PUSH, 0x66,
                MUL];                          // overflow

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x33], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x04,
                PUSH, 0x0c,
                DIV];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x03], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x01,
                DIV];                          // div by zero

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert_eq!(b"Unknown Error", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x04,
                PUSH, 0x37,
                MOD];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x03], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x37,
                MOD];                          // mod by zero

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.stack().is_empty());
            assert!(vm.data().is_empty());
            assert_eq!(b"Unknown Error", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x05,
                INC];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x06], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0xff,
                INC];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x00], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x05,
                DEC];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0x04], vm.stack());
            assert!(vm.data().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                DEC];

            let (output, vm) = run(&[], executable, 0);

            assert_eq!(&[0xff], vm.stack());
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
