
#[macro_use]
extern crate log;
extern crate log4rs;

extern crate rustc_serialize;
extern crate toml;
extern crate ftp;
extern crate tempdir;
extern crate zip;
extern crate walkdir;
extern crate time;
extern crate lettre;

use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::process::Command;

use zip::ZipWriter;

use tempdir::TempDir;

use ftp::FTPStream;

use walkdir::WalkDir;

use time::{strftime, now};

use lettre::email::EmailBuilder;
use lettre::transport::smtp::{SecurityLevel, SmtpTransportBuilder};
use lettre::transport::smtp::authentication::Mecanism;
use lettre::transport::EmailTransport;

#[derive(RustcDecodable)]
struct Notify {
    error_address: Vec<String>,
    success_address: Vec<String>,

    smtp_host: String,
    smtp_user: String,
    smtp_pass: String,
    smtp_port: u16,
    smtp_from: String,
}

#[derive(RustcDecodable)]
struct Ftp {
    host: String,
    port: u16,
    user: String,
    pass: String,
    backup_dir: String,
    backup_file_name: String,
    backup_suffix_format: String,
}

#[derive(RustcDecodable)]
struct Run {
    commands: Vec<String>,
}

#[derive(RustcDecodable)]
struct Src {
    path: String,
    prefix: String,
}

#[derive(RustcDecodable)]
struct Settings {
    run: Run,
    ftp: Ftp,
    src: Vec<Src>,
    notify: Notify,
}

const CONFIG_FILE: &'static str = "config.toml";

fn main() {
    log4rs::init_file("log.toml", Default::default()).unwrap();

    let settings = load_settings();

    match run(&settings) {
        Err(e) => {
            error!("Error {}", e);
            notify(&settings.notify, &settings.notify.error_address, "Error backup", &e);
        },
        Ok(_) => {
            info!("Backup finished successfull");
            notify(&settings.notify, &settings.notify.success_address, "Backup finished", "Ok");
        },
    };
}

fn notify(notify: &Notify, tos: &Vec<String>, subject: &str, body: &str) {
    if tos.is_empty() {
        return
    }

    let smtp_from: &str = &notify.smtp_from;
    let smtp_host: &str = &notify.smtp_host;

    let mut builder = EmailBuilder::new();
    for to in tos {
        let to_str: &str = to;
        builder = builder.to(to_str);
    }

    let email = builder.sender(smtp_from).subject(&subject).body(&body).build().unwrap();

    // Connect to a remote server on a custom port
    let mut mailer = SmtpTransportBuilder::new((smtp_host,
                                                notify.smtp_port)).unwrap()
        // Add credentials for authentication
        .credentials(&notify.smtp_user, &notify.smtp_pass)
        // Specify a TLS security level. You can also specify an SslContext with
        // .ssl_context(SslContext::Ssl23)
        .security_level(SecurityLevel::AlwaysEncrypt)
        // Enable SMTPUTF8 is the server supports it
        .smtp_utf8(true)
        // Configure accepted authetication mechanisms
        .authentication_mecanisms(vec![Mecanism::CramMd5])
        .build();

    mailer.send(email).ok().expect("Can't send mail");
}

fn run(settings: &Settings) -> Result<(), String> {
    try!(run_commands(&settings.run.commands));

    let temp_dir = try!(TempDir::new("backup-tool").map_err(|e| e.to_string()));
    let archive = try!(create_archive(&temp_dir, &settings.src));
    try!(send_to_ftp(&archive, &settings));
    Ok(())
}

fn create_archive(temp_dir: &TempDir, src_list: &Vec<Src>) -> Result<PathBuf, String> {
    let archive_path = temp_dir.path().join("backup.zip");
    let file = try!(File::create(&archive_path).map_err(|e| e.to_string()));

    let mut zip = ZipWriter::new(file);

    for src in src_list {
        try!(write_dir(&mut zip, &src));
    }

    try!(zip.finish().map_err(|e| e.to_string()));
    Ok(archive_path)
}

fn write_dir(zip: &mut ZipWriter<File>, src: &Src) -> Result<(), String> {
    for entry in WalkDir::new(&src.path) {
        let dir_entry = try!(entry.map_err(|e| e.to_string()));
        let path = dir_entry.path();
        let zip_path = Path::new(&src.prefix).join(&path);

        try!(zip.start_file(zip_path.to_str().unwrap(), zip::CompressionMethod::Stored)
                .map_err(|e| e.to_string()));
        if path.is_file() {
            let mut file_content = try!(File::open(path).map_err(|e| e.to_string()));
            try!(std::io::copy(&mut file_content, zip).map_err(|e| e.to_string()));
        }
    }
    Ok(())
}

fn send_to_ftp(archive: &Path, settings: &Settings) -> Result<(), String> {
    let mut ftp_stream = try!(FTPStream::connect(settings.ftp.host.to_owned(), settings.ftp.port)
                                  .map_err(|e| e.to_string()));
    try!(ftp_stream.login(&settings.ftp.user, &settings.ftp.pass));

    try!(ftp_stream.change_dir(&settings.ftp.backup_dir));
    let time_str = try!(strftime(&settings.ftp.backup_suffix_format, &now())
                            .map_err(|e| e.to_string()));
    let target_file_format = format!("{}-{}", settings.ftp.backup_file_name, time_str);

    let mut target_file = format!("{}.zip", target_file_format);
    for i in 1..100 {
        if let Err(_) = ftp_stream.retr(&target_file) {
            break;
        }

        if i >= 99 {
            return Err("To match backup files exists".to_owned());
        }
        target_file = format!("{}-{}.zip", target_file_format, i);
    }
    let mut src_file = try!(File::open(archive).map_err(|e| e.to_string()));
    try!(ftp_stream.stor(&target_file, &mut src_file));

    try!(ftp_stream.quit().map_err(|e| e.to_string()));
    Ok(())
}

fn run_commands(commands: &[String]) -> Result<(), String> {
    info!("Execute commands");
    for command in commands {
        info!("Execute '{}' ...", command);
        let status = try!(Command::new(command).status().map_err(|e| e.to_string()));
        info!("Status '{}'", status);
    }
    Ok(())
}

fn load_settings() -> Settings {
    info!("Load config '{}' ...", CONFIG_FILE);

    let mut f = File::open(CONFIG_FILE).expect("Can't open config file");

    let mut config_str = String::new();

    f.read_to_string(&mut config_str).expect("Can't read config file");

    toml::decode_str(&config_str).expect("can't decode config string")
}
