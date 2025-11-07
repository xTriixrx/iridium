use iridium::store::buffer_store::BufferStore;
use std::fs;
use std::io::Read;

fn unique_temp_file() -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let unique = format!(
        "iridium_buffer_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    path.push(unique);
    path
}

#[test]
fn save_persists_buffer_contents() {
    let path = unique_temp_file();
    let path_str = path.to_string_lossy().to_string();

    let mut store = BufferStore::new();
    let buffer = store.open(path_str.clone());
    buffer.append("first line".into());
    buffer.append("second line".into());

    store.save(&path_str).expect("save should succeed");

    assert!(!store.is_dirty(&path_str));

    let mut file = fs::File::open(&path).expect("file should exist");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("should read file");

    assert_eq!(contents, "first line\nsecond line\n");

    let _ = fs::remove_file(&path);
}

#[test]
fn store_lists_buffers_and_reports_emptiness() {
    let mut store = BufferStore::new();
    assert!(store.is_empty());

    store.open("alpha");
    store.open("beta");

    let mut listed = store.list();
    listed.sort();
    assert_eq!(listed, vec!["alpha".to_string(), "beta".to_string()]);
    assert!(!store.is_empty());
}

#[test]
fn store_insert_and_delete_characters() {
    let mut store = BufferStore::new();
    store.insert_char("buf", 0, 0, 'a');
    store.insert_char("buf", 0, 1, 'b');
    store.insert_char("buf", 0, 2, 'c');

    let deleted = store
        .delete_char("buf", 0, 3)
        .expect("delete should succeed");
    assert_eq!(deleted, (0, 2));

    let buffer = store.get("buf").expect("buffer");
    assert_eq!(buffer.lines(), &["ab".to_string()]);
}

#[test]
fn store_insert_newline_and_pad_line() {
    let mut store = BufferStore::new();
    store.insert_char("buf", 0, 0, 'h');
    store.insert_char("buf", 0, 1, 'i');

    let (row, col) = store.insert_newline("buf", 0, 1);
    assert_eq!((row, col), (1, 0));

    store.pad_line("buf", 1, 3);

    let buffer = store.get("buf").expect("buffer");
    assert_eq!(buffer.lines(), &["h".to_string(), "i  ".to_string()]);
}

#[test]
fn save_if_dirty_only_writes_when_modified() {
    let path = unique_temp_file();
    let path_str = path.to_string_lossy().to_string();

    let mut store = BufferStore::new();
    let buffer = store.open(path_str.clone());
    buffer.append("line".into());

    assert!(
        store
            .save_if_dirty(&path_str)
            .expect("save_if_dirty should succeed")
    );
    assert!(!store.is_dirty(&path_str));

    assert!(
        !store
            .save_if_dirty(&path_str)
            .expect("save_if_dirty should succeed")
    );

    let _ = fs::remove_file(&path);
}

#[test]
fn save_all_does_not_create_files_for_clean_buffers() {
    let path = unique_temp_file();
    let path_str = path.to_string_lossy().to_string();

    let mut store = BufferStore::new();
    store.open(path_str.clone());

    store
        .save_all()
        .expect("save_all should succeed for clean buffers");

    assert!(
        !path.exists(),
        "save_all should not create files for clean buffers"
    );
}
