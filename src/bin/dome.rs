use std::env;
use std::process::ExitCode;

use dome::installer::{Command, Environment, run};

fn main() -> ExitCode {
    let command = match Command::parse(env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("dome: {error}");
            eprintln!("usage: dome <add|remove> <binary>");
            return ExitCode::FAILURE;
        }
    };

    match Environment::from_process().and_then(|environment| run(command, &environment)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dome: {error}");
            ExitCode::FAILURE
        }
    }
}
