//! Binary-side re-export of the library language detection (discovery-extraction
//! rung one). Language detection compiles into `rust_arch_test_kit::language`;
//! this thin module keeps every `crate::archspec::language::…` path in the
//! modules above resolving from the same single source.

pub use rust_arch_test_kit::language::*;
