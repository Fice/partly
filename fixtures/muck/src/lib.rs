//! Hand-written output of the future `#[derive(Muck)]`.
//!
//! Like the `basic` fixtures, blocks are marked `per struct` or `per field`,
//! since these become the macro's templates. The tests use the extension
//! crates on these types, so they also show what writing an extension takes.

pub mod account;
pub mod foo;
pub mod labeled;
