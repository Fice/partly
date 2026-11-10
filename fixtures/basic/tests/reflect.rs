//! Field-by-field access for code that doesn't know the field names.

use partly::prelude::*;
use partly::{Field, MissingField};
use partly_fixtures_basic::foo::{
    FooFieldRef, FooFieldValue, FooFields, FooPartial,
};

#[test]
fn the_field_enum_lists_every_field() {
    assert_eq!(FooFields::ALL, [FooFields::Bar, FooFields::Zee]);
    assert_eq!(FooFields::Zee.name(), "zee");
    assert_eq!(FooFieldValue::Bar(1).field(), FooFields::Bar);
    assert_eq!(FooFieldValue::Zee("a".into()).field(), FooFields::Zee);
    assert_eq!(
        MissingField(FooFields::Bar).to_string(),
        "missing field `bar`"
    );
}

#[test]
fn iter_borrows_only_the_fields_that_are_set() {
    let partial = FooPartial::new().with_zee("a".into());
    let zee = "a".to_owned();
    assert_eq!(partial.iter().collect::<Vec<_>>(), [FooFieldRef::Zee(&zee)]);

    let both = partial.with_bar(1);
    assert_eq!(both.iter().collect::<Vec<_>>(), [
        FooFieldRef::Bar(&1),
        FooFieldRef::Zee(&zee)
    ]);
}

#[test]
fn into_iter_moves_the_fields_out() {
    let partial = FooPartial::new().with_bar(1).with_zee("a".into());
    assert_eq!(partial.into_iter().collect::<Vec<_>>(), [
        FooFieldValue::Bar(1),
        FooFieldValue::Zee("a".into())
    ]);
}

#[test]
fn set_field_returns_the_value_it_replaced() {
    let mut partial = FooPartial::new();
    assert_eq!(partial.set_field(FooFieldValue::Bar(1)), None);
    assert_eq!(
        partial.set_field(FooFieldValue::Bar(2)),
        Some(FooFieldValue::Bar(1))
    );
    assert_eq!(partial.bar(), Some(&2));

    assert_eq!(partial.set_field(FooFieldValue::Zee("a".into())), None);
    assert_eq!(
        partial.set_field(FooFieldValue::Zee("b".into())),
        Some(FooFieldValue::Zee("a".into()))
    );
}

#[test]
fn take_field_unsets_it() {
    let mut partial = FooPartial::new().with_bar(1).with_zee("a".into());
    assert_eq!(
        partial.take_field(FooFields::Bar),
        Some(FooFieldValue::Bar(1))
    );
    assert_eq!(partial.take_field(FooFields::Bar), None);
    assert_eq!(
        partial.take_field(FooFields::Zee),
        Some(FooFieldValue::Zee("a".into()))
    );
    assert!(partial.is_empty());
}

#[test]
fn a_partial_can_be_collected_from_field_values_and_round_trips() {
    let values = [FooFieldValue::Zee("a".into()), FooFieldValue::Bar(1)];
    let partial: FooPartial = values.into_iter().collect();
    assert!(partial.is_complete());

    let rebuilt: FooPartial = partial.clone().into_iter().collect();
    assert_eq!(rebuilt, partial);
}

#[test]
fn extend_sets_fields_and_later_values_win() {
    let mut partial = FooPartial::new().with_bar(1);
    partial.extend([FooFieldValue::Bar(2), FooFieldValue::Zee("a".into())]);
    assert_eq!(partial.bar(), Some(&2));
    assert!(partial.is_complete());
}
