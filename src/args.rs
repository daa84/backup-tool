extern crate docopt;

use self::docopt::Docopt;

#[cfg_attr(rustfmt, rustfmt_skip)]
const USAGE: &'static str = "
Backup Tool

Usage:
  backup-tool
  backup-tool test
  backup-tool zip <src> <dst>
  backup-tool (-h | --help)

Options:
  -h --help     Show this screen.
";

#[derive(RustcDecodable)]
pub struct Args {
    pub cmd_test: bool,
    pub cmd_zip: bool,
    pub arg_src: Option<String>,
    pub arg_dst: Option<String>,
}

impl Args {
    pub fn parse() -> Args {
        Docopt::new(USAGE)
            .and_then(|d| d.decode())
            .unwrap_or_else(|e| e.exit())
    }
}
