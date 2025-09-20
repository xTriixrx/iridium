use crate::process::builtin::Builtin;

pub struct Pushd {}

impl Builtin for Pushd {
    fn call(&mut self, args: &[String]) -> Option<i32> {
        println!("PUSHD!");
        Some(0)
    }
}

impl Pushd {
    pub fn new() -> Self {
        Pushd {}
    }
}
