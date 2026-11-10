//! Struct shapes that could break the expansion's templates.

use partly::prelude::*;
use partly::{BuildFailure, Field, MissingField};
use partly_fixtures_basic::keywords::{
    Keywords, KeywordsFields, KeywordsPartial,
};
use partly_fixtures_basic::opaque::{
    Handle, Opaque, OpaqueFields, OpaquePartial,
};
use partly_fixtures_basic::tagged::{Tagged, TaggedFields, TaggedPartial};

#[test]
fn a_renamed_field_does_not_shadow_the_build_operation() {
    let mut partial = KeywordsPartial::default();
    partial.set_build_count(1);
    partial.set_type("a".into());
    assert_eq!(partial.build_count(), Some(&1));
    assert_eq!(partial.r#type().map(String::as_str), Some("a"));
    assert!(partial.can_build());

    let built = partial.build().unwrap();
    assert_eq!(built.build, 1);
    assert_eq!(built.r#type, "a");
}

#[test]
fn a_rename_keeps_the_real_name_in_errors() {
    assert_eq!(KeywordsFields::Build.name(), "build");
    assert_eq!(KeywordsFields::Type.name(), "type");

    let rejected = KeywordsPartial::default().build().unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingField(KeywordsFields::Build))
    );
}

#[test]
fn a_keyword_field_can_be_applied() {
    let mut value = Keywords {
        build:  1,
        r#type: "a".into(),
    };
    let mut input = KeywordsPartial::default();
    input.set_type("b".into());
    value.apply_partial(input).unwrap();
    assert_eq!(value.r#type, "b");
}

#[test]
fn a_struct_with_a_lifetime_and_a_type_parameter_works() {
    let owned = String::from("label");
    let mut partial: TaggedPartial<'_, Vec<u8>> = TaggedPartial::default();
    partial.set_label(&owned);
    partial.set_value(vec![1]);
    assert_eq!(partial.value(), Some(&vec![1]));
    assert_eq!(partial.label(), Some(&"label"));
    assert!(partial.has(TaggedFields::Label));

    let mut built: Tagged<'_, Vec<u8>> = partial.build().unwrap();
    assert_eq!(built.value, [1]);

    let mut input = TaggedPartial::default();
    input.set_value(vec![2]);
    built.apply_partial(input).unwrap();
    assert_eq!(built.value, [2]);
    assert_eq!(built.label, "label");

    assert_eq!(built.into_partial().value(), Some(&vec![2]));
}

#[test]
fn a_generic_partial_with_a_type_lacking_traits_still_builds() {
    let mut partial: TaggedPartial<'_, Handle> = TaggedPartial::default();
    partial.set_label("x");
    let rejected = partial.build().map(|_| ()).unwrap_err();
    assert!(rejected.value.has(TaggedFields::Label));
}

#[test]
fn a_field_with_no_traits_compiles_and_builds() {
    let mut partial = OpaquePartial::default();
    partial.set_count(1);
    partial.set_handle(Handle);
    assert!(partial.is_complete());
    // `unwrap` needs `Debug` on the partial, which doesn't need it on `Handle`.
    let built: Opaque = partial.build().unwrap();
    assert_eq!(built.count, 1);
    assert_eq!(OpaqueFields::ALL.len(), 2);
}

#[test]
fn merging_works_without_any_field_traits() {
    let mut left = OpaquePartial::default();
    left.set_count(1);
    let mut right = OpaquePartial::default();
    right.set_count(2);
    assert_eq!(left.merge(right).count(), Some(&2));
}
