use crate::process::builtin::Builtin;

pub const EXIT_CODE: i32 = 1000;

pub struct Exit {

}

impl Builtin for Exit {
    fn call(&mut self, _args: &[String]) -> Option<i32> {
        return Some(EXIT_CODE);    
    }
}

impl Exit {
    pub fn new() -> Self {
        Exit {

        }
    }
}