use std::env;
use std::fs::File;
use std::io::Write;
use rev_lines::RevLines;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use crate::process::builtin::Builtin;

#[cfg(windows)]
/// Platform-specific newline used when persisting history entries.
const LINE_ENDING: &'static str = "\r\n";
#[cfg(not(windows))]
/// Platform-specific newline used when persisting history entries.
const LINE_ENDING: &'static str = "\n";

/// Implements the `history` builtin which prints recent commands.
pub struct History {}

impl Builtin for History {
    /// Dump at most the last 1000 persisted commands to stdout.
    fn call(&mut self, _args: &[String]) -> Option<i32> {
        let file = match File::open(history_file_path()) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Unable to read history file: {}", e);
                return None;
            }
        };

        let mut lines = lines_from_file(&file, 1000);
        lines.reverse();
        for (i, line) in lines.into_iter().enumerate() {
            let cmd: &str = line.split(":").last().unwrap();
            println!("{} {}", i, cmd);
        }

        Some(0)
    }
}

impl History {
    /// Construct a history builtin instance.
    pub fn new() -> Self {
        History {}
    }
}

/// Append an entry to the on-disk history log, creating the file if needed.
pub fn append_history(timestamp: u64, status: Option<i32>, line: &str) {
    let history_file_path = history_file_path();

    let status_code = match status {
        Some(val) => val,
        None => 1,
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_file_path)
        .unwrap();

    if line.ends_with(LINE_ENDING) {
        if let Err(e) = write!(file, "{}:{}:{}", timestamp, status_code, line) {
            eprintln!("Unable to write to history file: {}", e);
        }
        return;
    }

    if let Err(e) = writeln!(file, "{}:{}:{}", timestamp, status_code, line) {
        eprintln!("Unable to write to history file: {}", e);
    }
}

/// Return the fully qualified path to the shell history file.
pub fn history_file_path() -> PathBuf {
    let home =
        env::var("HOME").expect("Expected HOME environment variable to be set, aborting now.");
    Path::new(&home).join(".iridium_history")
}

// Need to clean this up... very rough impl
// Ideally, the rev_lines module would implement the FromIterator<String, RevLinesError> trait...
// That way you can write the following:
// rev_lines.take(100).collect();
/// Read up to `limit` lines from the end of the history file.
fn lines_from_file(file: &File, limit: usize) -> Vec<String> {
    let mut vec = vec![];
    let rev_lines = RevLines::new(file);

    for (i, line) in rev_lines.enumerate() {
        match line {
            Ok(line) => vec.push(line),
            Err(e) => panic!("RevLinesError in lines_from_file: {}", e),
        }
        if i == limit {
            break;
        }
    }
    return vec;
}
