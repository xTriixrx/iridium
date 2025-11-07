//! Runs the interactive command loop and hooks up the line editor.

use crate::complete::handler::TabEventHandler;
use crate::complete::helper::IridiumHelper;
use crate::complete::hinter::CompleteHintHandler;
use crate::complete::history::load_history_entries;
use crate::control_state::ControlFlow;
use crate::control_state::ControlState;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Cmd, Editor, Event, EventHandler, KeyEvent, Result, hint::HistoryHinter};
use std::io::{self, Write};

/// Run the interactive shell loop, handling input, history, and control flow.
#[doc(hidden)]
pub trait ControlSession {
    fn prompt(&self) -> String;
    fn handle_line(&mut self, line: &str) -> ControlFlow;
}

impl ControlSession for ControlState {
    fn prompt(&self) -> String {
        ControlState::prompt(self)
    }

    fn handle_line(&mut self, line: &str) -> ControlFlow {
        ControlState::handle_line(self, line)
    }
}

#[doc(hidden)]
pub trait LineEditor {
    fn readline(&mut self, prompt: &str) -> std::result::Result<String, ReadlineError>;
    fn add_history_entry(&mut self, entry: &str) -> rustyline::Result<bool>;
}

impl LineEditor for Editor<IridiumHelper, DefaultHistory> {
    fn readline(&mut self, prompt: &str) -> std::result::Result<String, ReadlineError> {
        Editor::readline(self, prompt)
    }

    fn add_history_entry(&mut self, entry: &str) -> rustyline::Result<bool> {
        Editor::add_history_entry(self, entry)
    }
}

pub fn control_loop() -> Result<()> {
    let mut stdout = io::stdout();
    let mut control_state = ControlState::new();
    let mut rl = Editor::<IridiumHelper, DefaultHistory>::new()?;

    // Set the custom helper callback
    rl.set_helper(Some(IridiumHelper::new(HistoryHinter::new())));

    // Loads iridium history file into context
    load_history(&mut rl);

    // Binds hinter & tab completion to key events
    bind_handlers(&mut rl);

    run_loop_with_editor(&mut control_state, &mut rl, &mut stdout)
}

/// Attach custom completion and hint handlers to the readline editor.
fn bind_handlers(rl: &mut Editor<IridiumHelper, DefaultHistory>) {
    let ceh = Box::new(CompleteHintHandler::new());

    //
    rl.bind_sequence(KeyEvent::ctrl('b'), EventHandler::Conditional(ceh.clone()));

    //
    rl.bind_sequence(KeyEvent::alt('f'), EventHandler::Conditional(ceh));

    //
    rl.bind_sequence(
        KeyEvent::from('\t'),
        EventHandler::Conditional(Box::new(TabEventHandler::new())),
    );

    //
    rl.bind_sequence(
        Event::KeySeq(vec![KeyEvent::ctrl('X'), KeyEvent::ctrl('E')]),
        EventHandler::Simple(Cmd::Suspend),
    );
}

/// Load persisted history entries and replay them into the editor state.
fn load_history(rl: &mut Editor<IridiumHelper, DefaultHistory>) {
    match load_history_entries(None) {
        Ok(history) => {
            for entry in history {
                if let Err(err) = rl.add_history_entry(entry.as_str()) {
                    eprintln!("Warning: unable to record persisted history entry: {err}");
                }
            }
        }
        Err(err) => {
            eprintln!("Warning: unable to load history hints: {err}");
        }
    }
}

#[doc(hidden)]
pub fn run_loop_with_editor<C, E, W>(
    control_state: &mut C,
    rl: &mut E,
    stdout: &mut W,
) -> Result<()>
where
    C: ControlSession,
    E: LineEditor,
    W: Write,
{
    loop {
        let prompt = control_state.prompt();
        stdout.flush()?;

        match rl.readline(&prompt) {
            Ok(line) => {
                if !line.is_empty() {
                    if let Err(err) = rl.add_history_entry(line.as_str()) {
                        eprintln!("Warning: unable to record line in history: {err}");
                    }
                }

                if let ControlFlow::EXIT = control_state.handle_line(&line) {
                    break;
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("Error: {err}");
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyline::Editor;
    use rustyline::history::History;
    use std::collections::VecDeque;
    use std::env;
    use std::fs;
    use std::io::{self, Cursor};
    use std::path::PathBuf;
    use uuid::Uuid;

    struct MockControl {
        lines: Vec<String>,
        exit_after: Option<usize>,
    }

    impl MockControl {
        fn new(exit_after: Option<usize>) -> Self {
            Self {
                lines: Vec::new(),
                exit_after,
            }
        }
    }

    impl ControlSession for MockControl {
        fn prompt(&self) -> String {
            format!("mock-prompt#{}", self.lines.len())
        }

        fn handle_line(&mut self, line: &str) -> ControlFlow {
            self.lines.push(line.to_string());
            if let Some(limit) = self.exit_after {
                if self.lines.len() >= limit {
                    return ControlFlow::EXIT;
                }
            }
            ControlFlow::CONTINUE
        }
    }

    enum Response {
        Line(String),
        Interrupted,
        Eof,
        Error(ReadlineError),
    }

    struct MockEditor {
        responses: VecDeque<Response>,
        history: Vec<String>,
        fail_history: bool,
    }

    impl MockEditor {
        fn new(responses: Vec<Response>) -> Self {
            Self {
                responses: responses.into(),
                history: Vec::new(),
                fail_history: false,
            }
        }

        fn with_failures(responses: Vec<Response>) -> Self {
            Self {
                responses: responses.into(),
                history: Vec::new(),
                fail_history: true,
            }
        }
    }

    impl LineEditor for MockEditor {
        fn readline(&mut self, _: &str) -> std::result::Result<String, ReadlineError> {
            match self.responses.pop_front().unwrap_or(Response::Eof) {
                Response::Line(value) => Ok(value),
                Response::Interrupted => Err(ReadlineError::Interrupted),
                Response::Eof => Err(ReadlineError::Eof),
                Response::Error(err) => Err(err),
            }
        }

        fn add_history_entry(&mut self, entry: &str) -> rustyline::Result<bool> {
            if self.fail_history {
                return Err(ReadlineError::Io(io::Error::new(
                    io::ErrorKind::Other,
                    "history failure",
                )));
            }
            self.history.push(entry.to_string());
            Ok(true)
        }
    }

    fn set_home(dir: &PathBuf) -> Option<String> {
        let previous = env::var("HOME").ok();
        unsafe {
            env::set_var("HOME", dir);
        }
        previous
    }

    #[test]
    fn bind_handlers_sets_key_sequences() {
        let mut editor = Editor::<IridiumHelper, DefaultHistory>::new().unwrap();
        bind_handlers(&mut editor);
    }

    #[test]
    fn load_history_replays_entries() {
        let temp_dir = env::temp_dir().join(format!("iridium_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let history_path = temp_dir.join(".iridium_history");
        fs::write(&history_path, "123:0:first\n124:0:second\n").unwrap();
        let prev_home = set_home(&temp_dir);

        let mut editor = Editor::<IridiumHelper, DefaultHistory>::new().unwrap();
        load_history(&mut editor);
        assert_eq!(editor.history().len(), 2);

        if let Some(home) = prev_home {
            unsafe {
                env::set_var("HOME", home);
            }
        }
    }

    #[test]
    fn load_history_handles_error() {
        let temp_dir = env::temp_dir().join(format!("iridium_test_{}", Uuid::new_v4()));
        fs::create_dir_all(temp_dir.join(".iridium_history")).unwrap();
        let prev_home = set_home(&temp_dir);

        let mut editor = Editor::<IridiumHelper, DefaultHistory>::new().unwrap();
        load_history(&mut editor);
        assert_eq!(editor.history().len(), 0);

        if let Some(home) = prev_home {
            unsafe {
                env::set_var("HOME", home);
            }
        }
    }

    #[test]
    fn loop_records_history_entries() {
        let mut control = MockControl::new(None);
        let mut editor = MockEditor::new(vec![Response::Line("cmd".into()), Response::Eof]);
        let mut sink = Cursor::new(Vec::new());

        run_loop_with_editor(&mut control, &mut editor, &mut sink).unwrap();

        assert_eq!(control.lines, vec!["cmd".to_string()]);
        assert_eq!(editor.history, vec!["cmd".to_string()]);
    }

    #[test]
    fn loop_stops_when_control_requests_exit() {
        let mut control = MockControl::new(Some(1));
        let mut editor = MockEditor::new(vec![
            Response::Line("first".into()),
            Response::Line("second".into()),
        ]);
        let mut sink = Cursor::new(Vec::new());

        run_loop_with_editor(&mut control, &mut editor, &mut sink).unwrap();

        assert_eq!(control.lines, vec!["first".to_string()]);
        assert_eq!(editor.history, vec!["first".to_string()]);
    }

    #[test]
    fn loop_handles_interrupts() {
        let mut control = MockControl::new(None);
        let mut editor = MockEditor::new(vec![Response::Interrupted]);
        let mut sink = Cursor::new(Vec::new());

        run_loop_with_editor(&mut control, &mut editor, &mut sink).unwrap();

        assert!(control.lines.is_empty());
        assert!(editor.history.is_empty());
    }

    #[test]
    fn loop_handles_errors() {
        let mut control = MockControl::new(None);
        let mut editor = MockEditor::new(vec![Response::Error(ReadlineError::Io(io::Error::new(
            io::ErrorKind::Other,
            "boom",
        )))]);
        let mut sink = Cursor::new(Vec::new());

        run_loop_with_editor(&mut control, &mut editor, &mut sink).unwrap();

        assert!(control.lines.is_empty());
        assert!(editor.history.is_empty());
    }

    #[test]
    fn loop_warns_when_history_addition_fails() {
        let mut control = MockControl::new(Some(1));
        let mut editor = MockEditor::with_failures(vec![Response::Line("cmd".into())]);
        let mut sink = Cursor::new(Vec::new());

        run_loop_with_editor(&mut control, &mut editor, &mut sink).unwrap();

        assert!(editor.history.is_empty());
        assert_eq!(control.lines.len(), 1);
    }
}
