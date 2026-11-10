//! The SQL extension, which knows the filter crate but no user type.

use fieldmuck_filter::{self as filter, Comparison, OptionFilter, TextFilter};
use fieldmuck_filter_sql::{SqlValue, condition};
use fieldmuck_fixtures::foo::Foo;
use fieldmuck_fixtures::labeled::Labeled;

#[test]
fn an_empty_filter_is_true() {
    let condition = condition::<Foo>(&filter::empty::<Foo>());
    assert_eq!(condition.sql(), "TRUE");
    assert!(condition.params().is_empty());
}

#[test]
fn each_filtered_field_becomes_a_condition_on_its_column() {
    let mut filter = filter::empty::<Foo>();
    filter.bar = Some(Comparison::GreaterOrEqual(3));
    filter.zee = Some(TextFilter::Contains("50%".into()));
    filter.nick = Some(OptionFilter::IsNotSet);

    let condition = condition::<Foo>(&filter);
    assert_eq!(
        condition.sql(),
        "bar >= ? AND zee LIKE ? ESCAPE '\\' AND nick IS NULL"
    );
    assert_eq!(condition.params(), [
        SqlValue::Integer(3),
        SqlValue::Text("%50\\%%".into()),
    ]);
}

#[test]
fn nested_filters_use_the_inner_filter() {
    let mut filter = filter::empty::<Foo>();
    filter.zee = Some(TextFilter::Length(Comparison::Less(8)));
    filter.nick = Some(OptionFilter::Value(TextFilter::Equal("n".into())));

    let condition = condition::<Foo>(&filter);
    assert_eq!(condition.sql(), "LENGTH(zee) < ? AND nick = ?");
    assert_eq!(condition.params(), [
        SqlValue::Integer(8),
        SqlValue::Text("n".into()),
    ]);
}

#[test]
fn a_generic_struct_works_the_same() {
    let mut filter = filter::empty::<Labeled<'_, f64>>();
    filter.value = Some(Comparison::Less(1.5));
    let condition = condition::<Labeled<'_, f64>>(&filter);
    assert_eq!(condition.sql(), "value < ?");
    assert_eq!(condition.params(), [SqlValue::Real(1.5)]);
}
