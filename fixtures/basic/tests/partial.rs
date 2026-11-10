//! The field accessors and the operations in `PartialOps`.

use partly::prelude::*;
use partly::{Partial, Set};
use partly_fixtures_basic::foo::{Foo, FooFields, FooPartial};

fn foo() -> Foo {
    Foo {
        bar: 1,
        zee: "a".into(),
    }
}

#[test]
fn a_new_partial_is_empty() {
    let partial = Foo::partial();
    assert!(partial.is_empty());
    assert!(!partial.is_complete());
    assert_eq!(partial, FooPartial::new());
    assert_eq!(partial, FooPartial::default());
}

#[test]
fn accessors_read_and_change_one_field() {
    let mut partial = FooPartial::new();
    assert!(!partial.has_bar());
    assert_eq!(partial.bar(), None);

    partial.set_bar(1);
    assert!(partial.has_bar());
    assert_eq!(partial.bar(), Some(&1));
    assert!(!partial.has_zee());

    *partial.bar_mut().unwrap() = 2;
    assert_eq!(partial.take_bar(), Some(2));
    assert_eq!(partial.bar(), None);

    partial.set_bar(3);
    partial.clear_bar();
    assert!(partial.is_empty());
}

#[test]
fn with_setters_chain() {
    let partial = FooPartial::new().with_bar(1).with_zee("a".into());
    assert!(partial.is_complete());
    assert!(!partial.is_empty());
}

#[test]
fn set_and_missing_fields_are_listed_in_declaration_order() {
    let partial = FooPartial::new().with_zee("a".into());
    assert_eq!(partial.set_fields().collect::<Vec<_>>(), [FooFields::Zee]);
    assert_eq!(partial.missing_fields().collect::<Vec<_>>(), [
        FooFields::Bar
    ]);
    assert!(partial.has(FooFields::Zee));
    assert!(!partial.has(FooFields::Bar));
}

#[test]
fn converting_a_struct_sets_every_field() {
    let moved = foo().into_partial();
    assert!(moved.is_complete());
    assert_eq!(foo().to_partial(), moved);
    assert_eq!(FooPartial::from(foo()), moved);
}

#[test]
fn merge_prefers_the_argument_and_or_prefers_the_receiver() {
    let left = || FooPartial::new().with_bar(1);
    let right = || FooPartial::new().with_bar(2).with_zee("b".into());

    let merged = left().merge(right());
    assert_eq!(merged.bar(), Some(&2));
    assert_eq!(merged.zee().map(String::as_str), Some("b"));

    let ordered = left().or(right());
    assert_eq!(ordered.bar(), Some(&1));
    assert_eq!(ordered.zee().map(String::as_str), Some("b"));
}

#[test]
fn collecting_partials_folds_them_and_later_ones_win() {
    let layers = [
        FooPartial::new().with_bar(1).with_zee("a".into()),
        FooPartial::new().with_bar(2),
    ];
    let folded: FooPartial = layers.into_iter().collect();
    assert_eq!(folded.bar(), Some(&2));
    assert_eq!(folded.zee().map(String::as_str), Some("a"));
}

#[test]
fn require_moves_one_field_to_a_compile_time_guarantee() {
    let partial = FooPartial::new().with_bar(1);

    // `bar()` now returns `&u32`, and `zee()` is still an `Option`.
    let typed: FooPartial<Set<u32>, Option<String>> =
        partial.require_bar().unwrap();
    assert_eq!(*typed.bar(), 1);
    assert_eq!(typed.zee(), None);

    // A failed check hands back the partial.
    let rejected = typed.require_zee().unwrap_err();
    assert_eq!(*rejected.bar(), 1);
}

/// Written against the traits only, so it works for any derived struct.
fn layered<T: Partly>(layers: Vec<Partial<T>>) -> Partial<T> {
    layers.into_iter().fold(T::partial(), PartialOps::merge)
}

#[test]
fn generic_code_can_use_the_core_traits() {
    let folded = layered::<Foo>(vec![
        FooPartial::new().with_bar(1),
        FooPartial::new().with_zee("a".into()),
    ]);
    assert!(folded.is_complete());
}
