
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
    println!("Load config '{}' ...", CONFIG_FILE);

    // TODO: process errors to email
    let settings = load_settings().unwrap();
    run_commands(&settings.commands);
}

fn run_commands(commands: &[String]) -> Result<(), String> {
    for command in commands {
        println!("Execute {} ...", command);
        let status = match Command::new(command).status() {
            Err(e) => return Err(format!("Failed to execute process: {}", e)),
            Ok(status) => status
        };
        println!("Status {}", status);
    }
    Ok(())
}

fn load_settings() -> Result<Settings, String> {
    let mut config_str = String::new();

    match File::open(CONFIG_FILE) {
        Err(msg) => return Err(msg.description().to_string()),
        Ok(mut f) => 
            if let Err(msg) = f.read_to_string(&mut config_str) {
                return Err(msg.description().to_string());
            }
    };
        

    let mut parser = toml::Parser::new(&config_str);
    let config = match parser.parse() {
        None => return Err(format!("Can't parse config file, {:?}", parser.errors)),
        Some(config) => config
    };

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
                    commands.as_slice().expect("Wrong type of comands section")
                        .iter().map(|v| v.as_str().expect("Command must be string").to_string()).collect::<Vec<String>>()
                }
            }
        }
    };

    Ok(Settings {
        commands: commands
    })
}

