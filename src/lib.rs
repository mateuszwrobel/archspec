//! Shared engine of the `archspec` CLI: Rust source parsing (`collector`) and
//! diagram formatting primitives (`render`). The binary links these modules
//! directly; they are doc-hidden implementation details, not a public API.
//! This crate is the `archspec` command-line tool — it is no longer an
//! architecture-assertion library.

#[doc(hidden)]
pub mod collector;
#[doc(hidden)]
pub mod render;
