use iridium::control_state::{ControlFlow, ControlState};
use uuid::Uuid;

fn headless_state() -> ControlState {
    unsafe {
        std::env::set_var("IRIDIUM_SKIP_EDITOR", "1");
    }
    ControlState::new()
}

fn sorted_names(state: &ControlState) -> Vec<String> {
    let mut names = state.list_buffers();
    names.sort();
    names
}

#[test]
fn prompt_opens_default_buffer() {
    let mut state = headless_state();
    let flow = state.handle_line(":b");

    assert_eq!(flow, ControlFlow::CONTINUE);
    let names = sorted_names(&state);
    assert_eq!(names.len(), 1);
    assert!(Uuid::parse_str(&names[0]).is_ok());
}

#[test]
fn list_option_does_not_create_buffer() {
    let mut state = headless_state();
    let flow = state.handle_line(":b -l");

    assert_eq!(flow, ControlFlow::CONTINUE);
    assert!(state.list_buffers().is_empty());
}

#[test]
fn delete_option_removes_multiple_buffers() {
    let mut state = headless_state();
    let _ = state.handle_line(":b alpha beta gamma");

    let flow = state.handle_line(":b -d alpha gamma");
    assert_eq!(flow, ControlFlow::CONTINUE);

    assert_eq!(sorted_names(&state), vec!["beta".to_string()]);
}

#[test]
fn rename_option_renames_multiple_pairs() {
    let mut state = headless_state();
    let _ = state.handle_line(":b alpha beta");

    let flow = state.handle_line(":b -r alpha gamma beta delta");
    assert_eq!(flow, ControlFlow::CONTINUE);

    let names = sorted_names(&state);
    assert_eq!(names, vec!["delta".to_string(), "gamma".to_string()]);
}
