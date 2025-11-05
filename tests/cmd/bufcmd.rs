use iridium::cmd::bufcmd;

#[test]
fn rejects_non_buffer_commands() {
    assert!(bufcmd::parse("").is_none());
    assert!(bufcmd::parse("   ").is_none());
    assert!(bufcmd::parse(":x -l file").is_none());
}

#[test]
fn parses_grouped_short_options_in_order() {
    let command = bufcmd::parse(":b -ab file1 file2").expect("expected to parse :b command");

    let expected_args = vec![String::from("file1"), String::from("file2")];

    assert!(command.pre_session_options().is_empty());
    assert_eq!(command.post_session_options(), &['a', 'b']);
    assert_eq!(command.args(), expected_args.as_slice());
}

#[test]
fn treats_double_dash_tokens_as_arguments() {
    let command = bufcmd::parse(":b -l -- --literal").expect("expected to parse :b command");

    let expected_args = vec![String::from("--"), String::from("--literal")];

    assert_eq!(command.post_session_options(), &['l']);
    assert_eq!(command.args(), expected_args.as_slice());
}

#[test]
fn supports_quoted_arguments() {
    let command = bufcmd::parse(":b \"file name.txt\" 'another file'")
        .expect("expected to parse quoted :b command");

    let expected_args = vec![String::from("file name.txt"), String::from("another file")];

    assert!(command.post_session_options().is_empty());
    assert_eq!(command.args(), expected_args.as_slice());
}

#[test]
fn rejects_unterminated_quote() {
    assert!(bufcmd::parse(":b \"unterminated").is_none());
}
