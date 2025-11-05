use crate::editor::buffer_editor::EditorMode;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    EnterInsertMode,
    EnterCommandMode,
    EnterPreviousMode,
    ExitInsertMode,
    InsertChar(char),
    DeleteChar,
    InsertNewLine,
    MoveCursor(KeyCode),
    UpdateCommandBuffer(String),
    ExecuteCommand(String),
    Quit,
}

#[derive(Debug, Default, Clone)]
pub struct InputHandler {
    colon_buffer: Option<String>,
}

impl InputHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(
        &mut self,
        event: &Event,
        mode: &EditorMode,
        in_insert_mode: bool,
    ) -> Option<InputAction> {
        match event {
            Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                ..
            }) => {
                if *modifiers == KeyModifiers::CONTROL && matches!(code, KeyCode::Char('c')) {
                    return Some(InputAction::Quit);
                }

                if self.colon_buffer.is_none() && matches!(code, KeyCode::Char(':')) {
                    self.colon_buffer = Some(String::new());
                    return Some(InputAction::EnterCommandMode);
                }

                if let Some(buffer) = &mut self.colon_buffer {
                    match code {
                        KeyCode::Esc => {
                            self.reset_colon();
                            return Some(InputAction::ExitInsertMode);
                        }
                        KeyCode::Backspace => {
                            let _ = buffer.pop();
                            if buffer.is_empty() {
                                self.reset_colon();
                                return Some(InputAction::EnterPreviousMode);
                            }
                            return Some(InputAction::UpdateCommandBuffer(buffer.clone()));
                        }
                        KeyCode::Enter => {
                            let command = buffer.clone();
                            self.reset_colon();
                            if command.is_empty() {
                                return Some(InputAction::ExitInsertMode);
                            }
                            return Some(InputAction::ExecuteCommand(command));
                        }
                        KeyCode::Char(ch) => {
                            buffer.push(*ch);
                            return Some(InputAction::UpdateCommandBuffer(buffer.clone()));
                        }
                        _ => {
                            self.reset_colon();
                            return None;
                        }
                    }
                }

                match code {
                    KeyCode::Esc if in_insert_mode => Some(InputAction::ExitInsertMode),
                    KeyCode::Backspace if in_insert_mode => Some(InputAction::DeleteChar),
                    KeyCode::Enter if in_insert_mode => Some(InputAction::InsertNewLine),
                    KeyCode::Char(ch) if in_insert_mode => Some(InputAction::InsertChar(*ch)),
                    KeyCode::Enter if in_insert_mode => None,
                    KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Home
                    | KeyCode::End
                    | KeyCode::PageUp
                    | KeyCode::PageDown => Some(InputAction::MoveCursor(*code)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn reset_colon(&mut self) {
        self.colon_buffer = None;
    }
}
