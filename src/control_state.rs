//! State machine backing the interactive control loop.

use shlex;
use std::env;
use crate::process;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::process::builtin::map::BuiltinMap;

/// Signals whether the control loop should continue or exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    /// Keep reading user input.
    CONTINUE,
    /// Terminate the control loop.
    EXIT,
}

/// Shared state that backs the interactive control loop.
pub struct ControlState {
    status: Option<i32>,
    builtin_map: BuiltinMap,
}

impl ControlState {
    /// Build a new control state with the default builtin set.
    pub fn new() -> Self {
        let builtin_map = BuiltinMap::new();
        Self {
            status: Some(0),
            builtin_map,
        }
    }

    /// Render the prompt string with status colouring and the current directory.
    pub fn prompt(&self) -> String {
        generate_prompt(self.status, &self.builtin_map.get_pwd())
    }

    /// Parse and execute a single line of user input, updating status and history.
    pub fn handle_line(&mut self, line: &str) -> ControlFlow {
        let mut tokens = parse_tokens(line);
        tokens = alias_parser(&self.builtin_map, tokens);

        let unix_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.status = process::execute(&self.builtin_map, &tokens);

        if !line.is_empty() {
            process::history::append_history(unix_timestamp, self.status, line);
        }

        if self.status == Some(process::exit::EXIT_CODE) {
            ControlFlow::EXIT
        } else {
            ControlFlow::CONTINUE
        }
    }
}

/// Construct the shell prompt string combining status colouring and the cwd.
fn generate_prompt(status: Option<i32>, pwd: &String) -> String {
    let arrow = 0x27A3;
    let red_text = "\u{1b}[31m";
    let green_text = "\u{1b}[32m";
    let purple_text = "\u{1b}[35m";
    let end_color_text = "\u{1b}[39m";

    format!(
        "{}{}{}{}{}{}{}{}",
        purple_text,
        update_cwd(pwd),
        match char::from_u32(0x0020) {
            Some(space) => space,
            None => ' ',
        },
        end_color_text,
        match status {
            Some(0) => green_text,
            _ => red_text,
        },
        match char::from_u32(arrow) {
            Some(arrow) => arrow,
            None => '>',
        },
        end_color_text,
        match char::from_u32(0x0020) {
            Some(space) => space,
            None => ' ',
        }
    )
}

/// Expand tokens if they match a defined alias, falling back to the original tokens.
fn alias_parser(builtin_map: &BuiltinMap, tokens: Vec<String>) -> Vec<String> {
    let aliases = builtin_map.get_alias();
    let aliases_borrow = aliases.as_ref().borrow();
    let alias = tokens.join(" ");

    if aliases_borrow.contains_alias(&alias) {
        let expansion = aliases_borrow.get_alias_expansion(&alias).unwrap();
        return parse_tokens(expansion);
    }

    tokens
}

/// Replace the home directory portion of the cwd with `~` for a compact prompt.
fn update_cwd(cwd: &str) -> String {
    cwd.replace(
        &env::var("HOME").expect("Expected HOME environment variable to be set, aborting now."),
        "~",
    )
}

/// Use shell-like parsing rules to split the input line into tokens.
fn parse_tokens(line: &str) -> Vec<String> {
    match shlex::split(line) {
        Some(vec) => vec,
        None => panic!("Unable to parse string: {}", line),
    }
}
