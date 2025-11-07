use iridium::editor::buffer_editor::BufferEditor;
use iridium::editor::terminal::Terminal;
use iridium::store::buffer_store::BufferStore;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};

fn reset_store() -> Arc<Mutex<BufferStore>> {
    unsafe {
        std::env::set_var("IRIDIUM_SKIP_EDITOR", "1");
    }

    let terminal = Terminal::instance();
    let candidate = Arc::new(Mutex::new(BufferStore::new()));
    terminal.attach_store(Arc::clone(&candidate));
    let handle = terminal.store_handle();
    {
        let mut store = handle.lock().unwrap();
        *store = BufferStore::new();
    }

    handle
}

#[test]
fn quit_all_now_succeeds_for_named_buffer() {
    let handle = reset_store();
    {
        let mut store = handle.lock().unwrap();
        store.open("alpha");
    }

    let mut editor = BufferEditor::new("alpha");
    editor.open("alpha");

    editor
        .quit_all_now()
        .expect("named buffer should quit without prompt");
    assert!(editor.take_quit_all_request());
}

#[test]
fn quit_all_now_requires_name_for_untitled_buffer() {
    let handle = reset_store();
    {
        let mut store = handle.lock().unwrap();
        store.open_untitled("Untitled-1");
    }

    let mut editor = BufferEditor::new("Untitled-1");
    editor.open("Untitled-1");

    let err = editor
        .quit_all_now()
        .expect_err("untitled buffers must be named before quitting");
    assert_eq!(err.kind(), ErrorKind::Other);

    {
        let mut store = handle.lock().unwrap();
        store.rename("Untitled-1", "named");
    }

    editor.open("named");
    editor
        .quit_all_now()
        .expect("once named the buffer can quit all");
    assert!(editor.take_quit_all_request());
}
