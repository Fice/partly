//! Enum patches: field changes within a variant, and replacing the value with
//! another variant.

use partly::prelude::*;
use partly::{
    AcceptAll, ApplyFailure, ChainFailure, EnumConflict, EnumMergeError,
    FieldMerge, MissingValue, Resolution, StrictMerge, ValidationErrors,
    VariantConflict,
};
use partly_fixtures_basic::enums::{
    EnumStructTest, EnumStructTestFields, EnumStructTestMergeStrategy,
    EnumStructTestPartial, EnumStructTestPatch, EnumStructTestValidator,
    EnumStructTestVariants, EnumTupleTest,
};

fn multiple() -> EnumStructTest {
    EnumStructTest::MultipleValues {
        bar: 1,
        zee: "a".into(),
    }
}

fn conflict(
    conflict: EnumConflict<EnumStructTestFields>,
) -> ApplyFailure<
    EnumMergeError<EnumStructTestFields>,
    ValidationErrors<EnumStructTestFields, std::convert::Infallible>,
> {
    ApplyFailure::Merge(EnumMergeError::Conflict(conflict))
}

// ------------------------------------------------------------ the same variant

#[test]
fn a_patch_of_the_same_variant_writes_its_fields() {
    let patch = multiple().edit(|bound| bound.multiple_values().set_bar(2));
    assert!(patch.has_variant());
    assert!(!patch.changes_variant());
    assert!(patch.has(EnumStructTestFields::MultipleValuesBar));
    assert!(!patch.is_complete());

    let mut value = multiple();
    value.apply_patch(patch).unwrap();
    assert_eq!(value, EnumStructTest::MultipleValues {
        bar: 2,
        zee: "a".into(),
    });
}

#[test]
fn the_value_must_match_exactly() {
    let patch = multiple().edit(|bound| bound.multiple_values().set_bar(2));

    let mut changed = EnumStructTest::MultipleValues {
        bar: 1,
        zee: "x".into(),
    };
    let rejected = changed.apply_patch(patch.clone()).unwrap_err();
    assert_eq!(
        rejected.error,
        conflict(EnumConflict::Field(EnumStructTestFields::MultipleValuesZee))
    );
    assert_eq!(rejected.value, patch);

    // `FieldMerge` keeps the change to `zee`, which the patch doesn't write.
    changed
        .apply_patch_with_strategy(patch, &FieldMerge)
        .unwrap();
    assert_eq!(changed, EnumStructTest::MultipleValues {
        bar: 2,
        zee: "x".into(),
    });
}

#[test]
fn a_target_of_another_variant_conflicts_on_the_variant() {
    let patch = multiple().edit(|bound| bound.multiple_values().set_bar(2));
    let mut value = EnumStructTest::NoValue;
    let rejected = value.apply_patch(patch).unwrap_err();
    assert_eq!(rejected.error, conflict(EnumConflict::Variant));
    assert_eq!(value, EnumStructTest::NoValue);
}

#[test]
fn check_lists_the_variant_or_every_field() {
    let patch = multiple().edit(|bound| bound.multiple_values().set_bar(2));
    assert_eq!(patch.check(&multiple()), Ok(()));
    assert_eq!(
        patch.check(&EnumStructTest::NoValue),
        Err(vec![EnumConflict::Variant])
    );
    assert_eq!(
        patch.check(&EnumStructTest::MultipleValues {
            bar: 9,
            zee: "x".into(),
        }),
        Err(vec![
            EnumConflict::Field(EnumStructTestFields::MultipleValuesBar),
            EnumConflict::Field(EnumStructTestFields::MultipleValuesZee),
        ])
    );
}

// ------------------------------------------------------------ another variant

#[test]
fn a_patch_of_another_variant_replaces_the_value() {
    let patch = multiple().edit(|bound| bound.single_value().set_foo(5));
    assert!(patch.changes_variant());
    assert_eq!(patch.fields(), [EnumStructTestFields::SingleValueFoo]);
    assert!(patch.is_complete());

    let mut value = multiple();
    value.apply_patch(patch).unwrap();
    assert_eq!(value, EnumStructTest::SingleValue { foo: 5 });
}

#[test]
fn a_patch_can_change_to_a_unit_variant() {
    let patch = multiple().edit(|bound| {
        bound.no_value();
    });
    let mut value = multiple();
    value.apply_patch(patch).unwrap();
    assert_eq!(value, EnumStructTest::NoValue);
}

#[test]
fn another_variant_must_build_with_the_default_strategy() {
    let patch = EnumStructTest::NoValue
        .edit(|bound| bound.multiple_values().set_bar(1));
    let mut value = EnumStructTest::NoValue;
    let rejected = value.apply_patch(patch).unwrap_err();
    assert_eq!(
        rejected.error,
        ApplyFailure::Merge(EnumMergeError::Build(MissingValue::Field(
            EnumStructTestFields::MultipleValuesZee
        )))
    );
    assert_eq!(value, EnumStructTest::NoValue);
}

#[test]
fn diff_writes_changed_fields_or_the_whole_other_variant() {
    let same_variant = EnumStructTest::MultipleValues {
        bar: 2,
        zee: "a".into(),
    };
    let patch = multiple().diff(&same_variant);
    assert!(!patch.changes_variant());
    assert!(patch.has(EnumStructTestFields::MultipleValuesBar));
    assert!(!patch.has(EnumStructTestFields::MultipleValuesZee));

    let other = multiple().diff(&EnumStructTest::SingleValue { foo: 3 });
    assert!(other.changes_variant());
    assert!(other.is_complete());

    let nothing = multiple().diff(&multiple());
    assert!(nothing.is_empty());
    assert!(nothing.is_noop());
    assert!(!nothing.has_variant());
}

// ------------------------------------------------------------ merge strategies

/// Records why it was called, and writes the patch's value anyway.
struct Overwrite;

impl EnumStructTestMergeStrategy for Overwrite {
    type Error = EnumMergeError<EnumStructTestFields>;

    fn resolve_variant(
        &self,
        _current: &EnumStructTest,
        _old: &EnumStructTest,
        _new: &EnumStructTestPartial,
        conflict: &VariantConflict<EnumStructTestFields>,
    ) -> Result<Resolution<EnumStructTest>, Self::Error> {
        match conflict {
            VariantConflict::OtherVariant => {
                Ok(Resolution::Use(EnumStructTest::SingleValue { foo: 99 }))
            },
            VariantConflict::Fields(fields) => {
                assert_eq!(fields, &[EnumStructTestFields::MultipleValuesZee]);
                Ok(Resolution::UseNew)
            },
        }
    }
}

#[test]
fn a_changed_field_during_a_variant_change_goes_to_resolve_variant() {
    let patch = multiple().edit(|bound| bound.single_value().set_foo(5));
    let mut value = EnumStructTest::MultipleValues {
        bar: 1,
        zee: "x".into(),
    };

    let rejected = value.apply_patch(patch.clone()).unwrap_err();
    assert_eq!(
        rejected.error,
        conflict(EnumConflict::Field(EnumStructTestFields::MultipleValuesZee))
    );

    value.apply_patch_with_strategy(patch, &Overwrite).unwrap();
    assert_eq!(value, EnumStructTest::SingleValue { foo: 5 });
}

#[test]
fn resolve_variant_can_write_its_own_value() {
    let patch = multiple().edit(|bound| bound.multiple_values().set_bar(2));
    let mut value = EnumStructTest::NoValue;
    value.apply_patch_with_strategy(patch, &Overwrite).unwrap();
    assert_eq!(value, EnumStructTest::SingleValue { foo: 99 });
}

/// Fills a missing `zee` with `"default"` when building another variant.
struct Defaults;

impl EnumStructTestMergeStrategy for Defaults {
    type Error = EnumMergeError<EnumStructTestFields>;

    fn build_new(
        &self,
        new: &EnumStructTestPartial,
    ) -> Result<EnumStructTest, Self::Error> {
        let mut new = new.clone();
        if new.variant() == Some(EnumStructTestVariants::MultipleValues)
            && !new.has(EnumStructTestFields::MultipleValuesZee)
        {
            new.multiple_values().set_zee("default".into());
        }
        new.build().map_err(|rejected| {
            match rejected.error {
                partly::BuildFailure::Strategy(missing) => missing.into(),
                // `Fields` holds a `Vec`, which can be empty, so the compiler
                // can't see that `AcceptAll` makes this impossible.
                partly::BuildFailure::Validation(_) => {
                    unreachable!("`AcceptAll` accepts every value")
                },
            }
        })
    }
}

// ------------------------------------------------------------ undoing

#[test]
fn invert_undoes_a_patch_of_the_same_variant() {
    let patch = multiple().edit(|bound| bound.multiple_values().set_bar(2));
    let mut value = multiple();
    value.apply_patch(patch.clone()).unwrap();
    value.apply_patch(patch.invert().unwrap()).unwrap();
    assert_eq!(value, multiple());
}

#[test]
fn invert_undoes_a_variant_change_that_builds() {
    let patch = multiple().edit(|bound| bound.single_value().set_foo(5));
    let mut value = multiple();
    value.apply_patch(patch.clone()).unwrap();
    value.apply_patch(patch.invert().unwrap()).unwrap();
    assert_eq!(value, multiple());
}

#[test]
fn only_the_undo_is_exact_when_the_strategy_builds_the_new_value() {
    let patch = EnumStructTest::NoValue
        .edit(|bound| bound.multiple_values().set_bar(1));

    // `invert` uses the default strategy, which can't build this.
    let rejected = patch.clone().invert().unwrap_err();
    assert_eq!(
        rejected.error,
        MissingValue::Field(EnumStructTestFields::MultipleValuesZee)
    );

    let mut value = EnumStructTest::NoValue;
    let undo = value
        .apply_patch_with_get_undo(patch, &Defaults, &AcceptAll)
        .unwrap();
    assert_eq!(value, EnumStructTest::MultipleValues {
        bar: 1,
        zee: "default".into(),
    });
    assert_eq!(undo.old_value(), Some(&value));
    value.apply_patch(undo).unwrap();
    assert_eq!(value, EnumStructTest::NoValue);
}

#[test]
fn an_undo_of_field_changes_writes_back_only_those_fields() {
    let mut value = multiple();
    let undo = value
        .apply_patch_get_undo(
            multiple()
                .edit(|bound| bound.multiple_values().set_zee("b".into())),
        )
        .unwrap();
    assert!(!undo.changes_variant());
    assert!(undo.has(EnumStructTestFields::MultipleValuesZee));
    assert!(!undo.has(EnumStructTestFields::MultipleValuesBar));
    value.apply_patch(undo).unwrap();
    assert_eq!(value, multiple());
}

// ------------------------------------------------------------ combining

#[test]
fn then_follows_a_variant_change() {
    let first = multiple().edit(|bound| bound.single_value().set_foo(5));
    let next = EnumStructTest::SingleValue { foo: 5 }
        .edit(|bound| bound.single_value().set_foo(6));
    let combined = first.clone().then(next).unwrap();

    let mut value = multiple();
    value.apply_patch(combined).unwrap();
    assert_eq!(value, EnumStructTest::SingleValue { foo: 6 });

    // Made against `multiple()`, but `first` leaves a `SingleValue`.
    let stale = multiple().edit(|bound| bound.multiple_values().set_bar(2));
    let rejected = first.then(stale).unwrap_err();
    assert_eq!(
        rejected.error,
        ChainFailure::Conflict(EnumConflict::Variant)
    );
}

#[test]
fn then_fails_if_the_first_patch_does_not_build() {
    let first = EnumStructTest::NoValue
        .edit(|bound| bound.multiple_values().set_bar(1));
    let rejected = first.then(EnumStructTestPatch::new()).map(|_| ());
    assert!(rejected.is_ok());

    let first = EnumStructTest::NoValue
        .edit(|bound| bound.multiple_values().set_bar(1));
    let next = EnumStructTest::NoValue.edit(|bound| {
        bound.no_value();
    });
    let rejected = first.then(next).unwrap_err();
    assert_eq!(
        rejected.error,
        ChainFailure::Build(MissingValue::Field(
            EnumStructTestFields::MultipleValuesZee
        ))
    );
}

#[test]
fn merge_combines_fields_of_the_same_variant_only() {
    let bar = multiple().edit(|bound| bound.multiple_values().set_bar(2));
    let zee =
        multiple().edit(|bound| bound.multiple_values().set_zee("b".into()));
    let merged = bar.clone().merge(zee).unwrap();
    assert!(merged.is_complete());

    let other = multiple().edit(|bound| {
        bound.no_value();
    });
    let rejected = bar.clone().merge(other).unwrap_err();
    assert_eq!(rejected.error, EnumConflict::Variant);

    let other_base = EnumStructTest::NoValue
        .edit(|bound| bound.multiple_values().set_bar(2));
    let rejected = bar.merge(other_base).unwrap_err();
    assert_eq!(rejected.error, EnumConflict::Variant);
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

    fn validate(&self, value: &EnumStructTest) -> Result<(), Self::Error> {
        if matches!(value, EnumStructTest::NoValue) {
            Err("no value")
        } else {
            Ok(())
        }
    }
}

#[test]
fn a_new_variant_is_validated_before_it_is_written() {
    let mut value = multiple();
    let zero = multiple().edit(|bound| bound.single_value().set_foo(0));
    let rejected = value
        .apply_patch_with(zero.clone(), &StrictMerge, &NonZero)
        .unwrap_err();
    assert!(matches!(
        rejected.error,
        ApplyFailure::Validation(ValidationErrors::Fields(_))
    ));
    assert_eq!(rejected.value, zero);

    let none = multiple().edit(|bound| {
        bound.no_value();
    });
    let rejected = value
        .apply_patch_with(none, &StrictMerge, &NonZero)
        .unwrap_err();
    assert_eq!(
        rejected.error,
        ApplyFailure::Validation(ValidationErrors::Struct("no value"))
    );
    assert_eq!(value, multiple());
}

// ------------------------------------------------------------ tuple variants

#[test]
fn a_tuple_enum_patch_writes_by_position_or_replaces() {
    let start = EnumTupleTest::MultipleValues(1, 2, "a".into());
    let mut value = start.clone();
    value
        .apply_patch(start.edit(|bound| bound.multiple_values().set_1(9)))
        .unwrap();
    assert_eq!(value, EnumTupleTest::MultipleValues(1, 9, "a".into()));

    let undo = value
        .apply_patch_get_undo(value.edit(|bound| bound.single_value().set_0(3)))
        .unwrap();
    assert_eq!(value, EnumTupleTest::SingleValue(3));
    value.apply_patch(undo).unwrap();
    assert_eq!(value, EnumTupleTest::MultipleValues(1, 9, "a".into()));
}
