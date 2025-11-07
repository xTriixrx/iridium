use iridium::store::buffer_store::BufferStore;

#[test]
fn buffer_append_and_clear_via_store() {
    let mut store = BufferStore::new();
    let buffer = store.open("mem://buffer");

    buffer.append("one".into());
    buffer.append("two".into());
    assert_eq!(buffer.lines(), &["one".to_string(), "two".to_string()]);

    buffer.clear();
    assert!(buffer.lines().is_empty());
}

#[test]
fn buffer_remove_last_returns_last_line() {
    let mut store = BufferStore::new();
    let buffer = store.open("mem://buffer");
    buffer.append("alpha".into());
    buffer.append("beta".into());

    let removed = buffer.remove_last().expect("line should be removed");
    assert_eq!(removed, "beta");
    assert_eq!(buffer.lines(), &["alpha".to_string()]);
}

#[test]
fn buffer_persists_line_access_through_store() {
    let mut store = BufferStore::new();
    let buffer = store.open("mem://buffer");
    buffer.append("example line".into());

    let retrieved = store
        .get("mem://buffer")
        .expect("buffer should exist")
        .lines();

    assert_eq!(retrieved, &["example line".to_string()]);
}

#[test]
fn buffer_insert_and_delete_characters_via_store() {
    let mut store = BufferStore::new();
    store.insert_char("mem://buffer", 0, 0, 'a');
    store.insert_char("mem://buffer", 0, 1, 'b');
    store.insert_char("mem://buffer", 0, 2, 'c');

    store.insert_char("mem://buffer", 0, 1, 'X');

    store
        .delete_char("mem://buffer", 0, 2)
        .expect("delete succeeds");

    let buffer = store.get("mem://buffer").expect("buffer exists");
    assert_eq!(buffer.lines(), &["ac".to_string()]);
}

#[test]
fn buffer_insert_newline_and_pad_line_via_store() {
    let mut store = BufferStore::new();
    store.insert_char("mem://buffer", 0, 0, 'h');
    store.insert_char("mem://buffer", 0, 1, 'e');
    store.insert_char("mem://buffer", 0, 2, 'l');
    store.insert_char("mem://buffer", 0, 3, 'l');
    store.insert_char("mem://buffer", 0, 4, 'o');

    let (row, col) = store.insert_newline("mem://buffer", 0, 2);
    assert_eq!((row, col), (1, 0));

    store.pad_line("mem://buffer", 1, 5);

    let buffer = store.get("mem://buffer").expect("buffer");
    assert_eq!(buffer.lines(), &["he".to_string(), "llo  ".to_string()]);
}
