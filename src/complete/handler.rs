use rustyline::{Cmd, ConditionalEventHandler, Event, EventContext, KeyEvent, RepeatCount};

/// Conditional handler that only inserts a tab when appropriate.
pub struct TabEventHandler;

impl TabEventHandler {
    /// Construct the tab event handler.
    pub fn new() -> Self {
        Self {
            
        }
    }
}

impl ConditionalEventHandler for TabEventHandler {
    /// Insert a literal tab when invoked in whitespace, otherwise fall back to completion.
    fn handle(&self, evt: &Event, n: RepeatCount, _: bool, ctx: &EventContext) -> Option<Cmd> {
        debug_assert_eq!(*evt, Event::from(KeyEvent::from('\t')));
        if ctx.line()[..ctx.pos()]
            .chars()
            .next_back()
            .filter(|c| c.is_whitespace())
            .is_some()
        {
            Some(Cmd::SelfInsert(n, '\t'))
        } else {
            None // default complete
        }
    }
}
