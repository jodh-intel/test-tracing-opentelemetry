use anyhow::{anyhow, Result};
use slog::{o, Logger};
use std::env;
use std::process::exit;
use std::{thread, time::Duration};
use tracing::{info_span, instrument, span};

mod tracer;

#[instrument]
fn func3() -> Result<()> {
    println!("INFO: func3");

    Ok(())
}

#[instrument]
fn func2() -> Result<()> {
    println!("INFO: func2");

    func3()
}

#[instrument]
fn func1() -> Result<()> {
    println!("INFO: func1");

    func2()
}

#[instrument]
fn do_something() -> Result<()> {
    println!("INFO: do_something");

    func1()
}

fn do_work() -> Result<()> {
    println!("INFO: do_work");

    do_something()
}

fn real_main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let program_name = &args[0];

    if args.len() < 2 {
        println!("ERROR: {}: specify <name>", program_name);
        exit(1);
    }

    let drain = slog::Discard;
    let logger = Logger::root(drain, o!("subsystem" => "foo", "wibble" => "bar"));

    tracer::setup_tracing(&logger)?;

    // Create the application root span.
    let root_span = info_span!("root");
    let _enter = root_span.enter();

    do_work()
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("ERROR: {}", e);
        exit(1);
    };
}
