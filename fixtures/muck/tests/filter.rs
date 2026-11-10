//! The filter extension on the fixtures: each field gets its type's filter.

use fieldmuck_filter::{
    self as filter, Comparison, Filter, OptionFilter, TextFilter,
};
use fieldmuck_fixtures::foo::{Foo, FooShape};
use fieldmuck_fixtures::labeled::Labeled;

fn foo() -> Foo {
    Foo {
        bar:  5,
        zee:  "hello".into(),
        nick: None,
    }
}

#[test]
fn an_empty_filter_matches_everything() {
    let filter = filter::empty::<Foo>();
    assert_eq!(filter, FooShape::from_slots(None, None, None));
    assert!(filter::matches(&filter, &foo()));
}

#[test]
fn each_field_gets_the_filter_of_its_type() {
    let filter: Filter<Foo> = FooShape::from_slots(
        Some(Comparison::Greater(3)),
        Some(TextFilter::Length(Comparison::Less(10))),
        Some(OptionFilter::IsNotSet),
    );
    assert!(filter::matches(&filter, &foo()));
}

#[test]
fn a_filter_prints_with_its_kind() {
    let mut filter = filter::empty::<Foo>();
    filter.bar = Some(Comparison::Greater(3));
    assert_eq!(
        format!("{filter:?}"),
        "FooFilter { bar: Some(Greater(3)), zee: None, nick: None }"
    );
}

#[test]
fn every_filtered_field_must_match() {
    let mut filter = filter::empty::<Foo>();
    filter.zee = Some(TextFilter::Contains("ell".into()));
    assert!(filter::matches(&filter, &foo()));

    filter.nick = Some(OptionFilter::IsSet);
    assert!(!filter::matches(&filter, &foo()));

    let named = Foo {
        nick: Some("n".into()),
        ..foo()
    };
    assert!(filter::matches(&filter, &named));
}

#[test]
fn an_option_can_filter_its_value() {
    let mut filter = filter::empty::<Foo>();
    filter.nick = Some(OptionFilter::Value(TextFilter::StartsWith("a".into())));
    let named = |nick: &str| {
        Foo {
            nick: Some(nick.into()),
            ..foo()
        }
    };
    assert!(filter::matches(&filter, &named("anna")));
    assert!(!filter::matches(&filter, &named("bob")));
    assert!(!filter::matches(&filter, &foo()));
}

#[test]
fn a_struct_with_a_lifetime_and_a_type_parameter_can_be_filtered() {
    let value = Labeled {
        label: "price",
        value: 2.5_f64,
    };
    let mut filter = filter::empty::<Labeled<'_, f64>>();
    filter.label = Some(TextFilter::Equal("price".into()));
    filter.value = Some(Comparison::LessOrEqual(3.0));
    assert!(filter::matches(&filter, &value));

    filter.value = Some(Comparison::Greater(3.0));
    assert!(!filter::matches(&filter, &value));
}
