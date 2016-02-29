extern crate toml;

use std::fs::File;
use std::io::prelude::*;

#[derive(RustcDecodable, Clone)]
pub struct Notify {
    pub error_address: Vec<String>,
    pub success_address: Vec<String>,

    pub smtp_host: String,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_port: u16,
    pub smtp_from: String,
}

#[derive(RustcDecodable, Clone)]
pub struct Ftp {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub path: String,
    pub backup_file_name: String,
    pub backup_suffix_format: String,
}

#[derive(RustcDecodable, Clone)]
pub struct Run {
    pub commands: Vec<String>,
}

#[derive(RustcDecodable, Clone)]
pub struct Src {
    pub path: String,
    pub prefix: String,
}

#[derive(RustcDecodable, Clone)]
pub struct Schedule {
    pub time: String,
}

#[derive(RustcDecodable, Clone)]
pub struct Settings {
    pub run: Run,
    pub ftp: Ftp,
    pub src: Vec<Src>,
    pub notify: Notify,
    pub schedule: Option<Schedule>,
}

const CONFIG_FILE: &'static str = "config.toml";

impl Settings {
    pub fn load() -> Result<Settings, String> {
        info!("Load config '{}' ...", CONFIG_FILE);

        let mut f = File::open(CONFIG_FILE).expect("Can't open config file");

        let mut config_str = String::new();

        f.read_to_string(&mut config_str).expect("Can't read config file");

        let mut parser = toml::Parser::new(&config_str);
        let value = try!(parser.parse()
                               .ok_or_else(|| format!("Error parsing {:?}", parser.errors)));

        ::rustc_serialize::Decodable::decode(&mut toml::Decoder::new(toml::Value::Table(value)))
            .map_err(|e| e.to_string())
    }
}
