//! Enum partials: choosing a variant, building, validating and applying.

use partly::prelude::*;
use partly::{
    BuildFailure, DefaultBuildStrategy, Field, FieldError, MissingField,
    MissingValue, ValidationErrors,
};
use partly_fixtures_basic::enums::{
    EnumStructTest, EnumStructTestBuildStrategy, EnumStructTestFieldRef,
    EnumStructTestFieldValue, EnumStructTestFields,
    EnumStructTestMultipleValuesPartial, EnumStructTestNoValuePartial,
    EnumStructTestPartial, EnumStructTestPartialState,
    EnumStructTestSingleValuePartial, EnumStructTestValidator,
    EnumStructTestVariants, EnumTupleTest, EnumTupleTestFieldValue,
    EnumTupleTestFields, EnumTupleTestMultipleValuesPartial,
    EnumTupleTestPartial,
};

fn multiple() -> EnumStructTest {
    EnumStructTest::MultipleValues {
        bar: 1,
        zee: "a".into(),
    }
}

// ------------------------------------------------------------ choosing

#[test]
fn a_new_partial_has_no_variant() {
    let partial = EnumStructTestPartial::new();
    assert_eq!(partial.variant(), None);
    assert!(partial.is_empty());
    assert!(!partial.is_complete());
    assert!(!partial.can_build());
    assert_eq!(partial.missing_fields().count(), 0);
}

#[test]
fn choosing_a_variant_makes_its_fields_required() {
    let mut partial = EnumStructTestPartial::new();
    partial.multiple_values().set_bar(1);

    assert_eq!(
        partial.variant(),
        Some(EnumStructTestVariants::MultipleValues)
    );
    assert!(!partial.is_empty());
    assert!(!partial.is_complete());
    assert_eq!(partial.missing_fields().collect::<Vec<_>>(), [
        EnumStructTestFields::MultipleValuesZee
    ]);
    assert!(partial.has(EnumStructTestFields::MultipleValuesBar));
    assert!(!partial.has(EnumStructTestFields::SingleValueFoo));
}

#[test]
fn a_unit_variant_is_complete_as_soon_as_it_is_chosen() {
    let mut partial = EnumStructTestPartial::new();
    partial.no_value();
    assert!(partial.is_complete());
    assert_eq!(partial.build().unwrap(), EnumStructTest::NoValue);
}

#[test]
fn choosing_the_same_variant_again_keeps_its_fields() {
    let mut partial = EnumStructTestPartial::new();
    partial.multiple_values().set_bar(1);
    partial.multiple_values().set_zee("a".into());
    assert_eq!(partial.build().unwrap(), multiple());
}

#[test]
fn choosing_another_variant_discards_the_fields() {
    let mut partial = EnumStructTestPartial::new();
    partial.multiple_values().set_bar(1);
    partial.single_value();
    assert_eq!(partial.multiple_values().bar(), None);
}

#[test]
fn the_state_can_be_matched_on() {
    let partial = EnumStructTestPartial::from(
        EnumStructTestSingleValuePartial::new().with_foo(3),
    );
    match partial.state() {
        Some(EnumStructTestPartialState::SingleValue(variant)) => {
            assert_eq!(variant.foo(), Some(&3));
        },
        other => panic!("unexpected state {other:?}"),
    }
}

#[test]
fn fields_and_values_know_their_variant() {
    assert_eq!(
        EnumStructTestFields::MultipleValuesBar.variant(),
        EnumStructTestVariants::MultipleValues
    );
    assert_eq!(
        EnumStructTestVariants::from(&multiple()),
        EnumStructTestVariants::MultipleValues
    );
    assert_eq!(EnumStructTestVariants::SingleValue.name(), "SingleValue");
    assert_eq!(EnumStructTestVariants::MultipleValues.fields(), [
        EnumStructTestFields::MultipleValuesBar,
        EnumStructTestFields::MultipleValuesZee
    ]);
}

// ------------------------------------------------------------ building

#[test]
fn building_without_a_variant_fails_with_missing_variant() {
    let rejected = EnumStructTestPartial::new().build().unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingValue::Variant)
    );
    assert!(rejected.missing_variant());
    assert_eq!(rejected.missing_field(), None);
    assert_eq!(rejected.error.to_string(), "missing variant");
}

#[test]
fn building_with_a_missing_field_names_the_field() {
    let mut partial = EnumStructTestPartial::new();
    partial.multiple_values().set_zee("a".into());

    let mut rejected = partial.build().unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingValue::Field(
            EnumStructTestFields::MultipleValuesBar
        ))
    );
    assert!(!rejected.missing_variant());
    assert_eq!(
        rejected.missing_field(),
        Some(EnumStructTestFields::MultipleValuesBar)
    );
    assert_eq!(rejected.error.to_string(), "missing field `bar`");
    assert_eq!(
        rejected.value.multiple_values().zee().map(String::as_str),
        Some("a")
    );
}

#[test]
fn a_variant_partial_builds_the_enum() {
    let built = EnumStructTestMultipleValuesPartial::new()
        .with_bar(1)
        .with_zee("a".into())
        .build()
        .unwrap();
    assert_eq!(built, multiple());
    assert_eq!(
        EnumStructTestNoValuePartial::new().build().unwrap(),
        EnumStructTest::NoValue
    );
}

#[test]
fn a_value_round_trips_through_its_partial() {
    let partial = multiple().into_partial();
    assert!(partial.is_complete());
    assert_eq!(partial.build().unwrap(), multiple());
}

#[test]
fn a_variant_partial_field_can_be_required() {
    let partial = EnumStructTestMultipleValuesPartial::new().with_bar(1);
    let typed = partial.require_bar().unwrap();
    assert_eq!(typed.bar(), &1);
}

/// Chooses `NoValue` when nothing was chosen.
struct DefaultToNoValue;

impl EnumStructTestBuildStrategy for DefaultToNoValue {
    type Error = MissingValue<EnumStructTestFields>;

    fn resolve_variant(
        &self,
    ) -> Result<EnumStructTestPartialState, Self::Error> {
        Ok(EnumStructTestPartialState::NoValue(
            EnumStructTestNoValuePartial,
        ))
    }
}

#[test]
fn a_strategy_can_choose_the_variant() {
    let built = EnumStructTestPartial::new()
        .build_with_strategy(&DefaultToNoValue)
        .unwrap();
    assert_eq!(built, EnumStructTest::NoValue);
}

// ------------------------------------------------------------ merging

#[test]
fn merging_the_same_variant_merges_its_fields() {
    let left = EnumStructTestPartial::from(
        EnumStructTestMultipleValuesPartial::new().with_bar(1),
    );
    let right = EnumStructTestPartial::from(
        EnumStructTestMultipleValuesPartial::new().with_zee("a".into()),
    );
    assert_eq!(left.merge(right).build().unwrap(), multiple());
}

#[test]
fn merging_another_variant_takes_the_other_choice() {
    let left = EnumStructTestPartial::from(multiple());
    let right = EnumStructTestPartial::from(EnumStructTestNoValuePartial);
    assert_eq!(
        left.clone().merge(right).variant(),
        Some(EnumStructTestVariants::NoValue)
    );
    assert_eq!(
        left.merge(EnumStructTestPartial::new()).variant(),
        Some(EnumStructTestVariants::MultipleValues)
    );
}

// ------------------------------------------------------------ validation

/// Rejects zeros, and rejects `NoValue` as a whole.
struct NonZero;

impl EnumStructTestValidator for NonZero {
    type Error = &'static str;

    fn validate_single_value_foo(
        &self,
        value: &u32,
    ) -> Result<(), Self::Error> {
        if *value == 0 { Err("zero") } else { Ok(()) }
    }

    fn validate_multiple_values_bar(
        &self,
        value: &u32,
    ) -> Result<(), Self::Error> {
        if *value == 0 { Err("zero") } else { Ok(()) }
    }

    fn validate(&self, value: &EnumStructTest) -> Result<(), Self::Error> {
        if matches!(value, EnumStructTest::NoValue) {
            Err("no value")
        } else {
            Ok(())
        }
    }
}

#[test]
fn only_the_chosen_variants_fields_are_validated() {
    let value = EnumStructTest::MultipleValues {
        bar: 0,
        zee: "a".into(),
    };
    assert_eq!(
        value.validate_with(&NonZero),
        Err(ValidationErrors::Fields(vec![FieldError {
            field: EnumStructTestFields::MultipleValuesBar,
            error: "zero",
        }]))
    );
    assert_eq!(
        EnumStructTest::SingleValue { foo: 1 }.validate_with(&NonZero),
        Ok(())
    );
}

#[test]
fn a_build_that_fails_validation_hands_back_the_variant() {
    let partial = EnumStructTestPartial::from(
        EnumStructTestMultipleValuesPartial::new()
            .with_bar(0)
            .with_zee("a".into()),
    );
    let rejected = partial
        .build_with(&DefaultBuildStrategy, &NonZero)
        .unwrap_err();
    assert!(matches!(
        rejected.error,
        BuildFailure::Validation(ValidationErrors::Fields(_))
    ));
    assert_eq!(
        rejected.value.variant(),
        Some(EnumStructTestVariants::MultipleValues)
    );
    assert!(rejected.value.is_complete());
}

// ------------------------------------------------------------ applying

#[test]
fn applying_the_same_variant_writes_the_fields_that_are_set() {
    let mut value = multiple();
    value
        .apply_partial(EnumStructTestMultipleValuesPartial::new().with_bar(2))
        .unwrap();
    assert_eq!(value, EnumStructTest::MultipleValues {
        bar: 2,
        zee: "a".into(),
    });
}

#[test]
fn applying_a_complete_partial_of_another_variant_replaces_the_value() {
    let mut value = multiple();
    value
        .apply_partial(EnumStructTestSingleValuePartial::new().with_foo(5))
        .unwrap();
    assert_eq!(value, EnumStructTest::SingleValue { foo: 5 });
}

#[test]
fn applying_an_incomplete_partial_of_another_variant_is_rejected() {
    let mut value = EnumStructTest::NoValue;
    let rejected = value
        .apply_partial(EnumStructTestMultipleValuesPartial::new().with_bar(1))
        .unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingField(
            EnumStructTestFields::MultipleValuesZee
        ))
    );
    assert_eq!(value, EnumStructTest::NoValue);
    assert!(rejected.value.has(EnumStructTestFields::MultipleValuesBar));
}

#[test]
fn applying_nothing_changes_nothing() {
    let mut value = multiple();
    value.apply_partial(EnumStructTestPartial::new()).unwrap();
    assert_eq!(value, multiple());
}

#[test]
fn a_failed_whole_value_check_rolls_back_a_replacement() {
    let mut value = multiple();
    let rejected = value
        .apply_partial_with(EnumStructTestNoValuePartial.into(), &NonZero)
        .unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Validation(ValidationErrors::Struct("no value"))
    );
    assert_eq!(value, multiple());
    assert_eq!(
        rejected.value.variant(),
        Some(EnumStructTestVariants::NoValue)
    );
}

#[test]
fn a_failed_field_check_leaves_the_value_untouched() {
    let mut value = multiple();
    let rejected = value
        .apply_partial_with(
            EnumStructTestMultipleValuesPartial::new()
                .with_bar(0)
                .into(),
            &NonZero,
        )
        .unwrap_err();
    assert!(matches!(
        rejected.error,
        BuildFailure::Validation(ValidationErrors::Fields(_))
    ));
    assert_eq!(value, multiple());
}

// ------------------------------------------------------------ debug

#[test]
fn a_partial_prints_its_state() {
    let partial = EnumStructTestPartial::from(
        EnumStructTestMultipleValuesPartial::new().with_bar(1),
    );
    assert_eq!(
        format!("{partial:?}"),
        "EnumStructTestPartial { state: \
         Some(MultipleValues(EnumStructTestMultipleValuesPartial { bar: \
         Some(1), zee: None })) }"
    );
}

// ------------------------------------------------------------ tuple variants

#[test]
fn tuple_fields_are_named_by_index() {
    assert_eq!(EnumTupleTestFields::MultipleValues1.name(), "1");

    let mut partial = EnumTupleTestPartial::new();
    let variant = partial.multiple_values();
    variant.set_0(1);
    variant.set_2("a".into());
    assert_eq!(variant.get_0(), Some(&1));

    let rejected = partial.build().unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingValue::Field(
            EnumTupleTestFields::MultipleValues1
        ))
    );
    assert_eq!(rejected.error.to_string(), "missing field `1`");
}

#[test]
fn a_tuple_variant_builds_and_round_trips() {
    let built = EnumTupleTestMultipleValuesPartial::new()
        .with_0(1)
        .with_1(2)
        .with_2("a".into())
        .build()
        .unwrap();
    assert_eq!(built, EnumTupleTest::MultipleValues(1, 2, "a".into()));
    assert_eq!(built.clone().into_partial().build().unwrap(), built);
}

#[test]
fn applying_a_tuple_variant_writes_by_position() {
    let mut value = EnumTupleTest::MultipleValues(1, 2, "a".into());
    value
        .apply_partial(EnumTupleTestMultipleValuesPartial::new().with_1(9))
        .unwrap();
    assert_eq!(value, EnumTupleTest::MultipleValues(1, 9, "a".into()));
}

// ------------------------------------------------------------ reflection

#[test]
fn iter_lists_the_chosen_variants_fields() {
    let partial = EnumStructTestPartial::from(multiple());
    assert_eq!(partial.iter().collect::<Vec<_>>(), [
        EnumStructTestFieldRef::MultipleValuesBar(&1),
        EnumStructTestFieldRef::MultipleValuesZee(&"a".into()),
    ]);
    assert_eq!(EnumStructTestPartial::new().iter().count(), 0);
}

#[test]
fn setting_a_field_of_the_chosen_variant_returns_the_old_value() {
    let mut partial = EnumStructTestPartial::from(multiple());
    assert_eq!(
        partial.set_field(EnumStructTestFieldValue::MultipleValuesBar(2)),
        Some(EnumStructTestFieldValue::MultipleValuesBar(1))
    );
}

#[test]
fn setting_a_field_of_another_variant_switches_to_it() {
    let mut partial = EnumStructTestPartial::from(multiple());
    assert_eq!(
        partial.set_field(EnumStructTestFieldValue::SingleValueFoo(3)),
        None
    );
    assert_eq!(partial.build().unwrap(), EnumStructTest::SingleValue {
        foo: 3,
    });
}

#[test]
fn taking_a_field_of_another_variant_gives_nothing() {
    let mut partial = EnumStructTestPartial::from(multiple());
    assert_eq!(
        partial.take_field(EnumStructTestFields::SingleValueFoo),
        None
    );
    assert_eq!(
        partial.take_field(EnumStructTestFields::MultipleValuesBar),
        Some(EnumStructTestFieldValue::MultipleValuesBar(1))
    );
    assert_eq!(
        partial.variant(),
        Some(EnumStructTestVariants::MultipleValues)
    );
}

#[test]
fn collecting_values_depends_on_their_order() {
    let partial: EnumStructTestPartial = [
        EnumStructTestFieldValue::MultipleValuesBar(1),
        EnumStructTestFieldValue::SingleValueFoo(2),
    ]
    .into_iter()
    .collect();
    assert_eq!(partial.build().unwrap(), EnumStructTest::SingleValue {
        foo: 2,
    });
}

#[test]
fn a_partial_round_trips_through_its_values() {
    let partial: EnumStructTestPartial =
        EnumStructTestPartial::from(multiple())
            .into_iter()
            .collect();
    assert_eq!(partial.build().unwrap(), multiple());
}

#[test]
fn a_unit_variant_is_chosen_by_name() {
    let mut partial = EnumStructTestPartial::new();
    partial.set_variant(EnumStructTestVariants::NoValue);
    assert_eq!(partial.build().unwrap(), EnumStructTest::NoValue);
    assert_eq!(partial_values(EnumStructTest::NoValue), 0);
}

fn partial_values(value: EnumStructTest) -> usize {
    value.into_partial().into_iter().count()
}

#[test]
fn setting_the_chosen_variant_by_name_keeps_its_fields() {
    let mut partial = EnumStructTestPartial::from(multiple());
    partial.set_variant(EnumStructTestVariants::MultipleValues);
    assert_eq!(partial.build().unwrap(), multiple());
}

#[test]
fn tuple_fields_reflect_by_position() {
    let partial: EnumTupleTestPartial = [
        EnumTupleTestFieldValue::MultipleValues0(1),
        EnumTupleTestFieldValue::MultipleValues1(2),
        EnumTupleTestFieldValue::MultipleValues2("a".into()),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        partial
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>(),
        [
            "MultipleValues0(1)",
            "MultipleValues1(2)",
            "MultipleValues2(\"a\")"
        ]
    );
    assert_eq!(
        partial.build().unwrap(),
        EnumTupleTest::MultipleValues(1, 2, "a".into())
    );
}
