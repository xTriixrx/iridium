use crate::process;

use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn control_loop() -> Result<(),  Box<dyn Error>> {
    let mut status: i32 = 0;
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut line = String::new();
    
    // Main command control loop for processing commands
    loop {
        generate_prompt(&status);
        let _ = stdout.flush();
        
        read_line(&mut line)?;
        
        let tokens = parse_tokens(&line);
        let unix_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        status = process::execute(&tokens);

        if status == process::exit::EXIT_CODE {
            return Ok(());
        }

        // Append executed line to end of history
        process::history::append_history(unix_timestamp, status, &line);

        line.clear();
    }
}

fn generate_prompt(status: &i32) {
    let arrow = 0x27A3;
    let red_text = "\u{1b}[31m";
    let green_text = "\u{1b}[32m";
    let purple_text = "\u{1b}[35m";
    let end_color_text = "\u{1b}[39m";
    
    let cwd = env::current_dir()
        .expect("Expected to retrieve current path, aborting now.");

    print!("{}{}{}{}{}{}{}{}",
    purple_text,
    update_cwd(cwd.to_str().expect("Expected a string slice for current path, aborting now")),
    match char::from_u32(0x0020) {
        Some(space) => space,
        None => ' ',
    },
    end_color_text,
    match *status {
        0 => green_text,
        _ => red_text,
    },
    match char::from_u32(arrow) {
        Some(arrow) => arrow,
        None => '>',
    },
    end_color_text,
    match char::from_u32(0x0020) {
        Some(space) => space,
        None => ' ',
    });
}

fn update_cwd(cwd: &str) -> String {
    let updated_cwd = cwd.replace(&env::var("HOME")
        .expect("Expected HOME environment variable to be set, aborting now."), "~");

    return updated_cwd;
}

fn read_line(line: &mut String) -> Result<(), Box<dyn Error>> {
    std::io::stdin().read_line(line)?; // including '\n'
    Ok(())
}

fn parse_tokens(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_string).collect()
}