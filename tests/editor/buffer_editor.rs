use iridium::editor::buffer_editor::BufferEditor;
use iridium::editor::terminal::Terminal;
use iridium::store::buffer_store::BufferStore;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use uuid::Uuid;

struct StoreTestContext {
    handle: Arc<Mutex<BufferStore>>,
    _guard: MutexGuard<'static, ()>,
}

impl StoreTestContext {
    fn handle(&self) -> Arc<Mutex<BufferStore>> {
        Arc::clone(&self.handle)
    }
}

fn test_lock() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner())
}

fn reset_store() -> StoreTestContext {
    let guard = test_lock();
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

    StoreTestContext {
        handle,
        _guard: guard,
    }
}

#[test]
fn quit_all_now_succeeds_for_named_buffer() {
    let ctx = reset_store();
    let handle = ctx.handle();
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
    let ctx = reset_store();
    let handle = ctx.handle();
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

#[test]
fn jump_to_buffer_switches_named_buffer() {
    let ctx = reset_store();
    let handle = ctx.handle();
    {
        let mut store = handle.lock().unwrap();
        store.open("alpha");
        store.open("beta");
    }

    let mut editor = BufferEditor::new("alpha");
    editor.open("alpha");

    editor
        .jump_to_buffer("beta")
        .expect("should switch to beta");
    assert!(editor.prompt_string().contains("[buffer:beta]"));
}

#[test]
fn colon_q_closes_current_buffer_and_moves() {
    let ctx = reset_store();
    let handle = ctx.handle();
    {
        let mut store = handle.lock().unwrap();
        store.open("alpha");
        store.open("beta");
    }

    let mut editor = BufferEditor::new("alpha");
    editor.open("alpha");

    editor
        .execute_colon_command("q")
        .expect(":q should succeed");

    {
        let store = handle.lock().unwrap();
        let alpha = store.get("alpha").expect("alpha should remain tracked");
        assert!(!alpha.is_open(), "closed buffer should not present as open");
        let beta = store.get("beta").expect("beta should exist");
        assert!(beta.is_open());
    }

    assert!(editor.prompt_string().contains("[buffer:beta]"));
    assert!(!editor.is_quit());
}

#[test]
fn colon_q_requires_force_for_dirty_buffer() {
    let ctx = reset_store();
    let handle = ctx.handle();
    {
        let mut store = handle.lock().unwrap();
        store.open("alpha").append("dirty".into());
    }

    let mut editor = BufferEditor::new("alpha");
    editor.open("alpha");

    editor.execute_colon_command("q").expect(":q should warn");
    {
        let store = handle.lock().unwrap();
        assert!(store.get("alpha").is_some());
    }
    assert!(!editor.is_quit());

    editor
        .execute_colon_command("q!")
        .expect(":q! should force close");
    {
        let store = handle.lock().unwrap();
        let alpha = store
            .get("alpha")
            .expect("alpha should still be tracked after force close");
        assert!(!alpha.is_open());
    }
    assert!(editor.is_quit());
}

#[test]
fn colon_q_reopening_restores_clean_context() {
    let ctx = reset_store();
    let handle = ctx.handle();
    let path = temp_file_path();
    let path_str = path.to_string_lossy().to_string();
    {
        let mut store = handle.lock().unwrap();
        store.open(path_str.clone()).append("keep".into());
        store
            .save(path_str.as_str())
            .expect("saving buffer should succeed");
    }

    let mut editor = BufferEditor::new(path_str.clone());
    editor.open(path_str.clone());

    editor
        .execute_colon_command("q")
        .expect(":q should close clean buffer");
    assert!(editor.is_quit());

    {
        let store = handle.lock().unwrap();
        let buffer = store
            .get(path_str.as_str())
            .expect("buffer should remain tracked after :q");
        assert!(!buffer.is_open());
    }

    {
        let mut store = handle.lock().unwrap();
        let reopened = store.open(path_str.clone());
        assert_eq!(reopened.lines(), &["keep".to_string()]);
    }

    let _ = fs::remove_file(path);
}

#[test]
fn colon_q_bang_preserves_dirty_buffer_in_memory() {
    let ctx = reset_store();
    let handle = ctx.handle();
    {
        let mut store = handle.lock().unwrap();
        store.open("alpha").append("unsaved".into());
    }

    let mut editor = BufferEditor::new("alpha");
    editor.open("alpha");

    editor
        .execute_colon_command("q!")
        .expect(":q! should close dirty buffer");
    assert!(editor.is_quit());

    {
        let mut store = handle.lock().unwrap();
        let reopened = store.open("alpha");
        assert_eq!(reopened.lines(), &["unsaved".to_string()]);
    }
}

#[test]
fn colon_s_marks_buffer_clean_without_disk_write() {
    let ctx = reset_store();
    let handle = ctx.handle();
    {
        let mut store = handle.lock().unwrap();
        store.open("alpha").append("unsaved".into());
    }

    let mut editor = BufferEditor::new("alpha");
    editor.open("alpha");

    editor
        .execute_colon_command("s")
        .expect(":s should mark buffer clean in memory");
    {
        let store = handle.lock().unwrap();
        assert!(!store.is_dirty("alpha"));
    }

    editor
        .execute_colon_command("q")
        .expect(":q should now close clean buffer");
    assert!(editor.is_quit());
}

fn temp_file_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("iridium_test_{}.txt", Uuid::new_v4()));
    path
}

#[test]
fn write_command_flushes_to_disk() {
    let ctx = reset_store();
    let handle = ctx.handle();
    let path = temp_file_path();
    let path_str = path.to_string_lossy().to_string();
    {
        let mut store = handle.lock().unwrap();
        store.open(path_str.clone()).append("hello".into());
    }

    let mut editor = BufferEditor::new(path_str.clone());
    editor.open(path_str.clone());

    editor.execute_colon_command("w").expect(":w should write");

    let contents = fs::read_to_string(&path).expect("file should exist");
    assert!(contents.contains("hello"));

    let _ = fs::remove_file(path);
}

#[test]
fn write_quit_command_writes_and_closes() {
    let ctx = reset_store();
    let handle = ctx.handle();
    let path = temp_file_path();
    let path_str = path.to_string_lossy().to_string();
    {
        let mut store = handle.lock().unwrap();
        store.open(path_str.clone()).append("bye".into());
    }

    let mut editor = BufferEditor::new(path_str.clone());
    editor.open(path_str.clone());

    editor
        .execute_colon_command("wq")
        .expect(":wq should write and quit");

    assert!(editor.is_quit());
    let contents = fs::read_to_string(&path).expect("file should exist");
    assert!(contents.contains("bye"));

    let _ = fs::remove_file(path);
}
