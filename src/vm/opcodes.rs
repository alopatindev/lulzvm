pub const NOP: u8 = 0x00;

pub const ADD: u8 = 0x01;
pub const SUB: u8 = 0x02;
pub const MUL: u8 = 0x03;
pub const DIV: u8 = 0x04;
pub const MOD: u8 = 0x05;
pub const INC: u8 = 0x06;
pub const DEC: u8 = 0x07;

pub const AND: u8 = 0x10;
pub const OR: u8 = 0x11;
pub const NOT: u8 = 0x12;
pub const SHL: u8 = 0x13;
pub const SHR: u8 = 0x14;
pub const XOR: u8 = 0x15;

pub const PUSH: u8 = 0x20;
pub const POP: u8 = 0x21;
pub const SWP: u8 = 0x22;
pub const STORE: u8 = 0x23;  // stack -> data
pub const LOAD: u8 = 0x24;   // data -> stack

pub const JMP: u8 = 0x30;
pub const JE: u8 = 0x31;     // ==
pub const JNE: u8 = 0x32;    // !=
pub const JL: u8 = 0x33;     // <
pub const JG: u8 = 0x34;     // >
pub const JLE: u8 = 0x35;    // <=
pub const JGE: u8 = 0x36;    // >=

pub const CALL: u8 = 0x41;
pub const RET: u8 = 0x42;
pub const EMIT: u8 = 0x43;
