use crate::process::builtin::Builtin;
use terminal_size::{Width, terminal_size};

/// Builtin responsible for rendering the startup banner.
pub struct Welcome {}

impl Builtin for Welcome {
    /// Delegate to the shared `welcome` function.
    fn call(&mut self, args: &[String]) -> Option<i32> {
        welcome(args)
    }
}

impl Welcome {
    /// Construct a new welcome builtin instance.
    pub fn new() -> Self {
        Welcome {}
    }
}

/// Print the Iridium banner, centring it using the terminal width when available.
pub fn welcome(_args: &[String]) -> Option<i32> {
    const DEFAULT_WIDTH: usize = 80;
    let width = terminal_size()
        .and_then(|(Width(w), _)| usize::try_from(w).ok())
        .filter(|w| *w > 0)
        .unwrap_or(DEFAULT_WIDTH);

    let heading = [
        " _       __     __                              __      ",
        "| |     / /__  / /________  ____ ___  ___      / /_____ ",
        "| | /| / / _ \\/ / ___/ __ \\/ __ `__ \\/ _ \\    / __/ __ \\",
        "| |/ |/ /  __/ / /__/ /_/ / / / / / /  __/   / /_/ /_/ /",
        "|__/|__/\\___/_/\\___/\\____/_/ /_/ /_/\\___/    \\__/\\____/ ",
    ];

    let iridium_msg = [
        "   ██▓ ██▀███   ██▓▓█████▄  ██▓ █    ██  ███▄ ▄███▓",
        "  ▓██▒▓██ ▒ ██▒▓██▒▒██▀ ██▌▓██▒ ██  ▓██▒▓██▒▀█▀ ██▒",
        "  ▒██▒▓██ ░▄█ ▒▒██▒░██   █▌▒██▒▓██  ▒██░▓██    ▓██░",
        "  ░██░▒██▀▀█▄  ░██░░▓█▄   ▌░██░▓▓█  ░██░▒██    ▒██ ",
        "  ░██░░██▓ ▒██▒░██░░▒████▓ ░██░▒▒█████▓ ▒██▒   ░██▒",
        "  ░▓  ░ ▒▓ ░▒▓░░▓   ▒▒▓  ▒ ░▓  ░▒▓▒ ▒ ▒ ░ ▒░   ░  ░",
        "   ▒ ░  ░▒ ░ ▒░ ▒ ░ ░ ▒  ▒  ▒ ░░░▒░ ░ ░ ░  ░      ░",
        "   ▒ ░  ░░   ░  ▒ ░ ░ ░  ░  ▒ ░ ░░░ ░ ░ ░      ░   ",
        "  ░     ░      ░     ░     ░     ░            ░    ",
        "                    ░                              ",
    ];

    let purple_text = "\u{1b}[35m";
    let end_color_text = "\u{1b}[39m";

    for line in heading {
        println!("{}", center_line(line, width));
    }
    println!();
    for line in iridium_msg {
        let padded_line = center_line(line, width);
        println!("{}{}{}", purple_text, padded_line, end_color_text);
    }

    Some(0)
}

/// Centre a single line of text within the provided width.
fn center_line(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if width <= len {
        return text.to_string();
    }
    let padding = (width - len) / 2;
    let mut line = String::with_capacity(padding + len);
    line.extend(std::iter::repeat(' ').take(padding));
    line.push_str(text);
    line
}
