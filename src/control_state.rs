//! State machine backing the interactive control loop.

use crate::cmd::bufcmd;
use shlex;
use std::env;
use std::mem;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::editor::buffer_editor::BufferEditor;
use crate::editor::terminal::Terminal;
use crate::process;
use crate::process::builtin::map::BuiltinMap;
use crate::store::buffer::BufferStore;

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
    mode: ShellMode,
    buffers: Arc<Mutex<BufferStore>>,
}

#[derive(Debug, Clone)]
enum ShellMode {
    Prompt,
    Buffer(String),
}

impl ControlState {
    /// Build a new control state with the default builtin set.
    pub fn new() -> Self {
        let builtin_map = BuiltinMap::new();
        let buffers = Arc::new(Mutex::new(BufferStore::new()));
        Terminal::instance().attach_store(Arc::clone(&buffers));
        Self {
            status: Some(0),
            builtin_map,
            mode: ShellMode::Prompt,
            buffers,
        }
    }

    /// Render the prompt string with status colouring and the current directory.
    pub fn prompt(&self) -> String {
        match &self.mode {
            ShellMode::Prompt => generate_prompt(self.status, &self.builtin_map.get_pwd()),
            ShellMode::Buffer(_) => {
                let editor = BufferEditor::instance();
                let editor = editor.lock().expect("buffer editor lock poisoned");
                editor.prompt_string()
            }
        }
    }

    /// Parse and execute a single line of user input, updating status and history.
    pub fn handle_line(&mut self, line: &str) -> ControlFlow {
        match self.mode {
            ShellMode::Prompt => self.handle_prompt_line(line),
            ShellMode::Buffer(_) => {
                self.run_buffer_session();
                ControlFlow::CONTINUE
            }
        }
    }
}

impl ControlState {
    fn handle_prompt_line(&mut self, line: &str) -> ControlFlow {
        let trimmed = line.trim();

        if trimmed.starts_with(':') {
            return self.handle_prompt_command(trimmed);
        }

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

    fn handle_prompt_command(&mut self, command: &str) -> ControlFlow {
        // All buffer commands start with :b
        if command.contains(":b") {
            return self.handle_buffer_commands(&command);
        }

        // All macros commands start with :m
        if command.contains(":m") {
            return self.handle_macro_commands(&command);
        }

        // All pipeline commands start with :p
        if command.contains(":p") {
            return self.handle_pipeline_commands(&command);
        }

        println!("Unknown command: {command}");
        ControlFlow::CONTINUE
    }

    fn run_buffer_session(&mut self) {
        if let ShellMode::Buffer(buffer_name) = mem::replace(&mut self.mode, ShellMode::Prompt) {
            let editor = BufferEditor::instance();
            let mut editor = editor.lock().expect("buffer editor lock poisoned");
            editor.open(buffer_name);
            editor.run();
        }
    }

    // :b [options] <values>
    fn handle_buffer_commands(&mut self, bufcmd: &str) -> ControlFlow {
        let Some(command) = bufcmd::parse(bufcmd) else {
            println!("Unknown buffer command: {bufcmd}");
            return ControlFlow::CONTINUE;
        };

        let mut store = self.buffers.lock().expect("buffer store lock poisoned");

        self.apply_pre_session_options(&mut store, command.pre_session_options());

        let args = command.args();
        let post_session_options = command.post_session_options();

        if args.is_empty() {
            if post_session_options.is_empty() {
                println!(":buffer requires a name");
            } else {
                self.apply_post_session_options(&mut store, post_session_options, args);
            }
            return ControlFlow::CONTINUE;
        }

        let buffer_name = args.last().cloned().expect("expected buffer argument");

        for name in args {
            store.open(name.clone());
        }

        drop(store);

        self.mode = ShellMode::Buffer(buffer_name.clone());
        println!(
            "Opened buffer '{buffer_name}'. Press ':i' to enter insert mode and Ctrl+C to exit the editor."
        );
        self.run_buffer_session();

        if !post_session_options.is_empty() {
            let mut store = self.buffers.lock().expect("buffer store lock poisoned");
            self.apply_post_session_options(&mut store, post_session_options, args);
        }
        ControlFlow::CONTINUE
    }

    fn handle_macro_commands(&mut self, bufcmd: &str) -> ControlFlow {
        ControlFlow::CONTINUE
    }

    fn handle_pipeline_commands(&mut self, bufcmd: &str) -> ControlFlow {
        ControlFlow::CONTINUE
    }
}

impl ControlState {
    fn apply_pre_session_options(&self, _store: &mut BufferStore, _options: &[char]) {}

    fn apply_post_session_options(
        &self,
        store: &mut BufferStore,
        options: &[char],
        args: &[String],
    ) {
        for option in options {
            match option {
                'l' => {
                    if store.is_empty() {
                        println!("(no buffers)");
                    } else {
                        for name in store.list() {
                            println!("- {name}");
                        }
                    }
                }
                _ => {
                    if let Some(buffer_name) = args.last() {
                        println!(
                            "Unhandled post-session option '-{option}' for buffer '{buffer_name}'"
                        );
                    } else {
                        println!("Unhandled post-session option '-{option}'");
                    }
                }
            }
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
