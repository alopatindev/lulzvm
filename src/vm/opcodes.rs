pub const NOP: u8 = 0x00;

pub const ADD: u8 = 0x01;
pub const SUB: u8 = 0x02;
pub const MUL: u8 = 0x03;
pub const DIV: u8 = 0x04;
pub const INC: u8 = 0x05;
pub const DEC: u8 = 0x06;

pub const MOV: u8 = 0x10;
pub const PUSH: u8 = 0x11;
pub const POP: u8 = 0x12;

// http://unixwiz.net/techtips/x86-jumps.html
pub const JMP: u8 = 0x20;
pub const JE: u8 = 0x21;
pub const JZ: u8 = 0x22;
pub const JNE: u8 = 0x23;
pub const JNZ: u8 = 0x24;
pub const JL: u8 = 0x25;
pub const JLE: u8 = 0x26;
pub const JG: u8 = 0x27;
pub const JGE: u8 = 0x28;

pub const CALL: u8 = 0x30;
pub const RET: u8 = 0x31;

pub const READ: u8 = 0x40;
pub const WRITE: u8 = 0x41;
