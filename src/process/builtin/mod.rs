pub mod map;

/// Trait implemented by all builtins so they can be invoked through [`BuiltinMap`].
pub trait Builtin {
    /// Execute the builtin with the provided arguments, returning an optional status code.
    fn call(&mut self, args: &[String]) -> Option<i32>;
}
