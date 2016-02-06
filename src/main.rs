
extern crate rustc_serialize;
extern crate toml;
extern crate ftp;
extern crate tempdir;
extern crate zip;

use std::io::prelude::*;
use std::fs::File;
use std::error::Error;
use std::process::Command;

use zip::ZipWriter;
use zip::result::ZipError;

use tempdir::TempDir;

use ftp::FTPStream;

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
        Err(e) => println!("Error in backup: {}", e.description()),
        Ok(_) => println!("Backup finished")
    }
    
}

fn run(settings: &Settings) -> Result<(), SimpleError> {
    try!(run_commands(&settings.run.commands));

    let temp_dir = try!(TempDir::new("backup-tool"));
    try!(create_archive(&temp_dir, &settings.src));
    try!(send_to_ftp(&settings));
    Ok(())
}

fn create_archive(temp_dir: &TempDir, src_list: &Vec<Src>) -> Result<(), SimpleError> {
    let file = try!(File::create(&temp_dir.path().join("backup.zip")));

    let mut zip = ZipWriter::new(file);

    for src in src_list {
        try!(write_dir(&mut zip, &src));
    }

    try!(zip.finish());
    Ok(())
}

fn write_dir(zip: &mut ZipWriter<File>, src: &Src) -> Result<(), SimpleError> {
    //try!(zip.start_file("test/", zip::CompressionMethod::Stored));

    //try!(zip.start_file("test/☃.txt", zip::CompressionMethod::Stored));
    //try!(zip.write_all(b"Hello, World!\n"));

    //try!(zip.start_file("test/lorem_ipsum.txt", zip::CompressionMethod::Deflated));
    //try!(zip.write_all(LOREM_IPSUM));
    Ok(())
}

fn send_to_ftp(settings: &Settings) -> Result<(), SimpleError> {
    let mut ftp_stream = try!(FTPStream::connect(settings.ftp.host.to_owned(), settings.ftp.port));
    try!(ftp_stream.login(&settings.ftp.user, &settings.ftp.pass));

    for src in &settings.src {
        
    }

    ftp_stream.quit();
    Ok(())
}

fn run_commands(commands: &[String]) -> Result<(), SimpleError> {
    println!("Execute commands");
    for command in commands {
        println!("Execute '{}' ...", command);
        let status = try!(Command::new(command).status());
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

impl From<String> for SimpleError {
    fn from(e: String) -> Self {
        SimpleError::Str(e)
    }
}

impl From<ZipError> for SimpleError {
    fn from(e: ZipError) -> Self {
        SimpleError::Str(e.description().to_owned())
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



