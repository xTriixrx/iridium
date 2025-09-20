use rustyline::{Cmd, ConditionalEventHandler, Event, EventContext, KeyEvent, RepeatCount};

pub struct TabEventHandler;

impl TabEventHandler {
    pub fn new() -> Self {
        Self {
            
        }
    }
}

impl ConditionalEventHandler for TabEventHandler {
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