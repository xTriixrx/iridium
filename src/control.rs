use std::io::{self, Write};
use rustyline::error::ReadlineError;
use crate::control_state::ControlFlow;
use crate::control_state::ControlState;
use rustyline::history::DefaultHistory;
use crate::complete::helper::IridiumHelper;
use crate::complete::handler::TabEventHandler;
use crate::complete::hinter::CompleteHintHandler;
use crate::complete::history::load_history_entries;
use rustyline::{hint::HistoryHinter, history::FileHistory};
use rustyline::{Cmd, Editor, Event, EventHandler, KeyEvent, Result};

pub fn control_loop() -> Result<()> {
    let mut stdout = io::stdout();
    let mut control_state = ControlState::new();
    let mut rl = Editor::<IridiumHelper, DefaultHistory>::new()?;
    
    // Set the custom helper callback
    rl.set_helper(Some(IridiumHelper::new(HistoryHinter::new())));

    // Loads iridium history file into context
    load_history(&mut rl);

    // Binds hinter & tab completion to key events
    bind_handlers(&mut rl);

    loop {
        let prompt = control_state.prompt();
        let _ = stdout.flush();

        match rl.readline(&prompt) {
            Ok(line) => {
                if !line.is_empty() {
                    if let Err(err) = rl.add_history_entry(line.as_str()) {
                        eprintln!("Warning: unable to record line in history: {err}");
                    }
                }

                if let ControlFlow::EXIT = control_state.handle_line(&line) {
                    break;
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("Error: {err}");
                break;
            }
        }
    }

    Ok(())
}

fn bind_handlers(rl: &mut Editor<IridiumHelper, FileHistory>) {
    let ceh = Box::new(CompleteHintHandler::new());

    //
    rl.bind_sequence(KeyEvent::ctrl('b'), EventHandler::Conditional(ceh.clone()));
    
    //
    rl.bind_sequence(KeyEvent::alt('f'), EventHandler::Conditional(ceh));
    
    //
    rl.bind_sequence(
        KeyEvent::from('\t'),
        EventHandler::Conditional(Box::new(TabEventHandler::new())));
    
    //
    rl.bind_sequence(
        Event::KeySeq(vec![KeyEvent::ctrl('X'), KeyEvent::ctrl('E')]),
        EventHandler::Simple(Cmd::Suspend));
}

fn load_history(rl: &mut Editor<IridiumHelper, FileHistory>) {
    match load_history_entries(None) {
        Ok(history) => {
            for entry in history {
                if let Err(err) = rl.add_history_entry(entry.as_str()) {
                    eprintln!("Warning: unable to record persisted history entry: {err}");
                }
            }
        }
        Err(err) => {
            eprintln!("Warning: unable to load history hints: {err}");
        }
    }
}