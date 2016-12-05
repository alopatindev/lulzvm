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

include!("tests.rs");

pub struct VM<R: Read, W: Write> {
    input: R,
    output: W,

    registers: Registers,
    memory: Memory,
    terminated: bool,
}

impl<R: Read, W: Write> VM<R, W> {
    pub fn new(input: R, output: W, executable: Data) -> Self {
        let memory = Memory::from_executable(executable);

        VM {
            input: input,
            output: output,
            registers: [0; REGISTERS as usize],
            memory: memory,
            terminated: false,
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
        while !self.terminated {
            let pc = self.get_register(PC);
            if !self.memory.is_in_code(pc) && self.event_queue().is_empty() {
                self.terminate();
            } else {
                self.fetch()
                    .decode(&mut args)
                    .execute(&args)
                    .process_events();
                args.clear();
            }
        }

        self.output.flush().unwrap();
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
        self.terminated = true;
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
            ADD | SUB | MUL | DIV | MOD | SWP => {
                if self.stack().len() >= 2 {
                    args.push(self.stack_pop());
                    args.push(self.stack_pop());
                }
            }
            INC | DEC => {
                if !self.stack().is_empty() {
                    args.push(self.stack_pop());
                }
            }
            PUSH => args.push(self.next_code_byte()),
            POP | NOP => (),
            EMIT => {
                let event = self.next_code_byte();
                let argument = if self.stack().is_empty() {
                    0x00
                } else {
                    self.stack_top()
                };
                args.push(event);
                args.push(argument);
            }
            _ => unimplemented!(),
        }

        self
    }

    fn execute(&mut self, args: DataSlice) -> &mut Self {
        debug!("execute {:?}", self);

        let opcode = self.get_register(IR) as u8;
        match opcode {
            NOP => (),
            ADD => {
                if args.is_empty() {
                    self.process_event(SEGFAULT, 0x00);
                } else {
                    let value = Wrapping(args[0]) + Wrapping(args[1]);
                    self.stack_push(value.0);
                }
            }
            SUB => {
                if args.is_empty() {
                    self.process_event(SEGFAULT, 0x00);
                } else {
                    let value = Wrapping(args[0]) - Wrapping(args[1]);
                    self.stack_push(value.0);
                }
            }
            MUL => {
                if args.is_empty() {
                    self.process_event(SEGFAULT, 0x00);
                } else {
                    let value = Wrapping(args[0]) * Wrapping(args[1]);
                    self.stack_push(value.0);
                }
            }
            DIV => {
                if args.is_empty() {
                    self.process_event(SEGFAULT, 0x00);
                } else if args[1] == 0 {
                    self.process_event(UNKNOWN_ERROR, 0x00);
                } else {
                    let value = Wrapping(args[0]) / Wrapping(args[1]);
                    self.stack_push(value.0);
                }
            }
            MOD => {
                if args.is_empty() {
                    self.process_event(SEGFAULT, 0x00);
                } else if args[1] == 0 {
                    self.process_event(UNKNOWN_ERROR, 0x00);
                } else {
                    let value = Wrapping(args[0]) % Wrapping(args[1]);
                    self.stack_push(value.0);
                }
            }
            INC => {
                if args.is_empty() {
                    self.process_event(SEGFAULT, 0x00);
                } else {
                    let value = Wrapping(args[0]) + Wrapping(1);
                    self.stack_push(value.0);
                }
            }
            DEC => {
                if args.is_empty() {
                    self.process_event(SEGFAULT, 0x00);
                } else {
                    let value = Wrapping(args[0]) - Wrapping(1);
                    self.stack_push(value.0);
                }
            }
            PUSH => {
                if self.get_register(SP) <= self.memory.stack_begin {
                    self.process_event(SEGFAULT, 0x00);
                } else {
                    self.stack_push(args[0]);
                }
            }
            POP => {
                if self.stack().is_empty() {
                    self.process_event(SEGFAULT, 0x00);
                } else {
                    let _ = self.stack_pop();
                }
            }
            SWP => {
                if args.len() < 2 {
                    self.process_event(SEGFAULT, 0x00);
                } else {
                    self.stack_push(args[0]);
                    self.stack_push(args[1]);
                }
            }
            EMIT => {
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
        debug!("process_event event={} argument={}",
               to_hex!(event),
               to_hex!(argument));

        let handler = self.memory.get_event_handler(event);
        if handler == 0 {
            debug!("handler is NOT set");
            match event {
                INPUT => {
                    let mut buffer = [0; 1];
                    let _ = self.input.read(&mut buffer).unwrap();
                    self.stack_push(buffer[0]);
                }
                OUTPUT => {
                    self.output.write(&[argument]).unwrap();
                }
                SEGFAULT => {
                    self.output.write(b"Segfault").unwrap();
                }
                UNKNOWN_ERROR => {
                    self.output.write(b"Unknown Error").unwrap();
                }
                _ => debug!("no default handler"),
            }

            if events::is_critical(event) {
                self.terminate();
            }
        } else {
            debug!("handler is set");
            let pc = self.get_register(PC);
            self.stack_push_word(pc);
            self.stack_push(argument);
            self.set_register(PC, handler);
        }
    }

    fn process_events(&mut self) {
        let nothing_to_process = self.terminated || self.event_queue().is_empty();
        if !nothing_to_process {
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
        debug!("set r{:x} = {}", id, to_hex!(value));
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
        debug!("stack_pop {} from [{}]",
               to_hex!(self.stack_top()),
               data_to_hex(self.stack()));
        let value = self.stack_top();
        self.increment_register(SP);
        value
    }

    fn stack_push(&mut self, value: u8) {
        debug!("stack_push {} to [{}]",
               to_hex!(value),
               data_to_hex(self.stack()));
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
