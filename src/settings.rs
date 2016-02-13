
extern crate toml;

use std::fs::File;
use std::io::prelude::*;

#[derive(RustcDecodable)]
pub struct Notify {
    pub error_address: Vec<String>,
    pub success_address: Vec<String>,

    pub smtp_host: String,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_port: u16,
    pub smtp_from: String,
}

#[derive(RustcDecodable)]
pub struct Ftp {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub backup_dir: String,
    pub backup_file_name: String,
    pub backup_suffix_format: String,
}

#[derive(RustcDecodable)]
pub struct Run {
    pub commands: Vec<String>,
}

#[derive(RustcDecodable)]
pub struct Src {
    pub path: String,
    pub prefix: String,
}

#[derive(RustcDecodable)]
pub struct Settings {
    pub run: Run,
    pub ftp: Ftp,
    pub src: Vec<Src>,
    pub notify: Notify,
}

pub const CONFIG_FILE: &'static str = "config.toml";

impl Settings {
    pub fn load() -> Settings {
        info!("Load config '{}' ...", CONFIG_FILE);

        let mut f = File::open(CONFIG_FILE).expect("Can't open config file");

        let mut config_str = String::new();

        f.read_to_string(&mut config_str).expect("Can't read config file");

        toml::decode_str(&config_str).expect("can't decode config string")
    }
}
