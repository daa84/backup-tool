
extern crate toml;

use std::io::prelude::*;
use std::fs::File;
use std::error::Error;
use std::process::Command;

struct Settings {
    commands: Vec<String>
}

const CONFIG_FILE: &'static str = "config.toml";

fn main() {

    // TODO: process errors to email
    match run() {
        Err(e) => println!("Error in backup {}", e.description()),
        Ok(_) => println!("Backup finished")
    }
    
}

fn run() -> Result<(), SimpleError> {
    let settings = try!(load_settings());
    try!(run_commands(&settings.commands));
    Ok(())
}

fn run_commands(commands: &[String]) -> Result<(), SimpleError> {
    for command in commands {
        println!("Execute {} ...", command);
        let status = try!(Command::new(command).status());
        println!("Status {}", status);
    }
    Ok(())
}

fn load_settings() -> Result<Settings, SimpleError> {
    println!("Load config '{}' ...", CONFIG_FILE);

    let mut f = try!(File::open(CONFIG_FILE));

    let mut config_str = String::new();

    try!(f.read_to_string(&mut config_str));

    let mut parser = toml::Parser::new(&config_str);
    let config = try!(parser.parse().ok_or(SimpleError::Str(format!("Can't parse config file, {:?}", parser.errors))));

    let commands = match config.get("run") {
        None => {
            println!("No run section found");
            vec![]
        },
        Some(run) => {
            match run.lookup("commands") {
                None => {
                    println!("No commands found");
                    vec![]
                },
                Some(commands) => {
                    let commands_slice = 
                        try!(commands.as_slice().ok_or(SimpleError::Str("Wrong type of comands section".to_owned())));
                    try!(commands_slice.iter()
                         .map(|v| v.as_str().map(|s| s.to_string()).ok_or(SimpleError::Str("Command must be string".to_owned())))
                         .collect::<Result<Vec<String>, SimpleError>>())
                }
            }
        }
    };

    Ok(Settings {
        commands: commands
    })
}


#[derive(Debug)]
enum SimpleError {
    Str(String),
    IoError(std::io::Error),
}

impl From<std::io::Error> for SimpleError {
    fn from(e: std::io::Error) -> Self {
        SimpleError::IoError(e)
    }
}


impl std::fmt::Display for SimpleError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "Error: {}", self.description())
    }
}

impl Error for SimpleError {
    fn description(&self) -> &str {
        match self {
            &SimpleError::Str(ref msg) => msg,
            &SimpleError::IoError(ref e) => e.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}



