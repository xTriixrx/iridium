//! State machine backing the interactive control loop.

use shlex;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::editor::buffer_editor::{BufferEditor, EditorAction, EditorMode};
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
    buffers: BufferStore,
}

#[derive(Debug, Clone)]
enum ShellMode {
    Prompt,
    Buffer(BufferEditor),
}

impl ControlState {
    /// Build a new control state with the default builtin set.
    pub fn new() -> Self {
        let builtin_map = BuiltinMap::new();
        Self {
            status: Some(0),
            builtin_map,
            mode: ShellMode::Prompt,
            buffers: BufferStore::new(),
        }
    }

    /// Render the prompt string with status colouring and the current directory.
    pub fn prompt(&self) -> String {
        match &self.mode {
            ShellMode::Prompt => generate_prompt(self.status, &self.builtin_map.get_pwd()),
            ShellMode::Buffer(editor) => editor.prompt(),
        }
    }

    /// Parse and execute a single line of user input, updating status and history.
    pub fn handle_line(&mut self, line: &str) -> ControlFlow {
        match &mut self.mode {
            ShellMode::Prompt => self.handle_prompt_line(line),
            ShellMode::Buffer(editor) => {
                let buffer_name = editor.name().to_string();
                let action = editor.handle_input(line);
                self.apply_editor_action(buffer_name, action);
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
        if command == ":buffers" {
            if self.buffers.is_empty() {
                println!("(no buffers)");
            } else {
                for name in self.buffers.list() {
                    println!("- {name}");
                }
            }
            return ControlFlow::CONTINUE;
        }

        if let Some(rest) = command.strip_prefix(":buffer") {
            let name = rest.trim();
            if name.is_empty() {
                println!(":buffer requires a name");
                return ControlFlow::CONTINUE;
            }

            let buffer_name = name.to_string();
            self.buffers.open(buffer_name.clone());
            self.mode = ShellMode::Buffer(BufferEditor::new(buffer_name.clone()));
            println!("Opened buffer '{buffer_name}'. Enter 'i' to insert text and ':q' to close.");
            return ControlFlow::CONTINUE;
        }

        println!("Unknown command: {command}");
        ControlFlow::CONTINUE
    }

    fn apply_editor_action(&mut self, buffer_name: String, action: EditorAction) {
        match action {
            EditorAction::Append(line) => {
                let buffer = self.buffers.open(buffer_name.clone());
                buffer.append(line);
            }
            EditorAction::Clear => {
                if let Some(buffer) = self.buffers.get_mut(&buffer_name) {
                    buffer.clear();
                }
                println!("Cleared buffer '{buffer_name}'.");
            }
            EditorAction::DeleteLast => {
                if let Some(buffer) = self.buffers.get_mut(&buffer_name) {
                    match buffer.remove_last() {
                        Some(_) => println!("Deleted last line."),
                        None => println!("Buffer '{buffer_name}' is empty."),
                    }
                }
            }
            EditorAction::Show => {
                if let Some(buffer) = self.buffers.get(&buffer_name) {
                    buffer.print();
                }
            }
            EditorAction::ListBuffers => {
                if self.buffers.is_empty() {
                    println!("(no buffers)");
                } else {
                    for name in self.buffers.list() {
                        println!("- {name}");
                    }
                }
            }
            EditorAction::Quit { write } => {
                if write {
                    if let Some(buffer) = self.buffers.get(&buffer_name) {
                        buffer.print();
                    }
                }
                println!("Leaving buffer '{buffer_name}'.");
                self.mode = ShellMode::Prompt;
            }
            EditorAction::SwitchMode(mode) => match mode {
                EditorMode::Insert => println!("-- INSERT --"),
                EditorMode::Command => println!("-- COMMAND --"),
            },
            EditorAction::UnknownCommand(cmd) => {
                println!("Unknown buffer command '{cmd}'. Use ':w', ':q', ':clear', 'i'.");
            }
            EditorAction::NoOp => {}
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
