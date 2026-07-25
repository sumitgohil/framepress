//! Built-in compression presets and the resolver that maps a preset +
//! format pair to engine settings.

mod builtin;
mod resolver;

pub use builtin::{builtin_spec, BuiltinPresetResolver};
pub use resolver::resolve;
