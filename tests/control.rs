use iridium::control::{LineEditor, run_loop_with_editor};
use iridium::control_state::ControlState;
use rustyline::error::ReadlineError;
use std::collections::VecDeque;
use std::io::Cursor;

struct ScriptedEditor {
    responses: VecDeque<Result<String, ReadlineError>>,
    history: Vec<String>,
}

impl ScriptedEditor {
    fn new(responses: Vec<Result<String, ReadlineError>>) -> Self {
        Self {
            responses: responses.into(),
            history: Vec::new(),
        }
    }
}

impl LineEditor for ScriptedEditor {
    fn readline(&mut self, _: &str) -> Result<String, ReadlineError> {
        self.responses
            .pop_front()
            .unwrap_or_else(|| Err(ReadlineError::Eof))
    }

    fn add_history_entry(&mut self, entry: &str) -> rustyline::Result<bool> {
        self.history.push(entry.to_string());
        Ok(true)
    }
}

#[test]
fn drive_control_state_open_buffer() {
    unsafe {
        std::env::set_var("IRIDIUM_SKIP_EDITOR", "1");
    }
    let mut control_state = ControlState::new();
    let mut editor = ScriptedEditor::new(vec![Ok(":b alpha".into()), Err(ReadlineError::Eof)]);
    let mut sink = Cursor::new(Vec::new());

    run_loop_with_editor(&mut control_state, &mut editor, &mut sink).unwrap();

    let buffers = control_state.list_buffers();
    assert!(buffers.iter().any(|name| name == "alpha"));
    assert_eq!(editor.history, vec![":b alpha".to_string()]);
}

#[test]
fn drive_control_state_handles_interrupt() {
    unsafe {
        std::env::set_var("IRIDIUM_SKIP_EDITOR", "1");
    }
    let mut control_state = ControlState::new();
    let mut editor = ScriptedEditor::new(vec![Err(ReadlineError::Interrupted)]);
    let mut sink = Cursor::new(Vec::new());

    run_loop_with_editor(&mut control_state, &mut editor, &mut sink).unwrap();

    assert!(control_state.list_buffers().is_empty());
    assert!(editor.history.is_empty());
}
