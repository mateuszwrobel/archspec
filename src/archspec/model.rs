//! Binary-side re-export of the library model vocabulary (discovery-extraction
//! rung one). The model compiles into `rust_arch_test_kit::model`; this thin
//! module keeps every `crate::archspec::model::…` path in the modules above
//! resolving, so the extracted-model types have exactly one source of truth.

pub use rust_arch_test_kit::model::*;
