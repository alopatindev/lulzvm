use common::*;
use env_logger;
use std::fmt;
use std::io::{Read, Write, Error, ErrorKind, Result};
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

        self.set_register(IR, NOP as Word);

        let locals_stack_end = self.memory.locals_stack_end;
        self.set_register(SP, locals_stack_end);

        let return_stack_end = self.memory.return_stack_end;
        self.set_register(RP, return_stack_end);

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

    pub fn locals_stack(&self) -> DataSlice {
        let sp = self.get_register(SP);
        self.memory.locals_stack(sp)
    }

    pub fn return_stack(&self) -> DataSlice {
        let rp = self.get_register(RP);
        self.memory.return_stack(rp)
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

    fn terminate_with_segfault(&mut self) {
        self.process_event(SEGFAULT, 0x00);
    }

    fn fetch(&mut self) -> &mut Self {
        let opcode = self.next_code_byte() as Word;
        self.set_register(IR, opcode);
        self
    }

    fn decode(&mut self, args: &mut Data) -> &mut Self {
        let opcode = self.get_register(IR) as u8;
        match opcode {
            ADD | SUB | MUL | DIV | MOD | SWP | AND | OR | XOR => {
                if self.locals_stack().len() >= 2 {
                    args.push(self.locals_stack_pop());
                    args.push(self.locals_stack_pop());
                }
            }
            INC | DEC | NOT => {
                if !self.locals_stack().is_empty() {
                    args.push(self.locals_stack_pop());
                }
            }
            SHL | SHR => {
                if !self.locals_stack().is_empty() {
                    args.push(self.locals_stack_pop());
                    args.push(self.next_code_byte());
                }
            }
            PUSH => args.push(self.next_code_byte()),
            POP | NOP => (),
            LOAD | STORE => {
                args.push(self.next_code_byte());
                args.push(self.next_code_byte());
                args.push(self.next_code_byte());
            }
            RET => {
                if self.return_stack().len() >= 2 {
                    args.push(self.return_stack()[0]);
                    args.push(self.return_stack()[1]);
                    let _ = self.return_stack_pop();
                }
            }
            JMP | CALL => {
                args.push(self.next_code_byte());
                args.push(self.next_code_byte());
            }
            JE | JNE | JL | JG | JLE | JGE => {
                if self.locals_stack().len() >= 2 {
                    args.push(self.locals_stack()[0]);
                    args.push(self.locals_stack()[1]);
                }
                args.push(self.next_code_byte());
                args.push(self.next_code_byte());
            }
            EMIT => {
                let event = self.next_code_byte();
                let argument = if self.locals_stack().is_empty() {
                    0x00
                } else {
                    self.locals_stack_top()
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
        let need_args = ![NOP, POP].contains(&opcode);

        if need_args && args.is_empty() {
            self.terminate_with_segfault();
        } else {
            match opcode {
                NOP => (),
                ADD => self.apply_bin_operator(args, |x, y| x + y),
                SUB => self.apply_bin_operator(args, |x, y| x - y),
                MUL => self.apply_bin_operator(args, |x, y| x * y),
                DIV => {
                    if args[1] == 0 {
                        self.process_event(UNKNOWN_ERROR, 0x00);
                    } else {
                        self.apply_bin_operator(args, |x, y| x / y);
                    }
                }
                MOD => {
                    if args[1] == 0 {
                        self.process_event(UNKNOWN_ERROR, 0x00);
                    } else {
                        self.apply_bin_operator(args, |x, y| x % y);
                    }
                }
                INC => {
                    let value = Wrapping(args[0]) + Wrapping(1);
                    self.locals_stack_push(value.0);
                }
                DEC => {
                    let value = Wrapping(args[0]) - Wrapping(1);
                    self.locals_stack_push(value.0);
                }
                SHL => self.apply_bin_operator(args, |x, y| x << y.0 as usize),
                SHR => self.apply_bin_operator(args, |x, y| x >> y.0 as usize),
                XOR => self.apply_bin_operator(args, |x, y| x ^ y),
                AND => self.apply_bin_operator(args, |x, y| x & y),
                OR => self.apply_bin_operator(args, |x, y| x | y),
                NOT => {
                    let value = args[0] == 0;
                    self.locals_stack_push(value as u8);
                }
                PUSH => {
                    if self.get_register(SP) <= self.memory.locals_stack_begin {
                        self.terminate_with_segfault();
                    } else {
                        self.locals_stack_push(args[0]);
                    }
                }
                POP => {
                    if self.locals_stack().is_empty() {
                        self.terminate_with_segfault();
                    } else {
                        let _ = self.locals_stack_pop();
                    }
                }
                SWP => {
                    self.locals_stack_push(args[0]);
                    self.locals_stack_push(args[1]);
                }
                LOAD => {
                    match self.load_data(args) {
                        Some(value) => self.locals_stack_push(value),
                        None => self.terminate_with_segfault(),
                    }
                }
                STORE => {
                    if self.store_data(args).is_err() {
                        self.terminate_with_segfault();
                    }
                }
                JMP => self.jump(args),
                JE => self.jump_if(args, |x, y| x == y),
                JNE => self.jump_if(args, |x, y| x != y),
                JL => self.jump_if(args, |x, y| x < y),
                JG => self.jump_if(args, |x, y| x > y),
                JLE => self.jump_if(args, |x, y| x <= y),
                JGE => self.jump_if(args, |x, y| x >= y),
                CALL => {
                    if self.get_register(RP) <= self.memory.return_stack_begin {
                        self.terminate_with_segfault();
                    } else {
                        let pc = self.get_register(PC);
                        self.return_stack_push(pc);
                        self.jump(args);
                    }
                }
                RET => self.jump(args),
                EMIT => {
                    // compiler won't let args be empty
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
        }

        self
    }

    fn load_data(&self, args: DataSlice) -> Option<u8> {
        self.extract_data_ptr(args)
            .map(|ptr| self.memory.get(ptr))
    }

    fn store_data(&mut self, args: DataSlice) -> Result<()> {
        let e = Error::new(ErrorKind::InvalidInput, "");

        match self.extract_ptr(args) {
            Some((ptr, with_offset)) => {
                let value = if with_offset {
                    if self.locals_stack().len() < 2 {
                        return Err(e);
                    } else {
                        self.locals_stack()[1]
                    }
                } else {
                    self.locals_stack()[0]
                };

                self.memory.put(ptr, value);
                Ok(())
            }
            None => Err(e),
        }
    }

    fn extract_data_ptr(&self, args: DataSlice) -> Option<Word> {
        self.extract_ptr(args)
            .and_then(|(ptr, _)| if self.memory.is_in_data(ptr) {
                Some(ptr)
            } else {
                None
            })
    }

    fn extract_ptr(&self, args: DataSlice) -> Option<(Word, bool)> {
        let mode = args[0];
        let ptr = Memory::read_word(&args, 1);

        let mut with_offset = false;

        let updated_ptr = match mode {
            PTR => ptr,
            PTR_WITH_OFFSET => {
                with_offset = true;

                if self.locals_stack().is_empty() {
                    return None;
                } else {
                    let offset = self.locals_stack_top() as Word;
                    ptr + offset
                }
            }
            _ => unreachable!(),
        };

        Some((updated_ptr, with_offset))
    }

    fn apply_bin_operator<F>(&mut self, args: DataSlice, op: F)
        where F: Fn(Wrapping<u8>, Wrapping<u8>) -> Wrapping<u8>
    {
        let value = op(Wrapping(args[0]), Wrapping(args[1]));
        self.locals_stack_push(value.0);
    }

    fn jump_if<F>(&mut self, args: DataSlice, condition: F)
        where F: Fn(u8, u8) -> bool
    {
        if args.len() < 4 {
            self.terminate_with_segfault();
        } else {
            if condition(args[0], args[1]) {
                self.jump(&args[2..])
            }
        }
    }

    fn jump(&mut self, args: DataSlice) {
        let new_pc = Memory::read_word(&args, 0);
        self.set_register(PC, new_pc);
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
                    self.locals_stack_push(buffer[0]);
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
            // FIXME
            // let pc = self.get_register(PC);
            // self.return_stack_push(pc);
            self.locals_stack_push(argument);
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
        debug!("set r{:x} = {}", id, to_hex!(value, Word));
        self.registers[id as usize] = value;
    }

    fn increment_register(&mut self, id: u8) {
        self.increment_register_by(id, 1);
    }

    fn decrement_register(&mut self, id: u8) {
        self.decrement_register_by(id, 1);
    }

    fn increment_register_by(&mut self, id: u8, acc: Word) {
        self.registers[id as usize] += acc;
    }

    fn decrement_register_by(&mut self, id: u8, acc: Word) {
        self.registers[id as usize] -= acc;
    }

    fn locals_stack_push(&mut self, value: u8) {
        debug!("locals_stack_push {} to [{}]",
               to_hex!(value),
               data_to_hex(self.locals_stack()));
        self.decrement_register(SP);
        let sp = self.get_register(SP);
        self.memory.put(sp, value);
    }

    fn locals_stack_pop(&mut self) -> u8 {
        debug!("locals_stack_pop {} from [{}]",
               to_hex!(self.locals_stack_top()),
               data_to_hex(self.locals_stack()));
        let value = self.locals_stack_top();
        self.increment_register(SP);
        value
    }

    fn locals_stack_top(&self) -> u8 {
        let sp = self.get_register(SP);
        self.memory.get(sp)
    }

    fn return_stack_push(&mut self, address: Word) {
        debug!("return_stack_push {} to [{}]",
               to_hex!(address, Word),
               data_to_hex(self.return_stack()));

        self.decrement_register_by(RP, WORD_SIZE);
        let rp = self.get_register(RP);
        self.memory.put_word(rp, address);
    }

    fn return_stack_pop(&mut self) -> Word {
        debug!("return_stack_pop {} from [{}]",
               to_hex!(self.return_stack_top(), Word),
               data_to_hex(self.return_stack()));

        let address = self.return_stack_top();
        self.increment_register_by(RP, WORD_SIZE);
        address
    }

    fn return_stack_top(&self) -> Word {
        let rp = self.get_register(RP);
        self.memory.get_word(rp)
    }
}

impl<R: Read, W: Write> fmt::Debug for VM<R, W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "PC={} IR={} SP={} RP={} EP={} EE={} locals_stack=[{}] return_stack=[{}] code=[{}]",
               to_hex!(self.get_register(PC), Word),
               to_hex!(self.get_register(IR), Word),
               to_hex!(self.get_register(SP), Word),
               to_hex!(self.get_register(RP), Word),
               to_hex!(self.get_register(EP), Word),
               to_hex!(self.get_register(EE), Word),
               data_to_hex(self.locals_stack()),
               data_to_hex(self.return_stack()),
               data_to_hex(self.code()))
    }
}
