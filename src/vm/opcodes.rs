pub const NOP: u8 = 0x00;

pub const ADD: u8 = 0x01;
pub const SUB: u8 = 0x02;
pub const MUL: u8 = 0x03;
pub const DIV: u8 = 0x04;
pub const MOD: u8 = 0x05;
pub const INC: u8 = 0x06;
pub const DEC: u8 = 0x07;

pub const MOV: u8 = 0x10;
pub const SWP: u8 = 0x11;
pub const PUSH: u8 = 0x12;
pub const POP: u8 = 0x13;

// http://unixwiz.net/techtips/x86-jumps.html
pub const JMP: u8 = 0x20;
pub const JE: u8 = 0x21;
pub const JZ: u8 = 0x22;
pub const JNE: u8 = 0x23;
pub const JNZ: u8 = 0x24;

pub const SHL: u8 = 0x30;
pub const SHR: u8 = 0x31;

pub const AND: u8 = 0x40;
pub const OR: u8 = 0x41;
pub const NOT: u8 = 0x42;
pub const XOR: u8 = 0x43;
pub const TEST: u8 = 0x44;
pub const CMP: u8 = 0x45;

pub const CALL: u8 = 0x51;
pub const RET: u8 = 0x52;

pub const INT: u8 = 0x60;
