use config::*;
use env_logger;
use std::io::{BufReader, BufWriter};
use vm::VM;
use vm::memory::Memory;

macro_rules! to_hex {
   ($data:expr, Word) => {
       format!("0x{:04x}", $data)
   };
   ($data:expr) => {
       format!("0x{:02x}", $data)
   };
}

pub fn data_to_hex(data: DataSlice) -> String {
    data.iter()
        .map(|i| to_hex!(i))
        .collect::<Vec<String>>()
        .join(" ")
}

pub fn test_run(input: DataSlice,
                mut executable: Data,
                data_size: Word)
                -> (Data, VM<BufReader<DataSlice>, BufWriter<Data>>) {
    let _ = env_logger::init();

    let code_size = executable.len() as Word - CODE_OFFSET - data_size;
    Memory::write_word(&mut executable, 0, code_size);

    let input = BufReader::new(input);

    let output: Data = vec![];
    let output = BufWriter::new(output);

    let mut vm = VM::new(input, output, executable);
    vm.run();

    let output = vm.get_output_ref()
        .get_ref()
        .iter()
        .map(|x| *x)
        .collect::<Data>();

    (output, vm)
}
