use config::*;
use std::fmt;
use std::io::{Read, Write};
use std::num::Wrapping;
use stopwatch::Stopwatch;
use utils;

#[cfg(test)]
pub mod tests;

pub mod events;
pub mod memory;
pub mod opcodes;
pub mod registers;

use self::events::*;
use self::memory::*;
use self::opcodes::*;
use self::registers::*;

pub struct VM<R: Read, W: Write> {
    input: R,
    output: W,

    registers: Registers,
    memory: Memory,

    terminated: bool,
    waiting: bool,

    clock: Stopwatch,
    clock_step: u8,
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
            waiting: false,

            clock: Stopwatch::new(),
            clock_step: 0,
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

        self.clock.start();

        while !self.terminated {
            let pc = self.get_register(PC);
            if !self.memory.is_in_code(pc) && self.event_queue().is_empty() {
                self.terminate();
            } else {
                if !self.waiting {
                    let mut args = vec![];
                    self.fetch()
                        .decode(&mut args)
                        .execute(&args);
                    args.clear();
                }

                self.process_events()
                    .update_clock();
            }
        }

        self.clock.stop();
        self.output.flush().unwrap();
    }

    pub fn get_output_ref(&self) -> &W {
        &self.output
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
            POP | NOP | WAIT => (),
            LOAD => {
                args.push(self.next_code_byte());
                args.push(self.next_code_byte());
            }
            LOAD_OFFS => {
                if !self.locals_stack().is_empty() {
                    args.push(self.next_code_byte());
                    args.push(self.next_code_byte());

                    let offset = self.locals_stack_top();
                    args.push(offset);
                }
            }
            STORE => {
                if !self.locals_stack().is_empty() {
                    args.push(self.next_code_byte());
                    args.push(self.next_code_byte());

                    let data = self.locals_stack_top();
                    args.push(data);
                }
            }
            STORE_OFFS => {
                if self.locals_stack().len() >= 2 {
                    args.push(self.next_code_byte());
                    args.push(self.next_code_byte());

                    let data = self.locals_stack()[1];
                    let offset = self.locals_stack()[0];
                    args.push(data);
                    args.push(offset);
                }
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
            SUBSCRIBE => {
                args.push(self.next_code_byte());
                args.push(self.next_code_byte());
                args.push(self.next_code_byte());
            }
            UNSUBSCRIBE => args.push(self.next_code_byte()),
            _ => unimplemented!(),
        }

        self
    }

    fn execute(&mut self, args: DataSlice) {
        debug!("execute {:?}", self);

        let opcode = self.get_register(IR) as u8;
        let need_args = ![NOP, POP, WAIT].contains(&opcode);

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
                    let offset = 0;
                    let data = self.extract_data_ptr(args, offset)
                        .map(|ptr| self.memory.get(ptr));
                    match data {
                        Some(data) => self.locals_stack_push(data),
                        None => self.terminate_with_segfault(),
                    }
                }
                LOAD_OFFS => {
                    let offset = args[2];
                    let data = self.extract_data_ptr(args, offset)
                        .map(|ptr| self.memory.get(ptr));
                    match data {
                        Some(data) => self.locals_stack_push(data),
                        None => self.terminate_with_segfault(),
                    }
                }
                STORE => {
                    let data = args[2];
                    let offset = 0;
                    match self.extract_data_ptr(args, offset) {
                        Some(ptr) => self.memory.put(ptr, data),
                        None => self.terminate_with_segfault(),
                    }
                }
                STORE_OFFS => {
                    let data = args[2];
                    let offset = args[3];
                    match self.extract_data_ptr(args, offset) {
                        Some(ptr) => self.memory.put(ptr, data),
                        None => self.terminate_with_segfault(),
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
                WAIT => self.waiting = true,
                SUBSCRIBE => {
                    let event = args[0];
                    let handler_address = Memory::read_word(&args, 1);
                    self.memory.set_event_handler(event, handler_address);
                }
                UNSUBSCRIBE => {
                    let event = args[0];
                    self.memory.set_event_handler(event, 0x0000);
                }
                _ => unimplemented!(),
            }
        }
    }

    fn extract_data_ptr(&self, args: DataSlice, offset: u8) -> Option<Word> {
        let ptr = Memory::read_word(&args, 0) + offset as Word;
        if self.memory.is_in_data(ptr) {
            Some(ptr)
        } else {
            None
        }
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
        if handler == 0x0000 {
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
            let pc = self.get_register(PC);
            self.return_stack_push(pc);

            self.locals_stack_push(argument); // default is zero
            self.set_register(PC, handler);
        }
    }

    fn process_events(&mut self) -> &mut Self {
        let nothing_to_process = self.terminated || self.event_queue().is_empty();

        if !nothing_to_process {
            if self.waiting {
                self.waiting = false;
            }

            let (event, argument) = self.event_queue_pop();
            self.process_event(event, argument);
        }

        self
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

    fn update_clock(&mut self) {
        if self.clock.elapsed_ms() > CLOCK_TIMEOUT_MS {
            self.clock.restart();

            let clock_step = self.clock_step;
            self.event_queue_push(CLOCK, clock_step);

            let new_clock_step = Wrapping(clock_step) + Wrapping(1);
            self.clock_step = new_clock_step.0;
        }
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
               utils::data_to_hex(self.locals_stack()));
        self.decrement_register(SP);
        let sp = self.get_register(SP);
        self.memory.put(sp, value);
    }

    fn locals_stack_pop(&mut self) -> u8 {
        debug!("locals_stack_pop {} from [{}]",
               to_hex!(self.locals_stack_top()),
               utils::data_to_hex(self.locals_stack()));
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
               utils::data_to_hex(self.return_stack()));

        self.decrement_register_by(RP, WORD_SIZE);
        let rp = self.get_register(RP);
        self.memory.put_word(rp, address);
    }

    fn return_stack_pop(&mut self) -> Word {
        debug!("return_stack_pop {} from [{}]",
               to_hex!(self.return_stack_top(), Word),
               utils::data_to_hex(self.return_stack()));

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
               "PC={} IR={} SP={} RP={} EP={} EE={} waiting={} \
               locals_stack=[{}] return_stack=[{}] code=[{}]",
               to_hex!(self.get_register(PC), Word),
               to_hex!(self.get_register(IR), Word),
               to_hex!(self.get_register(SP), Word),
               to_hex!(self.get_register(RP), Word),
               to_hex!(self.get_register(EP), Word),
               to_hex!(self.get_register(EE), Word),
               self.waiting,
               utils::data_to_hex(self.locals_stack()),
               utils::data_to_hex(self.return_stack()),
               utils::data_to_hex(self.code()))
    }
}
