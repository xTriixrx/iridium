use rustyline::hint::HistoryHinter;
use rustyline::highlight::Highlighter;
use std::borrow::Cow::{self, Borrowed, Owned};
use rustyline_derive::{Completer, Helper, Hinter, Validator};

/// Aggregates the rustyline helper traits used by Iridium.
#[derive(Completer, Helper, Hinter, Validator)]
pub struct IridiumHelper(#[rustyline(Hinter)] HistoryHinter);

impl IridiumHelper {
    /// Build a helper with the provided hinter implementation.
    pub fn new(hinter: HistoryHinter) -> Self {
        Self { 0: hinter }
    }
}

impl Highlighter for IridiumHelper {
    /// Highlight the prompt when rustyline requests it.
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

    /// Render completion hints in bold so they stand out.
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned(format!("\x1b[1m{hint}\x1b[m"))
    }
}
