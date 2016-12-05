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

use self::events::*;
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

        let stack_end = self.memory.stack_end;
        self.set_register(SP, stack_end);

        let event_queue_end = self.memory.event_queue_end;
        self.set_register(EP, event_queue_end);
        self.set_register(EE, event_queue_end);

        let mut args = vec![];
        while !self.is_terminated() {
            self.fetch()
                .decode(&mut args)
                .execute(&args)
                .process_events();
            args.clear();
        }
    }

    pub fn code(&self) -> DataSlice {
        self.memory.code()
    }

    pub fn data(&self) -> DataSlice {
        self.memory.data()
    }

    pub fn stack(&self) -> DataSlice {
        let sp = self.get_register(SP);
        self.memory.stack(sp)
    }

    pub fn event_queue(&self) -> DataSlice {
        let ep = self.get_register(EP);
        let ee = self.get_register(EE);
        self.memory.event_queue(ep, ee)
    }

    fn terminate(&mut self) {
        debug!("terminate {:?}", self);
        let code_end = self.memory.code_end;
        self.set_register(PC, code_end);
    }

    fn is_terminated(&self) -> bool {
        let pc = self.get_register(PC);
        !self.memory.is_in_code(pc)
    }

    fn fetch(&mut self) -> &mut Self {
        debug!("fetch {:?}", self);

        let opcode = self.next_code_byte() as Word;
        self.set_register(IR, opcode);
        self
    }

    fn decode(&mut self, args: &mut Data) -> &mut Self {
        debug!("decode {:?}", self);

        let opcode = self.get_register(IR) as u8;
        match opcode {
            PUSH => args.push(self.next_code_byte()),
            POP => (),
            INT => {
                let event = self.next_code_byte();
                let argument = if self.stack().is_empty() {
                    0x00
                } else {
                    self.stack_top()
                };
                args.push(argument);
                args.push(event);
            }
            ADD | SUB | MUL | DIV | MOD => {
                args.push(self.stack_pop());
                args.push(self.stack_pop());
            }
            INC | DEC => args.push(self.stack_pop()),
            _ => unimplemented!(),
        }

        self
    }

    fn execute(&mut self, args: DataSlice) -> &mut Self {
        debug!("execute {:?}", self);

        let opcode = self.get_register(IR) as u8;
        match opcode {
            PUSH => {
                self.stack_push(args[0]);
            }
            POP => {
                let _ = self.stack_pop();
            }
            ADD => {
                let value = Wrapping(args[0]) + Wrapping(args[1]);
                self.stack_push(value.0);
            }
            SUB => {
                let value = Wrapping(args[0]) - Wrapping(args[1]);
                self.stack_push(value.0);
            }
            MUL => {
                let value = Wrapping(args[0]) * Wrapping(args[1]);
                self.stack_push(value.0);
            }
            DIV => {
                // FIXME: check div by zero
                let value = Wrapping(args[0]) / Wrapping(args[1]);
                self.stack_push(value.0);
            }
            MOD => {
                // FIXME: check mod by zero
                let value = Wrapping(args[0]) % Wrapping(args[1]);
                self.stack_push(value.0);
            }
            INT => {
                let event = args[0];
                let argument = args[1];
                if events::is_critical(event) {
                    self.process_event(event, argument);
                } else {
                    self.event_queue_push(event, argument);
                }
            }
            _ => unimplemented!(),
        }

        self
    }

    fn process_event(&mut self, event: u8, argument: u8) {
        let handler = self.memory.get_event_handler(event);
        if handler == 0 {
            if events::is_critical(event) {
                self.terminate();
            }
        } else {
            let pc = self.get_register(PC);
            self.stack_push_word(pc);
            self.stack_push(argument);
            self.set_register(PC, handler);
        }
    }

    fn process_events(&mut self) {
        if !(self.is_terminated() || self.event_queue().is_empty()) {
            let (event, argument) = self.event_queue_pop();
            self.process_event(event, argument);
        }
    }

    fn event_queue_push(&mut self, event: u8, argument: u8) {
        self.decrement_register(EP);
        let ep = self.get_register(EP);
        self.memory.put(ep, argument);

        self.decrement_register(EP);
        let ep = self.get_register(EP);
        self.memory.put(ep, event);
    }

    fn event_queue_pop(&mut self) -> (u8, u8) {
        let ee = self.get_register(EE);
        let ep = self.get_register(EP);
        assert_lt!(ep, ee);

        self.decrement_register(EE);
        let ee = self.get_register(EE);
        let argument = self.memory.get(ee);

        self.decrement_register(EE);
        let ee = self.get_register(EE);
        let event = self.memory.get(ee);

        if ep == ee {
            let event_queue_end = self.memory.event_queue_end;
            self.set_register(EP, event_queue_end);
            self.set_register(EE, event_queue_end);
        }

        (event, argument)
    }

    fn next_code_byte(&mut self) -> u8 {
        let code_begin = self.get_register(PC);
        let value = self.memory.get(code_begin);
        self.increment_register(PC);
        value
    }

    fn get_register(&self, id: u8) -> Word {
        self.registers[id as usize]
    }

    fn set_register(&mut self, id: u8, value: Word) {
        debug!("set r{:x} := {}", id, to_hex!(value));
        self.registers[id as usize] = value;
    }

    fn increment_register(&mut self, id: u8) {
        self.registers[id as usize] += 1;
    }

    fn decrement_register(&mut self, id: u8) {
        self.registers[id as usize] -= 1;
    }

    fn stack_top(&self) -> u8 {
        let sp = self.get_register(SP);
        self.memory.get(sp)
    }

    fn stack_pop(&mut self) -> u8 {
        debug!("stack_pop from [{}]", data_to_hex(self.stack()));
        let value = self.stack_top();
        self.increment_register(SP);
        value
    }

    fn stack_push(&mut self, value: u8) {
        debug!("stack_push to [{}]", data_to_hex(self.stack()));
        self.decrement_register(SP);
        let sp = self.get_register(SP);
        self.memory.put(sp, value);
    }

    fn stack_push_word(&mut self, value: Word) {
        debug!("stack_push_word to [{}]", data_to_hex(self.stack()));
        unimplemented!()
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
            let executable = vec![0x00, 0x00];
            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,
                NOP];
            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x03,
                PUSH, 0x04,
                SWP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x03, 0x04], vm.stack());
            assert!(vm.event_queue().is_empty());
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

            assert!(vm.data().is_empty());
            assert_eq!([0], vm.stack());
            assert!(vm.event_queue().is_empty());
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

            assert_eq!(&[0x03, 0x02, 0x01, 0x00], vm.data());
            assert_eq!(&[0x00], vm.stack());
            assert!(vm.event_queue().is_empty());
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

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00, 0x00], vm.stack());
            assert!(vm.event_queue().is_empty());
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

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00, 0x00], vm.stack());
            assert!(vm.event_queue().is_empty());
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

            assert_eq!(&[0x02], vm.data());
            assert_eq!(&[0x03], vm.stack());
            assert!(vm.event_queue().is_empty());
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

            assert!(vm.data().is_empty());
            assert_eq!(&[0x02, 0x03], vm.stack());
            assert!(vm.event_queue().is_empty());
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

            assert!(vm.data().is_empty());
            assert_eq!(&[0x55], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                PUSH, 0x77];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x77, 0x55], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
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

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x55,
                PUSH, 0x77,
                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x55], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                POP];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
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

            assert!(vm.data().is_empty());
            assert_eq!(STACK_SIZE, vm.stack().len() as Word);
            assert!(vm.event_queue().is_empty());
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

            assert!(vm.data().is_empty());
            assert_eq!(STACK_SIZE, vm.stack().len() as Word);
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }
    }

    #[test]
    fn event_queue() {
        let executable = vec![0x00, 0x00];

        let (output, mut vm) = run(&[], executable, 0);
        let event_queue_end = vm.memory.event_queue_end;

        assert_eq!(event_queue_end, vm.get_register(EP));
        assert_eq!(event_queue_end, vm.get_register(EE));

        vm.event_queue_push(CLOCK, 0x05);
        vm.event_queue_push(OUTPUT, 0x06);

        assert_eq!(&[OUTPUT, 0x06, CLOCK, 0x05], vm.event_queue());

        assert_lt!(vm.get_register(EP), vm.get_register(EE));
        assert_eq!(event_queue_end, vm.get_register(EE));

        let (event, argument) = vm.event_queue_pop();
        assert_eq!(CLOCK, event);
        assert_eq!(0x05, argument);
        assert_eq!(&[OUTPUT, 0x06], vm.event_queue());

        assert_lt!(vm.get_register(EP), vm.get_register(EE));
        assert_gt!(event_queue_end, vm.get_register(EE));

        let (event, argument) = vm.event_queue_pop();
        assert_eq!(OUTPUT, event);
        assert_eq!(0x06, argument);
        assert!(vm.event_queue().is_empty());

        assert_eq!(event_queue_end, vm.get_register(EP));
        assert_eq!(event_queue_end, vm.get_register(EE));

        vm.event_queue_push(CLOCK, 0x07);
        let _ = vm.event_queue_pop();

        assert!(vm.data().is_empty());
        assert!(vm.stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert!(output.is_empty());
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

            assert!(vm.data().is_empty());
            assert_eq!(&[0x7b], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                LOAD, PTR, 0xff, 0xff];        // access violation

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Segfault", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                LOAD, PTR, 0x02, 0x00];        // try to load code segment
                                               // as data segment

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
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

            assert!(vm.data().is_empty());
            assert_eq!(&[0x05], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0xff,
                PUSH, 0x01,
                ADD];                          // overflow

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x02,
                PUSH, 0x03,
                SUB];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x01], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x03,
                PUSH, 0x02,
                SUB];                          // overflow

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0xff], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x03,
                PUSH, 0x02,
                MUL];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x06], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x99,
                PUSH, 0x66,
                MUL];                          // overflow

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0xf6], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x04,
                PUSH, 0x0c,
                DIV];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x03], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x01,
                DIV];                          // div by zero

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Unknown Error", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x04,
                PUSH, 0x37,
                MOD];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x03], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x37,
                MOD];                          // mod by zero

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Unknown Error", output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x05,
                INC];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x06], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0xff,
                INC];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x00], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x05,
                DEC];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x04], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                DEC];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0xff], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn logical() {
        // TODO
        assert!(false)
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn bitwise() {
        // TODO
        assert!(false)
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn jumps() {
        // TODO
        assert!(false)
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn functions() {
        // TODO
        assert!(false)
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    #[test]
    fn events() {
        // TODO: set event handler

        {
            let executable = vec![
                0x00, 0x00,

                INT, TERMINATE];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                INT, INPUT];

            let input = [0x11];
            let (output, vm) = run(&input, executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x11], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert!(output.is_empty());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x11,
                INT, OUTPUT];

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert_eq!(&[0x11], vm.stack());
            assert!(vm.event_queue().is_empty());
            assert_eq!(&[0x11], output.as_slice());
        }

        {
            let executable = vec![
                0x00, 0x00,

                PUSH, 0x00,
                PUSH, 0x01,
                DIV];                          // div by zero

            let (output, vm) = run(&[], executable, 0);

            assert!(vm.data().is_empty());
            assert!(vm.stack().is_empty());
            assert!(vm.event_queue().is_empty());
            assert_eq!(b"Unknown Error", output.as_slice());
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
