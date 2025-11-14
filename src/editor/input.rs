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
    Navigation(NavigationCommand),
    UpdateCommandBuffer(String),
    ExecuteCommand(String),
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationCommand {
    LineStart,
    LineEnd,
    PageStart,
    PageEnd,
    WordLeft,
    WordRight,
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
        _mode: &EditorMode,
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

                if shift_alt_combo(*modifiers) {
                    if let Some(action) = navigation_action_for_key(*code) {
                        return Some(InputAction::Navigation(action));
                    }
                }

                if alt_word_combo(*modifiers) {
                    if let Some(action) = alt_word_navigation(*code) {
                        return Some(InputAction::Navigation(action));
                    }
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

fn navigation_action_for_key(code: KeyCode) -> Option<NavigationCommand> {
    match code {
        KeyCode::Left => Some(NavigationCommand::LineStart),
        KeyCode::Right => Some(NavigationCommand::LineEnd),
        KeyCode::Up => Some(NavigationCommand::PageStart),
        KeyCode::Down => Some(NavigationCommand::PageEnd),
        _ => None,
    }
}

fn alt_word_navigation(code: KeyCode) -> Option<NavigationCommand> {
    match code {
        KeyCode::Char('b') | KeyCode::Char('B') => Some(NavigationCommand::WordLeft),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(NavigationCommand::WordRight),
        _ => None,
    }
}

fn shift_alt_combo(modifiers: KeyModifiers) -> bool {
    modifiers.contains(KeyModifiers::SHIFT)
        && modifiers.contains(KeyModifiers::ALT)
        && !modifiers.contains(KeyModifiers::CONTROL)
}

fn alt_word_combo(modifiers: KeyModifiers) -> bool {
    modifiers.contains(KeyModifiers::ALT) && !modifiers.contains(KeyModifiers::CONTROL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    fn shift_alt_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::SHIFT | KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn alt_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn shift_alt_right_enters_navigation_line_end() {
        let mut handler = InputHandler::new();
        let action = handler.process(&shift_alt_event(KeyCode::Right), &EditorMode::Read, false);
        assert_eq!(
            action,
            Some(InputAction::Navigation(NavigationCommand::LineEnd))
        );
    }

    #[test]
    fn shift_alt_left_enters_navigation_line_start() {
        let mut handler = InputHandler::new();
        let action = handler.process(&shift_alt_event(KeyCode::Left), &EditorMode::Read, false);
        assert_eq!(
            action,
            Some(InputAction::Navigation(NavigationCommand::LineStart))
        );
    }

    #[test]
    fn shift_alt_up_enters_navigation_page_start() {
        let mut handler = InputHandler::new();
        let action = handler.process(&shift_alt_event(KeyCode::Up), &EditorMode::Read, false);
        assert_eq!(
            action,
            Some(InputAction::Navigation(NavigationCommand::PageStart))
        );
    }

    #[test]
    fn shift_alt_down_enters_navigation_page_end() {
        let mut handler = InputHandler::new();
        let action = handler.process(&shift_alt_event(KeyCode::Down), &EditorMode::Read, false);
        assert_eq!(
            action,
            Some(InputAction::Navigation(NavigationCommand::PageEnd))
        );
    }

    #[test]
    fn alt_b_enters_navigation_word_left() {
        let mut handler = InputHandler::new();
        let action = handler.process(&alt_event(KeyCode::Char('b')), &EditorMode::Read, false);
        assert_eq!(
            action,
            Some(InputAction::Navigation(NavigationCommand::WordLeft))
        );
    }

    #[test]
    fn alt_f_enters_navigation_word_right() {
        let mut handler = InputHandler::new();
        let action = handler.process(&alt_event(KeyCode::Char('f')), &EditorMode::Read, false);
        assert_eq!(
            action,
            Some(InputAction::Navigation(NavigationCommand::WordRight))
        );
    }
}
