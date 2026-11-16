//! Binary-side re-export of the library phase-timing seam (discovery-extraction
//! rung one). The seam compiles into `rust_arch_test_kit::profile`; this thin
//! module keeps every `crate::archspec::profile::…` path in the modules above
//! resolving from the same single source.

pub use rust_arch_test_kit::profile::*;
