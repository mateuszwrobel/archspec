//! Shared engine of the `archspec` CLI: Rust source parsing (`collector`),
//! diagram formatting primitives (`render`), and the scan-facts engine
//! (`scan`, `language`, `model`, plus the `parse` grammar registry and the
//! `profile` phase seam it runs on). The binary links these modules
//! directly; `collector` and `render` stay doc-hidden implementation
//! details, while the scan-facts surface (`scan`, `language`, `model`) is
//! exported for consumer reuse: `scan::extract` walks a project tree and
//! returns the `model::Model` vocabulary with zero writes (the stateless
//! fresh-walk law). This crate is the `archspec` command-line tool — it is
//! no longer an architecture-assertion library.

#[doc(hidden)]
pub mod collector;
#[doc(hidden)]
pub mod render;

pub mod language;
pub mod model;
pub mod parse;
pub mod profile;
pub mod scan;
