use crate::process::builtin::Builtin;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;

/// Output destination for alias diagnostics and listings.
pub enum AliasSink {
    Stdout,
    Stderr,
    #[allow(dead_code)]
    Buffer(Rc<RefCell<Vec<u8>>>),
}

impl AliasSink {
    fn write_line(&mut self, line: &str) {
        match self {
            AliasSink::Stdout => {
                let mut out = io::stdout();
                let _ = writeln!(out, "{}", line);
            }
            AliasSink::Stderr => {
                let mut err = io::stderr();
                let _ = writeln!(err, "{}", line);
            }
            AliasSink::Buffer(buffer) => {
                let mut buf = buffer.borrow_mut();
                buf.extend_from_slice(line.as_bytes());
                buf.push(b'\n');
            }
        }
    }
}

impl Default for AliasSink {
    fn default() -> Self {
        AliasSink::Stdout
    }
}

// ENV variables used:
// LANG
// LC_ALL
// LC_CTYPE
// LC_MESSAGES
// NLSPATH
// man page: https://www.man7.org/linux/man-pages/man1/alias.1p.html

/// Stores command aliases and exposes the POSIX `alias` builtin behaviour.
pub struct Alias {
    alias_map: HashMap<String, String>,
    stdout: AliasSink,
    stderr: AliasSink,
}

impl Builtin for Alias {
    /// Print, query, or define shell aliases according to the provided arguments.
    fn call(&mut self, args: &[String]) -> Option<i32> {
        let mut queries = Vec::new();
        let mut definitions = Vec::new();

        for arg in args {
            if arg.starts_with('-') && !arg.contains('=') {
                let message = format!("alias: {}: invalid option", arg);
                self.stderr.write_line(&message);
                return Some(1);
            }

            if let Some(eq_index) = arg.find('=') {
                let name = arg[..eq_index].to_string();
                let value = arg[eq_index + 1..].to_string();
                definitions.push((name, value));
            } else {
                queries.push(arg.to_string());
            }
        }

        for (name, value) in definitions {
            self.insert_alias(&name, &value);
        }

        if args.is_empty() {
            self.write_all_definitions();
            return Some(0);
        }

        let mut status = 0;

        for name in queries {
            if let Some(value) = self.alias_map.get(&name).cloned() {
                let line = format_definition(&name, &value);
                self.stdout.write_line(&line);
            } else {
                let message = format!("alias: {}: not found", name);
                self.stderr.write_line(&message);
                status = 1;
            }
        }

        Some(status)
    }
}

impl Alias {
    /// Create an alias builtin that writes to standard streams.
    pub fn new() -> Self {
        Self {
            alias_map: HashMap::new(),
            stdout: AliasSink::Stdout,
            stderr: AliasSink::Stderr,
        }
    }

    /// Construct an alias builtin with custom sinks (useful for testing).
    #[allow(dead_code)]
    pub fn with_sinks(stdout: AliasSink, stderr: AliasSink) -> Self {
        Self {
            alias_map: HashMap::new(),
            stdout,
            stderr,
        }
    }

    /// Replace the output sinks for stdout and stderr.
    #[allow(dead_code)]
    pub fn set_sinks(&mut self, stdout: AliasSink, stderr: AliasSink) {
        self.stdout = stdout;
        self.stderr = stderr;
    }

    /// Store or replace an alias mapping.
    fn insert_alias(&mut self, alias_name: &str, expansion: &str) -> Option<String> {
        self.alias_map
            .insert(alias_name.to_string(), expansion.to_string())
    }

    /// Check if a given alias key is defined.
    pub fn contains_alias(&self, alias_name: &str) -> bool {
        self.alias_map.contains_key(alias_name)
    }

    /// Retrieve the stored expansion for an alias, if any.
    pub fn get_alias_expansion(&self, alias_name: &str) -> Option<&String> {
        self.alias_map.get(alias_name)
    }

    fn write_all_definitions(&mut self) {
        let mut names: Vec<String> = self.alias_map.keys().cloned().collect();
        names.sort();

        for name in names {
            if let Some(value) = self.alias_map.get(&name).cloned() {
                let line = format_definition(&name, &value);
                self.stdout.write_line(&line);
            }
        }
    }
}

/// Render an alias definition using POSIX-compliant quoting rules.
pub fn format_definition(name: &str, value: &str) -> String {
    format!("alias {}={}", name, single_quote(value))
}

fn single_quote(value: &str) -> String {
    let mut quoted = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            quoted.push('\'');
            quoted.push('\\');
            quoted.push('\'');
            quoted.push('\'');
        } else {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_alias() -> (Alias, Rc<RefCell<Vec<u8>>>, Rc<RefCell<Vec<u8>>>) {
        let stdout_buffer = Rc::new(RefCell::new(Vec::new()));
        let stderr_buffer = Rc::new(RefCell::new(Vec::new()));
        let alias = Alias::with_sinks(
            AliasSink::Buffer(stdout_buffer.clone()),
            AliasSink::Buffer(stderr_buffer.clone()),
        );
        (alias, stdout_buffer, stderr_buffer)
    }

    fn buffer_to_string(buffer: &Rc<RefCell<Vec<u8>>>) -> String {
        String::from_utf8(buffer.borrow().clone()).unwrap()
    }

    #[test]
    fn reports_default_sink_as_stdout() {
        assert!(matches!(AliasSink::default(), AliasSink::Stdout));
    }

    #[test]
    fn contains_alias_returns_true_for_existing_alias() {
        let mut alias = Alias::new();
        let _ = alias.call(&["ll=ls".into()]);
        assert!(alias.contains_alias("ll"));
    }

    #[test]
    fn get_alias_expansion_returns_value_when_present() {
        let (mut alias, _, _) = setup_alias();
        let _ = alias.call(&["ll=ls".into()]);
        assert_eq!(
            alias.get_alias_expansion("ll").map(|s| s.as_str()),
            Some("ls")
        );
    }

    #[test]
    fn alias_sink_writes_to_stdout() {
        let buffer = Rc::new(RefCell::new(Vec::new()));
        // Using buffer to simulate stdout.
        let mut sink = AliasSink::Buffer(buffer.clone());
        sink.write_line("test");
        assert_eq!(buffer_to_string(&buffer), "test\n");
    }

    #[test]
    fn alias_sink_writes_to_stderr() {
        let buffer = Rc::new(RefCell::new(Vec::new()));
        let mut sink = AliasSink::Buffer(buffer.clone());
        sink.write_line("error");
        assert_eq!(buffer_to_string(&buffer), "error\n");
    }

    #[test]
    fn prints_all_aliases_when_no_arguments() {
        let (mut alias, stdout, stderr) = setup_alias();
        let _ = alias.call(&["ls=ls -p".into()]);
        let _ = alias.call(&["grep=grep --color=auto".into()]);
        stdout.borrow_mut().clear();
        stderr.borrow_mut().clear();

        let status = alias.call(&[]);
        assert_eq!(status, Some(0));

        let output = buffer_to_string(&stdout);
        assert_eq!(output, "alias grep='grep --color=auto'\nalias ls='ls -p'\n");
        assert!(buffer_to_string(&stderr).is_empty());
    }

    #[test]
    fn defines_and_queries_alias() {
        let (mut alias, stdout, stderr) = setup_alias();
        let status = alias.call(&["ll=ls -al".into()]);
        assert_eq!(status, Some(0));
        stdout.borrow_mut().clear();
        stderr.borrow_mut().clear();

        let status = alias.call(&["ll".into()]);
        assert_eq!(status, Some(0));
        assert_eq!(buffer_to_string(&stdout), "alias ll='ls -al'\n");
        assert!(buffer_to_string(&stderr).is_empty());
    }

    #[test]
    fn reports_missing_alias() {
        let (mut alias, stdout, stderr) = setup_alias();
        let status = alias.call(&["unknown".into()]);
        assert_eq!(status, Some(1));
        assert!(buffer_to_string(&stdout).is_empty());
        assert_eq!(buffer_to_string(&stderr), "alias: unknown: not found\n");
    }

    #[test]
    fn rejects_invalid_option() {
        let (mut alias, stdout, stderr) = setup_alias();
        let status = alias.call(&["-x".into()]);
        assert_eq!(status, Some(1));
        assert!(buffer_to_string(&stdout).is_empty());
        assert_eq!(buffer_to_string(&stderr), "alias: -x: invalid option\n");
    }

    #[test]
    fn rejects_dash_p_option() {
        let (mut alias, stdout, stderr) = setup_alias();
        let status = alias.call(&["-p".into()]);
        assert_eq!(status, Some(1));
        assert!(buffer_to_string(&stdout).is_empty());
        assert_eq!(buffer_to_string(&stderr), "alias: -p: invalid option\n");
    }

    #[test]
    fn quotes_single_quotes_in_values() {
        let (mut alias, stdout, stderr) = setup_alias();
        let _ = alias.call(&["quote=it'".into()]);
        stdout.borrow_mut().clear();
        stderr.borrow_mut().clear();

        let status = alias.call(&["quote".into()]);
        assert_eq!(status, Some(0));
        assert_eq!(buffer_to_string(&stdout), "alias quote='it'\\'''\n");
        assert!(buffer_to_string(&stderr).is_empty());
    }
}
