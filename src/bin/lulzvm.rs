extern crate ctrlc;

extern crate lulzvm;

#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate clap;

use clap::{ArgGroup, ArgMatches, App};
use lulzvm::vm::VM;
use std::env;
use std::fs::File;
use std::io::{stdin, stdout, Read, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    let matches = App::new("LulzVM")
        .args_from_usage("[FILE] 'Bytecode executable'
                            -d, --debug 'Enable debug messages'")
        .group(ArgGroup::with_name("required")
            .args(&["FILE"])
            .required(true))
        .get_matches();

    match do_checked_main(matches) {
        Ok(_) => (),
        Err(e) => println!("Error: {:?}", e),
    }
}

fn do_checked_main(matches: ArgMatches) -> Result<()> {
    let executable_filename = matches.value_of("FILE").unwrap();

    let mut executable = Vec::new();
    let mut executable_file = try!(File::open(executable_filename));
    let _ = try!(executable_file.read_to_end(&mut executable));

    if matches.is_present("debug") {
        env::set_var("RUST_LOG", "lulzvm::vm=debug,error,info,warn,trace");
        let _ = env_logger::init().unwrap();
    }

    let termination_scheduled = Arc::new(AtomicBool::new(false));
    let r = termination_scheduled.clone();
    ctrlc::set_handler(move || {
        info!("Terminating...");
        r.store(true, Ordering::Relaxed);
    });

    let mut vm = VM::new(stdin(), stdout(), executable, termination_scheduled);
    vm.run();

    Ok(())
}
