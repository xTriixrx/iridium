use crate::process::builtin::Builtin;

pub struct Help {

}

impl Builtin for Help {
    fn call(&mut self, _args: &[String]) -> Option<i32> {
        return Some(0);    
    }
}

impl Help {
    pub fn new() -> Self {
        Help {
            
        }
    }
}