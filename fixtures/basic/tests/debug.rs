//! `Debug` output, which must always be available so that errors holding a
//! partial or a patch can be printed.

use partly::Set;
use partly::prelude::*;
use partly_fixtures_basic::foo::{Foo, FooPartial, FooPatch, FooSetters};
use partly_fixtures_basic::opaque::{Handle, Opaque, OpaquePartial};
use partly_fixtures_basic::tagged::TaggedPartial;

// `Foo`'s representations print through one hook, `DebugShape`, each with
// its own kind of slot, and named after what it is.

#[test]
fn a_partial_prints_its_slots() {
    let partial = FooPartial::new().with_bar(1);
    assert_eq!(
        format!("{partial:?}"),
        r#"FooPartial { bar: Some(1), zee: None }"#
    );
}

#[test]
fn a_typed_slot_prints_as_set() {
    let typed: FooPartial<Set<u32>, Option<String>> =
        FooPartial::new().with_bar(1).require_bar().unwrap();
    assert_eq!(
        format!("{typed:?}"),
        "FooPartial { bar: Set(1), zee: None }"
    );
}

#[test]
fn a_rejected_partial_prints_its_values() {
    let rejected = FooPartial::new().with_bar(1).build().unwrap_err();
    assert_eq!(
        format!("{:?}", rejected.value),
        "FooPartial { bar: Some(1), zee: None }"
    );
    assert!(format!("{rejected:?}").contains("bar: Some(1)"));
}

// `Opaque`'s hook is written by hand. It prints `handle` as `<opaque>`, slot
// and all, since it can't know whether the slot holds a value.

#[test]
fn a_field_without_debug_prints_as_opaque_and_the_others_still_print() {
    let mut partial = OpaquePartial::default();
    partial.set_handle(Handle);
    partial.set_count(1);
    assert_eq!(
        format!("{partial:?}"),
        "OpaquePartial { handle: <opaque>, count: Some(1) }"
    );
}

#[test]
fn a_rejected_partial_with_an_opaque_field_can_still_be_unwrapped() {
    let mut partial = OpaquePartial::default();
    partial.set_handle(Handle);

    // `unwrap_err` needs `Debug` on the success value, and `Opaque` has none.
    let rejected = partial.build().map(|_| ()).unwrap_err();
    assert!(format!("{rejected:?}").contains("handle: <opaque>"));

    // The real check: this line only compiles because the error is `Debug`.
    let mut complete = OpaquePartial::default();
    complete.set_handle(Handle);
    complete.set_count(1);
    let _: Opaque = complete.build().unwrap();
}

#[test]
fn a_patch_prints_what_it_does_to_each_field() {
    let value = Foo {
        bar: 1,
        zee: "a".into(),
    };
    let patch = value.edit(|bound| bound.set_bar(2));
    assert_eq!(
        format!("{patch:?}"),
        r#"FooPatch { bar: Changed { old: 1, new: 2 }, zee: Unchanged("a") }"#
    );
    assert_eq!(format!("{:?}", FooPatch::new()), "EmptyPatch");
}

#[test]
fn a_bound_patch_prints_what_is_staged() {
    let value = Foo {
        bar: 1,
        zee: "a".into(),
    };
    let mut bound = value.patch();
    bound.set_bar(2);
    assert_eq!(
        format!("{bound:?}"),
        r#"FooStaged { bar: Staged(2), zee: Unchanged("a") }"#
    );
}

#[test]
fn a_generic_field_prints_when_its_type_parameter_is_debug() {
    let mut partial: TaggedPartial<'_, Vec<u8>> = TaggedPartial::default();
    partial.set_value(vec![1]);
    assert_eq!(
        format!("{partial:?}"),
        "TaggedPartial { label: None, value: Some([1]) }"
    );
}
