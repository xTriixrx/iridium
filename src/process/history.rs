use std::env;
use std::fs::File;
use std::io::Write;
use rev_lines::RevLines;
use std::fs::OpenOptions;

#[cfg(windows)]
const LINE_ENDING: &'static str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &'static str = "\n";

pub fn append_history(timestamp: u64, status: Option<i32>, line: &str) {
    let history_file_path = String::from(env::var("HOME").unwrap() +
        "/.iridium_history");
    
    let status_code = match status {
        Some(val) => val,
        None => 1,
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_file_path)
        .unwrap();

    let last_char = line.chars().last().unwrap();

    if last_char.to_string() == LINE_ENDING {
        if let Err(e) = write!(file, "{}:{}:{}", timestamp, status_code, line) {
            eprintln!("Unable to write to history file: {}", e);
        }
        return;    
    }

    if let Err(e) = writeln!(file, "{}:{}:{}", timestamp, status_code, line) {
        eprintln!("Unable to write to history file: {}", e);
    }
}

pub fn history(_args: &[String]) -> Option<i32> {
    let history_file_path = String::from(env::var("HOME").unwrap() +
        "/.iridium_history");
    
    let file = match File::open(history_file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Unable to read history file: {}", e);
            return None;
        },
    };

    let lines = lines_from_file(&file, 100);
    
    for (i, line) in lines.into_iter().enumerate() {
        let cmd: &str = line.split(":").last().unwrap();
        println!("{} {}", i, cmd);
    }

    return Some(0);
}

// Need to clean this up... very rough impl
// Ideally, the rev_lines module would implement the FromIterator<String, RevLinesError> trait...
// That way you can write the following:
// rev_lines.take(100).collect();
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