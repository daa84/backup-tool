
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
extern crate chrono;

use std::error::Error;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, UTC};

use zip::ZipWriter;

use tempdir::TempDir;

use ftp::FtpStream;

use walkdir::WalkDir;

use time::{strftime, now};

use lettre::email::EmailBuilder;
use lettre::transport::smtp::SmtpTransportBuilder;
use lettre::transport::EmailTransport;

mod settings;
mod args;
mod timer;

use settings::*;
use args::*;

use timer::*;

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
    if settings.schedule.is_some() {
        schedule_backup(&settings);
    } else if args.cmd_zip {
        create_zip(&args.arg_src.expect("Arg <src> not given"),
                   &args.arg_dst.expect("Arg <dst> not given"));
    } else if args.cmd_test {
        test_run(&settings);
    } else {
        backup(&settings);
    }
}

fn schedule_backup(settings: &Settings) {
    let schedule = settings.schedule.as_ref().unwrap();

    let event_time = DateTime::parse_from_str(&schedule.time, "%H:%M")
                         .expect("Can't parse schedule time");

    info!("Start scheduler at '{}' every day", schedule.time);

    loop {
        let start_timestamp = UTC::now().timestamp() as u64;
        let event_timestamp = event_time.timestamp() as u64;
        let sleep_time = Duration::from_secs(event_timestamp - start_timestamp);
        info!("Sleep for {}", sleep_time.to_hhmmss());

        thread::sleep(sleep_time);

        backup(settings);
    }
}

fn create_zip(src: &str, dst: &str) {
    let mut zip_action = ZipAction::new(File::create(dst).expect("Can't create <dst> path"));
    zip_action.write_all(&vec![Src {
                                   path: src.to_string(),
                                   prefix: "".to_string(),
                               }])
              .expect("Error write <src>");
    zip_action.finish().expect("Error write file");
}

fn backup(settings: &Settings) {
    match timer::calc_time(run, settings) {
        Err((e, time)) => {
            error!("Error {}", e);
            notify(&settings.notify,
                   &settings.notify.error_address,
                   "Error backup",
                   &format!("Error in backup process: {}\nExecution time: {}",
                            e,
                            time.to_hhmmss()));
        }
        Ok((_, time)) => {
            info!("Backup finished successfull");
            notify(&settings.notify,
                   &settings.notify.success_address,
                   &format!("Backup finished\nExecution time: {}", time.to_hhmmss()),
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

    let email = builder.from(smtp_from).subject(&subject).body(&body).build().unwrap();

    let mut mailer = SmtpTransportBuilder::new((smtp_host, notify.smtp_port))
                         .unwrap()
                         .credentials(&notify.smtp_user, &notify.smtp_pass)
                         .build();

    mailer.send(email).ok().expect("Can't send mail");
}

fn run(settings: &Settings) -> Result<(), String> {
    // use thread as panic isolation bound
    let thread_settings = settings.clone();
    thread::spawn(move || {
        match _run(&thread_settings) {
            Ok(_) => (),
            Err(e) => panic!("{}", e),
        }
    })
        .join()
        .map_err(|e| e.downcast_ref::<String>().unwrap().to_owned())
}

fn _run(settings: &Settings) -> Result<(), Box<Error>> {
    try!(run_commands(&settings.run.commands));

    let temp_dir = try!(TempDir::new("backup-tool"));
    let archive = try!(create_archive(&temp_dir, &settings.src));
    try!(FtpAction::new(&settings.ftp).send_to_ftp(&archive));
    Ok(())
}

fn create_archive(temp_dir: &TempDir, src_list: &Vec<Src>) -> Result<PathBuf, Box<Error>> {
    let archive_path = temp_dir.path().join("backup.zip");
    info!("Create zip archve {}", archive_path.to_str().unwrap_or("Can't get file name"));
    let file = try!(File::create(&archive_path));

    let mut zip_action = ZipAction::new(file);
    try!(zip_action.write_all(src_list));
    try!(zip_action.finish());
    Ok(archive_path)
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

    let ftp_action = FtpAction::new(&settings.ftp);
    ftp_action.test_ftp();
    ftp_action.test_file_format();
    info!("Test finisehd!");
}

fn test_run_commands(commands: &[String]) {
    for command in commands {
        if !Path::new(command).exists() {
            error!("Command file '{}' does not exists", command)
        }
    }
}

struct ZipAction {
    writer: ZipWriter<BufWriter<File>>,
}

use std::io::BufWriter;

impl ZipAction {
    fn new(file: File) -> ZipAction {
        ZipAction { writer: ZipWriter::new(BufWriter::with_capacity(10 * 1024 * 1024, file)) }
    }

    fn finish(&mut self) -> Result<(), Box<Error>> {
        try!(self.writer.finish());
        Ok(())
    }

    fn write_all(&mut self, src_list: &Vec<Src>) -> Result<(), Box<Error>> {
        for src in src_list {
            try!(self.write_dir(src));
        }
        Ok(())
    }

    fn write_dir(&mut self, src: &Src) -> Result<(), Box<Error>> {
        info!("Add dir '{}' to archive", src.path);
        for entry in WalkDir::new(&src.path) {
            let dir_entry = try!(entry);
            let path = dir_entry.path();
            let zip_path = Path::new(&src.prefix).join(&path);

            if path.is_file() {
                let mut file_content = try!(File::open(path));
                let metadata = try!(file_content.metadata());

                if metadata.len() > 0 {
                    try!(self.writer
                         .start_file(zip_path.to_str().unwrap(), zip::CompressionMethod::Deflated));
                    try!(std::io::copy(&mut file_content, &mut self.writer));
                }
                else {
                    try!(self.writer
                         .start_file(zip_path.to_str().unwrap(), zip::CompressionMethod::Stored));
                }
            } else {
                try!(self.writer.start_file(format!("{}/", zip_path.to_str().unwrap()),
                                            zip::CompressionMethod::Stored));
            }
        }
        Ok(())
    }
}

struct FtpAction<'a> {
    settings: &'a Ftp,
}

impl<'a> FtpAction<'a> {
    fn new(ftp: &'a Ftp) -> FtpAction {
        FtpAction { settings: ftp }
    }

    fn generate_file_name(&self) -> Result<String, Box<Error>> {
        let time_str = try!(strftime(&self.settings.backup_suffix_format, &now()));
        Ok(format!("{}-{}", self.settings.backup_file_name, time_str))
    }

    fn send_to_ftp(&self, archive: &Path) -> Result<(), Box<Error>> {
        info!("Send backup to ftp");
        let mut ftp_stream = try!(self.start_ftp_session());

        let target_file_format = try!(self.generate_file_name());
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
        info!("Send file '{}' to '{}'", archive.to_str().unwrap_or("Can't get filename"), target_file);
        try!(ftp_stream.stor(&target_file, &mut src_file));

        try!(ftp_stream.quit());
        Ok(())
    }

    fn start_ftp_session(&self) -> Result<FtpStream, Box<Error>> {
        let mut ftp_stream = try!(FtpStream::connect(self.settings.host.to_owned(),
                                                     self.settings.port));
        try!(ftp_stream.login(&self.settings.user, &self.settings.pass));
        try!(ftp_stream.change_dir(&self.settings.path));
        Ok(ftp_stream)
    }

    fn test_ftp(&self) {
        match self._test_ftp() {
            Ok(_) => info!("Ftp works!"),
            Err(e) => error!("Error on connecting to ftp: {}", e),
        };
    }

    fn _test_ftp(&self) -> Result<(), Box<Error>> {
        let mut ftp_stream = try!(self.start_ftp_session());
        try!(ftp_stream.quit());
        Ok(())
    }

    fn test_file_format(&self) {
        match self.generate_file_name() {
            Ok(file_name) => info!("Target file prefix: {}", file_name),
            Err(e) => error!("Error in file name format: {}", e),
        };
    }
}
