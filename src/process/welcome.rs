use crate::process::builtin::Builtin;

pub struct Welcome {

}

impl Builtin for Welcome {
    fn call(&mut self, args: &[String]) -> Option<i32> {
        welcome(args)
    }
}

impl Welcome {
    pub fn new() -> Self {
        Welcome {

        }
    }
}

pub fn welcome(_args: &[String]) -> Option<i32> {
    let purple_text = "\u{1b}[35m";
    let end_color_text = "\u{1b}[39m";
    let mut title = String::from("");

    // Welcome to title sign
    title.push_str(" _       __     __                              __      \n");
    title.push_str("| |     / /__  / /________  ____ ___  ___      / /_____ \n");
    title.push_str("| | /| / / _ \\/ / ___/ __ \\/ __ `__ \\/ _ \\    / __/ __ \\\n");
    title.push_str("| |/ |/ /  __/ / /__/ /_/ / / / / / /  __/   / /_/ /_/ /\n");
    title.push_str("|__/|__/\\___/_/\\___/\\____/_/ /_/ /_/\\___/    \\__/\\____/ \n\n");

    
    title.push_str(purple_text);

    title.push_str("   ██▓ ██▀███   ██▓▓█████▄  ██▓ █    ██  ███▄ ▄███▓\n");
    title.push_str("  ▓██▒▓██ ▒ ██▒▓██▒▒██▀ ██▌▓██▒ ██  ▓██▒▓██▒▀█▀ ██▒\n");
    title.push_str("  ▒██▒▓██ ░▄█ ▒▒██▒░██   █▌▒██▒▓██  ▒██░▓██    ▓██░\n");
    title.push_str("  ░██░▒██▀▀█▄  ░██░░▓█▄   ▌░██░▓▓█  ░██░▒██    ▒██ \n");
    title.push_str("  ░██░░██▓ ▒██▒░██░░▒████▓ ░██░▒▒█████▓ ▒██▒   ░██▒\n");
    title.push_str("  ░▓  ░ ▒▓ ░▒▓░░▓   ▒▒▓  ▒ ░▓  ░▒▓▒ ▒ ▒ ░ ▒░   ░  ░\n");
    title.push_str("   ▒ ░  ░▒ ░ ▒░ ▒ ░ ░ ▒  ▒  ▒ ░░░▒░ ░ ░ ░  ░      ░\n");
    title.push_str("   ▒ ░  ░░   ░  ▒ ░ ░ ░  ░  ▒ ░ ░░░ ░ ░ ░      ░   \n");
    title.push_str("  ░     ░      ░     ░     ░     ░            ░    \n");
    title.push_str("                    ░                              \n");
    
    title.push_str(end_color_text);

    println!("{}", title);
    return Some(0);
}