use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

use crate::process::history::history_file_path;

/// Default maximum number of history entries to load for hinting.
const DEFAULT_HISTORY_LIMIT: usize = 1024;

/// Load shell history lines from disk up to the requested limit.
pub fn load_history_entries(limit: Option<usize>) -> io::Result<Vec<String>> {
    let path = history_file_path();
    let limit = limit.unwrap_or(DEFAULT_HISTORY_LIMIT);

    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };

    let reader = BufReader::new(file);
    let mut entries = VecDeque::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(cmd) = parse_history_command(&line) {
            entries.push_back(cmd);
            if entries.len() > limit {
                entries.pop_front();
            }
        }
    }

    Ok(entries.into_iter().collect())
}

/// Parse a persisted history line and extract the raw command if present.
fn parse_history_command(line: &str) -> Option<String> {
    let mut parts = line.splitn(3, ':');
    let timestamp = parts.next()?;
    if timestamp.is_empty() {
        return None;
    }
    parts.next()?; // status field, can be ignored
    let command = parts.next()?;
    if command.is_empty() {
        None
    } else {
        Some(command.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_history_command;

    #[test]
    fn parses_basic_command() {
        let line = "1695938355:0:ls -la";
        assert_eq!(parse_history_command(line).as_deref(), Some("ls -la"));
    }

    #[test]
    fn ignores_incomplete_lines() {
        assert!(parse_history_command("1695938355:0").is_none());
        assert!(parse_history_command("1695938355").is_none());
    }
}
