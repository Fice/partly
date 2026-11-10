//! Hand-written output of the future derive macro.
//!
//! This is a separate crate because derive output lives in the user's crate, so
//! it can only reach the public API of `partly`. When the macro exists, each
//! expansion here is replaced by `#[derive(Partly)]` and the tests stay as they
//! are.
//!
//! Rules for the expansions:
//! - Only use what a macro can see: tokens and attributes, no type information.
//! - Every block is marked `per struct` or `per field`, since these become the
//!   macro's templates.
//!
//! Each file starts with the struct as the user would write it.

// A macro can't tell that a field type is `Copy`, so it always writes `clone`,
// and slot generics make for long types. Real macro output is exempt from these
// lints.
#![expect(clippy::clone_on_copy, clippy::type_complexity)]

pub mod bar;
pub mod config;
pub mod enums;
pub mod foo;
pub mod keywords;
pub mod opaque;
pub mod tagged;
