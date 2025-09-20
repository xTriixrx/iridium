use rustyline::hint::HistoryHinter;
use rustyline::highlight::Highlighter;
use std::borrow::Cow::{self, Borrowed, Owned};
use rustyline_derive::{Completer, Helper, Hinter, Validator};

#[derive(Completer, Helper, Hinter, Validator)]
pub struct IridiumHelper(#[rustyline(Hinter)] HistoryHinter);

impl IridiumHelper {
    pub fn new(hinter: HistoryHinter) -> Self {
        Self {
            0: hinter,
        }
    }
}

impl Highlighter for IridiumHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Owned(format!("\x1b[1;32m{prompt}\x1b[m"))
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned(format!("\x1b[1m{hint}\x1b[m"))
    }
}