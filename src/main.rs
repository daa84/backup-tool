
#[macro_use]
extern crate log;
extern crate log4rs;

extern crate rustc_serialize;
extern crate ftp;
extern crate tempdir;
extern crate zip;
extern crate walkdir;
extern crate time;
extern crate lettre;

use std::error::Error;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
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

mod settings;
mod args;

use settings::*;
use args::*;

fn main() {
    log4rs::init_file("log.toml", Default::default()).unwrap();

    let settings = match Settings::load() {
        Err(err) => {
            error!("{}", err);
            panic!(err)
        }
        Ok(value) => value,
    };

    let args = Args::parse();
    if args.cmd_test {
        test_run(&settings);
    }
    else {
        backup(&settings);
    }
}

fn backup(settings: &Settings) {
    match run(settings) {
        Err(e) => {
            error!("Error {}", e);
            notify(&settings.notify,
                   &settings.notify.error_address,
                   "Error backup",
                   e.description());
        }
        Ok(_) => {
            info!("Backup finished successfull");
            notify(&settings.notify,
                   &settings.notify.success_address,
                   "Backup finished",
                   "Ok");
        }
    }
}

fn notify(notify: &Notify, tos: &Vec<String>, subject: &str, body: &str) {
    if tos.is_empty() {
        return;
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

fn run(settings: &Settings) -> Result<(), Box<Error>> {
    try!(run_commands(&settings.run.commands));

    let temp_dir = try!(TempDir::new("backup-tool"));
    let archive = try!(create_archive(&temp_dir, &settings.src));
    try!(send_to_ftp(&archive, &settings));
    Ok(())
}

fn create_archive(temp_dir: &TempDir, src_list: &Vec<Src>) -> Result<PathBuf, Box<Error>> {
    let archive_path = temp_dir.path().join("backup.zip");
    let file = try!(File::create(&archive_path));

    let mut zip = ZipWriter::new(file);

    for src in src_list {
        try!(write_dir(&mut zip, &src));
    }

    try!(zip.finish());
    Ok(archive_path)
}

fn write_dir(zip: &mut ZipWriter<File>, src: &Src) -> Result<(), Box<Error>> {
    for entry in WalkDir::new(&src.path) {
        let dir_entry = try!(entry);
        let path = dir_entry.path();
        let zip_path = Path::new(&src.prefix).join(&path);

        try!(zip.start_file(zip_path.to_str().unwrap(), zip::CompressionMethod::Stored));
        if path.is_file() {
            let mut file_content = try!(File::open(path));
            try!(std::io::copy(&mut file_content, zip));
        }
    }
    Ok(())
}

fn send_to_ftp(archive: &Path, settings: &Settings) -> Result<(), Box<Error>> {
    let mut ftp_stream = try!(FTPStream::connect(settings.ftp.host.to_owned(), settings.ftp.port));
    try!(ftp_stream.login(&settings.ftp.user, &settings.ftp.pass));

    try!(ftp_stream.change_dir(&settings.ftp.path));
    let time_str = try!(strftime(&settings.ftp.backup_suffix_format, &now()));
    let target_file_format = format!("{}-{}", settings.ftp.backup_file_name, time_str);

    let mut target_file = format!("{}.zip", target_file_format);
    for i in 1..100 {
        if let Err(_) = ftp_stream.retr(&target_file) {
            break;
        }

        if i >= 99 {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::AlreadyExists,
                                                    "To match backup files exists")));
        }
        target_file = format!("{}-{}.zip", target_file_format, i);
    }
    let mut src_file = try!(File::open(archive));
    try!(ftp_stream.stor(&target_file, &mut src_file));

    try!(ftp_stream.quit());
    Ok(())
}

fn run_commands(commands: &[String]) -> Result<(), Box<Error>> {
    info!("Execute commands");
    for command in commands {
        info!("Execute '{}' ...", command);
        let status = try!(Command::new(command).status());
        info!("Status '{}'", status);
    }
    Ok(())
}

fn test_run(settings: &Settings) {
    test_run_commands(&settings.run.commands);
    if let Err(e) = test_ftp(settings) {
        error!("Error on connecting to ftp {}", e);
    }
    test_file_format(settings);
    info!("Test finisehd!");
}

fn test_run_commands(commands: &[String]) {
    for command in commands {
        if !Path::new(command).exists() {
            error!("Command file '{}' does not exists", command)
        }
    }
}

fn test_file_format(settings: &Settings) {
    let time_str = strftime(&settings.ftp.backup_suffix_format, &now()).expect("Wrong suffix time format");
    let target_file_format = format!("{}-{}", settings.ftp.backup_file_name, time_str);
    let target_file = format!("{}.zip", target_file_format);
    info!("Target file {}", target_file);
}

fn test_ftp(settings: &Settings) -> Result<(), Box<Error>> {
    let mut ftp_stream = try!(FTPStream::connect(settings.ftp.host.to_owned(), settings.ftp.port));
    try!(ftp_stream.login(&settings.ftp.user, &settings.ftp.pass));
    try!(ftp_stream.change_dir(&settings.ftp.path));
    try!(ftp_stream.quit());
    Ok(())
}

