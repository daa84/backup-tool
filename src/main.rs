
extern crate rustc_serialize;
extern crate toml;
extern crate ftp;
extern crate tempdir;
extern crate zip;
extern crate walkdir;

use std::io::prelude::*;
use std::fs::File;
use std::error::Error;
use std::process::Command;

use zip::ZipWriter;

use tempdir::TempDir;

use ftp::FTPStream;

use walkdir::WalkDir;

#[derive(RustcDecodable)]
struct Ftp {
    host: String,
    port: u16,
    user: String,
    pass: String
}

#[derive(RustcDecodable)]
struct Run {
    commands: Vec<String>,
}

#[derive(RustcDecodable)]
struct Src {
    path: String,
    prefix: String
}

#[derive(RustcDecodable)]
struct Settings {
    run: Run,
    ftp: Ftp,
    src: Vec<Src>
}

const CONFIG_FILE: &'static str = "config.toml";

fn main() {
    let settings = load_settings();

    // TODO: process errors to email
    match run(&settings) {
        Err(e) => println!("Error in backup: {}", e),
        Ok(_) => println!("Backup finished")
    }
    
}

fn run(settings: &Settings) -> Result<(), String> {
    try!(run_commands(&settings.run.commands));

    let temp_dir = try!(TempDir::new("backup-tool").map_err(|e|e.to_string()));
    try!(create_archive(&temp_dir, &settings.src));
    try!(send_to_ftp(&settings));
    Ok(())
}

fn create_archive(temp_dir: &TempDir, src_list: &Vec<Src>) -> Result<(), String> {
    let file = try!(File::create(&temp_dir.path().join("backup.zip")).map_err(|e|e.to_string()));

    let mut zip = ZipWriter::new(file);

    for src in src_list {
        try!(write_dir(&mut zip, &src));
    }

    try!(zip.finish().map_err(|e|e.to_string()));
    Ok(())
}

fn write_dir(zip: &mut ZipWriter<File>, src: &Src) -> Result<(), String> {
    for entry in WalkDir::new(&src.path) {
        let dir_entry = try!(entry.map_err(|e|e.to_string()));
        let path = dir_entry.path();
        let zip_path = path.join(&src.prefix);
        
        try!(zip.start_file(zip_path.to_str().unwrap(), zip::CompressionMethod::Stored).map_err(|e|e.to_string()));
        if path.is_file() {
            let mut file_content = try!(File::open(path).map_err(|e|e.to_string()));
            try!(std::io::copy(&mut file_content, zip).map_err(|e|e.to_string()));
        }
    }
    Ok(())
}

fn send_to_ftp(settings: &Settings) -> Result<(), String> {
    let mut ftp_stream = try!(FTPStream::connect(settings.ftp.host.to_owned(), settings.ftp.port).map_err(|e|e.to_string()));
    try!(ftp_stream.login(&settings.ftp.user, &settings.ftp.pass));

    try!(ftp_stream.quit().map_err(|e|e.to_string()));
    Ok(())
}

fn run_commands(commands: &[String]) -> Result<(), String> {
    println!("Execute commands");
    for command in commands {
        println!("Execute '{}' ...", command);
        let status = try!(Command::new(command).status().map_err(|e|e.to_string()));
        println!("Status '{}'", status);
    }
    Ok(())
}

fn load_settings() -> Settings {
    println!("Load config '{}' ...", CONFIG_FILE);

    let mut f = File::open(CONFIG_FILE).expect("Can't open config file");

    let mut config_str = String::new();

    f.read_to_string(&mut config_str).expect("Can't read config file");

    toml::decode_str(&config_str).expect("can't decode config string")
}

