//! Buffer command parsing utilities.

use shlex;

/// Represents a parsed `:b` buffer command broken into option groups and values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferCommand {
    pre_session_options: Vec<char>,
    post_session_options: Vec<char>,
    args: Vec<String>,
}

impl BufferCommand {
    /// Build a buffer command from its constituent parts.
    fn new(
        pre_session_options: Vec<char>,
        post_session_options: Vec<char>,
        args: Vec<String>,
    ) -> Self {
        Self {
            pre_session_options,
            post_session_options,
            args,
        }
    }

    /// Options that must be handled prior to launching the buffer session.
    pub fn pre_session_options(&self) -> &[char] {
        &self.pre_session_options
    }

    /// Options that should be handled after the buffer session ends.
    pub fn post_session_options(&self) -> &[char] {
        &self.post_session_options
    }

    /// Positional buffer arguments provided to the command.
    pub fn args(&self) -> &[String] {
        &self.args
    }
}

/// Attempt to parse a `:b` command into short options and buffer arguments.
pub fn parse(input: &str) -> Option<BufferCommand> {
    let tokens = match shlex::split(input) {
        Some(tokens) => tokens,
        None => return None,
    };
    let Some(first) = tokens.first() else {
        return None;
    };

    if first != ":b" {
        return None;
    }

    let (options, args) = split_short_options(&tokens[1..]);
    let (pre_session_options, post_session_options) = partition_options(options);

    Some(BufferCommand::new(
        pre_session_options,
        post_session_options,
        args,
    ))
}

fn split_short_options(tokens: &[String]) -> (Vec<char>, Vec<String>) {
    let mut options = Vec::new();
    let mut args = Vec::new();

    for token in tokens {
        if let Some(stripped) = token.strip_prefix('-') {
            if stripped.is_empty() || token.starts_with("--") {
                args.push(token.clone());
                continue;
            }

            stripped.chars().for_each(|ch| options.push(ch));
        } else {
            args.push(token.clone());
        }
    }

    (options, args)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimingBucket {
    PreSession,
    PostSession,
}

fn partition_options(options: Vec<char>) -> (Vec<char>, Vec<char>) {
    let mut pre_session = Vec::new();
    let mut post_session = Vec::new();

    for option in options {
        match option_timing(option) {
            TimingBucket::PreSession => pre_session.push(option),
            TimingBucket::PostSession => post_session.push(option),
        }
    }

    (pre_session, post_session)
}

fn option_timing(option: char) -> TimingBucket {
    match option {
        'l' => TimingBucket::PostSession,
        _ => TimingBucket::PostSession,
    }
}

#[cfg(test)]
mod tests {
    use super::{TimingBucket, option_timing, parse};

    #[test]
    fn parse_list_only() {
        let command = parse(":b -l").expect("expected parse result");
        assert_eq!(command.pre_session_options(), &[]);
        assert_eq!(command.post_session_options(), &['l']);
        assert!(command.args().is_empty());
    }

    #[test]
    fn parse_single_file() {
        let command = parse(":b file").expect("expected parse result");
        assert_eq!(command.pre_session_options(), &[]);
        assert_eq!(command.post_session_options(), &[]);
        assert_eq!(command.args(), &["file".to_string()]);
    }

    #[test]
    fn parse_list_with_file() {
        let command = parse(":b -l file").expect("expected parse result");
        assert_eq!(command.post_session_options(), &['l']);
        assert_eq!(command.args(), &[String::from("file")]);
    }

    #[test]
    fn parse_list_with_multiple_files() {
        let command = parse(":b -l file1 file2").expect("expected parse result");
        assert_eq!(command.post_session_options(), &['l']);
        assert_eq!(
            command.args(),
            &[String::from("file1"), String::from("file2")]
        );
    }

    #[test]
    fn classify_option_timing() {
        assert_eq!(option_timing('l'), TimingBucket::PostSession);
        assert_eq!(option_timing('x'), TimingBucket::PostSession);
    }
}
