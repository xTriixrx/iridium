//! State machine backing the interactive control loop.

use crate::cmd::bufcmd;
use shlex;
use std::env;
use std::mem;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::editor::buffer_editor::BufferEditor;
use crate::editor::terminal::Terminal;
use crate::process;
use crate::process::builtin::map::BuiltinMap;
use crate::store::buffer_store::BufferStore;

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
    #[cfg(test)]
    opened_buffers: Vec<String>,
    #[cfg(test)]
    force_quit_all: bool,
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
            #[cfg(test)]
            opened_buffers: Vec::new(),
            #[cfg(test)]
            force_quit_all: false,
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
                let _ = self.run_buffer_session();
                ControlFlow::CONTINUE
            }
        }
    }

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

    #[cfg(not(test))]
    fn run_buffer_session(&mut self) -> bool {
        if let ShellMode::Buffer(buffer_name) = mem::replace(&mut self.mode, ShellMode::Prompt) {
            if std::env::var("IRIDIUM_SKIP_EDITOR").is_ok() {
                return true;
            }
            let editor = BufferEditor::instance();
            let mut editor = editor.lock().expect("buffer editor lock poisoned");
            editor.open(buffer_name);
            editor.run();
            if editor.take_quit_all_request() {
                return false;
            }
        }
        true
    }

    #[cfg(test)]
    fn run_buffer_session(&mut self) -> bool {
        if let ShellMode::Buffer(buffer_name) = mem::replace(&mut self.mode, ShellMode::Prompt) {
            self.opened_buffers.push(buffer_name);
        }
        if self.force_quit_all {
            self.force_quit_all = false;
            return false;
        }
        true
    }

    // :b [options] <values>
    fn handle_buffer_commands(&mut self, bufcmd: &str) -> ControlFlow {
        let Some(command) = bufcmd::parse(bufcmd) else {
            println!("Unknown buffer command: {bufcmd}");
            return ControlFlow::CONTINUE;
        };

        let args = command.args();
        let mut store = self.buffers.lock().expect("buffer store lock poisoned");

        if self.apply_pre_session_options(&mut store, command.pre_session_options(), args) {
            return ControlFlow::CONTINUE;
        }
        let post_session_options = command.post_session_options();

        let create_default_buffer =
            args.is_empty() && !command.post_session_options().contains(&'l');
        let buffer_targets: Vec<(String, bool)> = if args.is_empty() {
            if create_default_buffer {
                let untitled = generate_untitled_name(&store);
                vec![(untitled, true)]
            } else {
                Vec::new()
            }
        } else {
            args.iter().cloned().map(|name| (name, false)).collect()
        };

        let should_launch_editor = !buffer_targets.is_empty();

        for (name, requires_name) in &buffer_targets {
            if *requires_name {
                store.open_untitled(name.clone());
            } else {
                store.open(name.clone());
            }
        }

        drop(store);

        if should_launch_editor {
            for (buffer_name, _) in &buffer_targets {
                self.mode = ShellMode::Buffer(buffer_name.clone());
                println!(
                    "Opened buffer '{buffer_name}'. Press ':i' to enter insert mode and Ctrl+C to exit the editor."
                );
                let continue_sessions = self.run_buffer_session();
                if !continue_sessions {
                    break;
                }
            }
        }

        if !post_session_options.is_empty() {
            self.apply_post_session_options(post_session_options, args);
        }
        ControlFlow::CONTINUE
    }

    fn handle_macro_commands(&mut self, bufcmd: &str) -> ControlFlow {
        ControlFlow::CONTINUE
    }

    fn handle_pipeline_commands(&mut self, bufcmd: &str) -> ControlFlow {
        ControlFlow::CONTINUE
    }

    fn apply_pre_session_options(
        &self,
        store: &mut BufferStore,
        options: &[char],
        args: &[String],
    ) -> bool {
        let mut handled = false;

        for option in options {
            match option {
                'd' => {
                    handled = true;
                    if args.is_empty() {
                        println!(":buffer -d requires a name");
                    } else {
                        for name in args {
                            if store.remove(name) {
                                println!("Removed buffer '{name}'");
                            }
                        }
                    }
                }
                'r' => {
                    handled = true;
                    if args.len() < 2 {
                        println!(":buffer -r requires pairs of old and new names");
                        continue;
                    }

                    if args.len() % 2 != 0 {
                        println!(":buffer -r requires pairs of old and new names");
                    }

                    for pair in args.chunks(2) {
                        if pair.len() < 2 {
                            break;
                        }

                        let old_name = pair[0].as_str();
                        let new_name = pair[1].as_str();
                        let renamed = store.rename(old_name, new_name);
                        if renamed {
                            println!("Renamed buffer '{}' to '{}'", old_name, new_name);
                        } else {
                            println!("Failed to rename buffer '{}' to '{}'", old_name, new_name);
                        }
                    }
                }
                _ => {}
            }
        }

        handled
    }

    fn apply_post_session_options(&mut self, options: &[char], args: &[String]) {
        let store = self
            .buffers
            .lock()
            .expect("buffer store lock poisoned");
        for option in options {
            match option {
                'l' => {
                    if store.is_empty() {
                        println!("(no buffers)");
                    } else {
                        let names = store.list();
                        for name in &names {
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

    /// Return the names of all buffers currently tracked in the store.
    #[allow(dead_code)]
    pub fn list_buffers(&self) -> Vec<String> {
        let store = self.buffers.lock().expect("buffer store lock poisoned");
        store.list()
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

fn generate_untitled_name(store: &BufferStore) -> String {
    loop {
        let candidate = Uuid::new_v4().to_string();
        if store.get(&candidate).is_none() {
            return candidate;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    fn make_state() -> ControlState {
        ControlState {
            status: Some(0),
            builtin_map: BuiltinMap::new(),
            mode: ShellMode::Prompt,
            buffers: Arc::new(Mutex::new(BufferStore::new())),
            opened_buffers: Vec::new(),
            force_quit_all: false,
        }
    }

    #[test]
    fn opens_multiple_buffers_in_sequence() {
        let mut state = make_state();
        let flow = state.handle_buffer_commands(":b first second");

        assert_eq!(flow, ControlFlow::CONTINUE);
        assert_eq!(
            state.opened_buffers,
            vec!["first".to_string(), "second".to_string()]
        );

        let store = state.buffers.lock().expect("buffer store lock poisoned");
        let mut names = store.list();
        names.sort();
        assert_eq!(names, vec!["first".to_string(), "second".to_string()]);
    }

    #[test]
    fn opens_single_buffer() {
        let mut state = make_state();
        let flow = state.handle_buffer_commands(":b only");

        assert_eq!(flow, ControlFlow::CONTINUE);
        assert_eq!(state.opened_buffers, vec!["only".to_string()]);

        let store = state.buffers.lock().expect("buffer store lock poisoned");
        assert!(store.get("only").is_some());
    }

    #[test]
    fn opens_untitled_buffer_when_no_arguments() {
        let mut state = make_state();
        let flow = state.handle_buffer_commands(":b");

        assert_eq!(flow, ControlFlow::CONTINUE);
        assert_eq!(state.opened_buffers.len(), 1);
        let buffer_name = &state.opened_buffers[0];
        assert!(Uuid::parse_str(buffer_name).is_ok());

        let store = state.buffers.lock().unwrap();
        assert!(store.requires_name(buffer_name));
    }

    #[test]
    fn deletes_buffers_via_option() {
        let mut state = make_state();
        {
            let mut store = state.buffers.lock().unwrap();
            store.open("alpha");
            store.open("beta");
        }

        let flow = state.handle_buffer_commands(":b -d alpha gamma");

        assert_eq!(flow, ControlFlow::CONTINUE);
        assert!(state.opened_buffers.is_empty());

        let store = state.buffers.lock().unwrap();
        assert!(store.get("alpha").is_none());
        assert!(store.get("beta").is_some());
    }

    #[test]
    fn renames_buffers_via_option() {
        let mut state = make_state();
        {
            let mut store = state.buffers.lock().unwrap();
            store.open("alpha");
        }

        let flow = state.handle_buffer_commands(":b -r alpha beta");

        assert_eq!(flow, ControlFlow::CONTINUE);

        let store = state.buffers.lock().unwrap();
        assert!(store.get("beta").is_some());
        assert!(store.get("alpha").is_none());
    }

    #[test]
    fn renames_multiple_pairs_via_option() {
        let mut state = make_state();
        {
            let mut store = state.buffers.lock().unwrap();
            store.open("alpha");
            store.open("beta");
        }

        let flow = state.handle_buffer_commands(":b -r alpha gamma beta delta");

        assert_eq!(flow, ControlFlow::CONTINUE);

        let store = state.buffers.lock().unwrap();
        assert!(store.get("gamma").is_some());
        assert!(store.get("delta").is_some());
        assert!(store.get("alpha").is_none());
        assert!(store.get("beta").is_none());
    }

    #[test]
    fn list_option_does_not_create_untitled_buffer() {
        let mut state = make_state();
        let flow = state.handle_buffer_commands(":b -l");

        assert_eq!(flow, ControlFlow::CONTINUE);
        assert!(state.opened_buffers.is_empty());

        let store = state.buffers.lock().unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn list_option_leaves_existing_buffers_intact() {
        let mut state = make_state();
        {
            let mut store = state.buffers.lock().unwrap();
            store.open("alpha");
            store.open("beta");
        }

        let flow = state.handle_buffer_commands(":b -l");
        assert_eq!(flow, ControlFlow::CONTINUE);

        let store = state.buffers.lock().unwrap();
        let mut names = store.list();
        names.sort();
        assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn list_option_outputs_existing_buffers() {
        let mut state = make_state();
        {
            let mut store = state.buffers.lock().unwrap();
            store.open("alpha");
            store.open("beta");
        }

        let flow = state.handle_buffer_commands(":b -l");

        assert_eq!(flow, ControlFlow::CONTINUE);

        let store = state.buffers.lock().unwrap();
        let mut names = store.list();
        names.sort();
        assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn opens_each_named_buffer_in_list() {
        let mut state = make_state();
        let flow = state.handle_buffer_commands(":b alpha beta gamma");

        assert_eq!(flow, ControlFlow::CONTINUE);
        assert_eq!(
            state.opened_buffers,
            vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
        );

        let store = state.buffers.lock().unwrap();
        for name in ["alpha", "beta", "gamma"] {
            assert!(
                store.get(name).is_some(),
                "expected buffer {name} to be opened"
            );
        }
    }

    #[test]
    fn quit_all_stops_opening_additional_buffers() {
        let mut state = make_state();
        state.force_quit_all = true;

        let flow = state.handle_buffer_commands(":b first second");

        assert_eq!(flow, ControlFlow::CONTINUE);
        assert_eq!(state.opened_buffers, vec!["first".to_string()]);
    }
}
