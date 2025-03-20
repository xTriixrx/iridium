
use shlex;
use std::env;
use crate::process;
use std::error::Error;
use std::io::{self, Write};
use rustyline::{self, DefaultEditor};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::process::builtin::map::BuiltinMap;

pub fn control_loop() -> Result<(),  Box<dyn Error>> {
    let mut status: Option<i32> = Some(0);
    let mut stdout = io::stdout();
    let mut rustyline = DefaultEditor::new().unwrap();

    let mut builtin_map = BuiltinMap::new();
    builtin_map.populate_func_map();
    
    // Main command control loop for processing commands
    loop {
        let prompt = generate_prompt(status);
        let _ = stdout.flush();
        
        let readline = rustyline.readline(&prompt);

        match readline {
            Ok(line) => {
                let mut tokens = parse_tokens(&line);
                tokens = alias_parser(&mut builtin_map, tokens);
                
                let unix_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

                // println!("Tokens: {:?}", tokens);
                status = process::execute(&mut builtin_map, &tokens);

                if status == Some(process::exit::EXIT_CODE) {
                    return Ok(());
                }

                // Append executed line to end of history
                if !line.is_empty() {
                    process::history::append_history(unix_timestamp, status, &line);
                }
            },
            Err(rustyline::error::ReadlineError::Interrupted) => {
                break;
            },
            Err(rustyline::error::ReadlineError::Eof) => {
                break;
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

fn alias_parser(builtin_map: &mut BuiltinMap, tokens: Vec<String>) -> Vec<String> {
    let aliases = builtin_map.get_alias();
    let aliases_borrow = aliases.as_ref().borrow();
    let alias = tokens.join(" ");

    // Determine if command is an alias, and call alias
    if aliases_borrow.contains_alias(&alias) {
        let expansion = aliases_borrow.get_alias_expansion(&alias).unwrap();
        return parse_tokens(expansion);
    }

    tokens
}

fn generate_prompt(status: Option<i32>) -> String {
    let arrow = 0x27A3;
    let red_text = "\u{1b}[31m";
    let green_text = "\u{1b}[32m";
    let purple_text = "\u{1b}[35m";
    let end_color_text = "\u{1b}[39m";
    
    let cwd = env::current_dir()
        .expect("Expected to retrieve current path, aborting now.");

    let prompt = format!("{}{}{}{}{}{}{}{}",
    purple_text,
    update_cwd(cwd.to_str().expect("Expected a string slice for current path, aborting now")),
    match char::from_u32(0x0020) {
        Some(space) => space,
        None => ' ',
    },
    end_color_text,
    match status {
        Some(0) => green_text,
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

    return prompt;
}

fn update_cwd(cwd: &str) -> String {
    let updated_cwd = cwd.replace(&env::var("HOME")
        .expect("Expected HOME environment variable to be set, aborting now."), "~");

    return updated_cwd;
}

fn parse_tokens(line: &str) -> Vec<String> {
    match shlex::split(line) {
        Some(vec) => vec,
        None => panic!("Unable to parse string: {}", line),
    }
}