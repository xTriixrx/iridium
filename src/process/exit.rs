pub const EXIT_CODE: i32 = 1000;

pub fn exit(_args: &[String]) -> Option<i32> {
    return Some(EXIT_CODE);
}