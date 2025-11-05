use iridium::process::alias::AliasSink;
use iridium::process::builtin::map::BuiltinMap;
use std::cell::RefCell;
use std::rc::Rc;

fn configure_alias_io(map: &BuiltinMap) -> (Rc<RefCell<Vec<u8>>>, Rc<RefCell<Vec<u8>>>) {
    let stdout_buffer = Rc::new(RefCell::new(Vec::new()));
    let stderr_buffer = Rc::new(RefCell::new(Vec::new()));
    let handle = map.get_alias();
    {
        let mut alias = handle.borrow_mut();
        alias.set_sinks(
            AliasSink::Buffer(stdout_buffer.clone()),
            AliasSink::Buffer(stderr_buffer.clone()),
        );
    }
    (stdout_buffer, stderr_buffer)
}

fn invoke_alias(map: &BuiltinMap, args: &[&str]) -> Option<i32> {
    let owned_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    map.invoke("alias", &owned_args)
        .expect("alias builtin not registered")
}

fn buffer_to_string(buffer: &Rc<RefCell<Vec<u8>>>) -> String {
    String::from_utf8(buffer.borrow().clone()).unwrap()
}

#[test]
fn lists_all_aliases_when_no_operands() {
    let map = BuiltinMap::new();
    let (stdout, stderr) = configure_alias_io(&map);

    assert_eq!(invoke_alias(&map, &["ll=ls -al"]), Some(0));
    assert_eq!(invoke_alias(&map, &["gs=git status"]), Some(0));
    stdout.borrow_mut().clear();
    stderr.borrow_mut().clear();

    assert_eq!(invoke_alias(&map, &[]), Some(0));
    assert_eq!(
        buffer_to_string(&stdout),
        "alias gs='git status'\nalias ll='ls -al'\n"
    );
    assert!(buffer_to_string(&stderr).is_empty());
}

#[test]
fn queries_specific_aliases_and_reports_missing() {
    let map = BuiltinMap::new();
    let (stdout, stderr) = configure_alias_io(&map);

    assert_eq!(invoke_alias(&map, &["ll=ls -al"]), Some(0));
    stdout.borrow_mut().clear();

    assert_eq!(invoke_alias(&map, &["ll"]), Some(0));
    assert_eq!(buffer_to_string(&stdout), "alias ll='ls -al'\n");
    stdout.borrow_mut().clear();

    assert_eq!(invoke_alias(&map, &["missing"]), Some(1));
    assert_eq!(buffer_to_string(&stdout), "");
    assert_eq!(buffer_to_string(&stderr), "alias: missing: not found\n");
}

#[test]
fn alias_contains_alias_via_map() {
    let map = BuiltinMap::new();
    let (stdout, _stderr) = configure_alias_io(&map);
    assert_eq!(invoke_alias(&map, &["ll=ls"]), Some(0));
    let contains = map.get_alias().borrow().contains_alias("ll");
    assert!(contains);
    assert!(buffer_to_string(&stdout).is_empty());
}

#[test]
fn alias_get_alias_expansion_via_map() {
    let map = BuiltinMap::new();
    let (_stdout, _stderr) = configure_alias_io(&map);
    assert_eq!(invoke_alias(&map, &["ll=ls"]), Some(0));
    let expansion = map.get_alias().borrow().get_alias_expansion("ll").cloned();
    assert_eq!(expansion.as_deref(), Some("ls"));
}

#[test]
fn alias_sink_default_is_stdout() {
    assert!(matches!(AliasSink::default(), AliasSink::Stdout));
}

#[test]
fn alias_writes_results_to_stdout_and_stderr() {
    let map = BuiltinMap::new();
    let (stdout, stderr) = configure_alias_io(&map);

    assert_eq!(invoke_alias(&map, &["ll=ls -al"]), Some(0));
    stdout.borrow_mut().clear();
    stderr.borrow_mut().clear();

    assert_eq!(invoke_alias(&map, &["ll", "missing"]), Some(1));

    assert_eq!(buffer_to_string(&stdout), "alias ll='ls -al'\n");
    assert_eq!(buffer_to_string(&stderr), "alias: missing: not found\n");
}

#[test]
fn invalid_option_returns_error_status() {
    let map = BuiltinMap::new();
    let (stdout, stderr) = configure_alias_io(&map);

    assert_eq!(invoke_alias(&map, &["-x"]), Some(1));
    assert_eq!(buffer_to_string(&stdout), "");
    assert_eq!(buffer_to_string(&stderr), "alias: -x: invalid option\n");
}

#[test]
fn dash_p_is_rejected() {
    let map = BuiltinMap::new();
    let (stdout, stderr) = configure_alias_io(&map);

    assert_eq!(invoke_alias(&map, &["-p"]), Some(1));
    assert_eq!(buffer_to_string(&stdout), "");
    assert_eq!(buffer_to_string(&stderr), "alias: -p: invalid option\n");
}

#[test]
fn default_sinks_write_to_standard_streams_without_override() {
    let map = BuiltinMap::new();
    // Define aliases while sinks are still the defaults.
    assert_eq!(invoke_alias(&map, &["ll=ls -al"]), Some(0));
    assert_eq!(invoke_alias(&map, &["gs=git status"]), Some(0));

    // Listing all aliases should print via the default stdout sink.
    assert_eq!(invoke_alias(&map, &[]), Some(0));

    // Querying an unknown alias triggers the stderr sink.
    assert_eq!(invoke_alias(&map, &["unknown"]), Some(1));
}
