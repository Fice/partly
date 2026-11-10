//! Patches: creating them, applying them and combining them.

use fieldmuck::ReadField;
use partly::prelude::*;
use partly::shape_patch::Base;
use partly::{
    AcceptAll, ApplyFailure, ChainFailure, ChangeRecorder, Conflict,
    FieldChange, FieldMerge, PatchValue, Resolution, StrictMerge,
    ValidationErrors,
};
// Named methods on the generic patch types come from generated traits, all
// in the type's prelude.
use partly_fixtures_basic::foo::foo_prelude::*;
use partly_fixtures_basic::foo::{
    BoundFooPatch, Foo, FooFields, FooMergeStrategy, FooPatch, FooShape,
    FooValidator, foo_field,
};

/// `foo()` as the base of a patch: the value of every patched field.
fn foo_base() -> Base<Foo> {
    FooShape::from_slots(1, "a".into())
}

fn foo() -> Foo {
    Foo {
        bar: 1,
        zee: "a".into(),
    }
}

/// A patch made against `foo()` that writes `bar`.
fn change_bar(new: u32) -> FooPatch {
    foo().edit(|bound| bound.set_bar(new))
}

/// A patch made against `foo()` that writes `zee`.
fn change_zee(new: &str) -> FooPatch {
    foo().edit(|bound| bound.set_zee(new.into()))
}

#[test]
fn a_patch_reads_as_the_value_after_it() {
    let patch = change_bar(2);
    assert_eq!(
        ReadField::<Foo, foo_field::Bar>::read_field(&patch),
        Some(&2)
    );
    assert_eq!(
        ReadField::<Foo, foo_field::Zee>::read_field(&patch)
            .map(String::as_str),
        Some("a")
    );
    assert_eq!(
        ReadField::<Foo, foo_field::Bar>::read_field(&FooPatch::new()),
        None
    );
}

#[test]
fn a_patch_shows_what_it_does_to_each_field() {
    let patch = change_bar(2);
    let data = patch.data().unwrap();
    assert_eq!(data.bar_slot(), FieldChange::Changed { old: &1, new: &2 });
    assert_eq!(data.zee_slot(), FieldChange::Unchanged(&"a".to_owned()));
}

#[test]
fn patch_value_helpers() {
    let change = PatchValue::new(1, 2);
    assert_eq!(PatchValue::from((1, 2)), change);
    assert!(!change.is_noop());
    assert!(PatchValue::new(1, 1).is_noop());
    assert_eq!(change.invert(), PatchValue::new(2, 1));
}

// ------------------------------------------------------------ making patches

#[test]
fn a_bound_patch_reads_old_values_from_the_struct() {
    let value = foo();
    let mut bound = value.patch();
    bound.set_bar(15);
    assert_eq!(bound.bar(), Some(PatchValue::new(&1, &15)));

    // Setting a field again only replaces the value to write.
    bound.set_bar(17);
    assert_eq!(bound.finish().bar(), Some(PatchValue::new(&1, &17)));
}

#[test]
fn finishing_a_bound_patch_releases_the_borrow() {
    let mut value = foo();
    let mut bound = BoundFooPatch::new(&value, ChangeRecorder::new());
    bound.set_bar(15);
    let patch = bound.clone().finish();

    // `value` is mutable again here, because `patch` owns a copy of it.
    value.apply_patch(patch).unwrap();
    assert_eq!(value.bar, 15);
}

#[test]
fn edit_builds_a_patch_in_a_closure() {
    let patch = change_zee("b");
    assert_eq!(
        patch.zee(),
        Some(PatchValue::new(&"a".to_owned(), &"b".to_owned()))
    );
    assert!(!patch.has_bar());
}

#[test]
fn a_patch_holds_the_value_of_every_patched_field() {
    let patch = change_bar(2);
    assert_eq!(patch.old_value(), Some(&foo_base()));
    assert_eq!(patch.new_value().and_then(|new| new.bar), Some(2));

    let (old, new) = patch.into_parts().unwrap();
    assert_eq!(old, foo_base());
    assert_eq!(FooPatch::from_parts(old, new), change_bar(2));

    assert_eq!(FooPatch::new().old_value(), None);
    assert_eq!(FooPatch::new().into_parts(), None);
}

#[test]
fn diff_writes_only_the_fields_that_differ() {
    let new = Foo {
        bar: 2,
        zee: "a".into(),
    };
    assert_eq!(foo().diff(&new), change_bar(2));

    // A diff of equal values writes nothing, but is still made against them.
    let same = foo().diff(&foo());
    assert!(same.is_empty());
    assert_eq!(same.old_value(), Some(&foo_base()));
}

#[test]
fn empty_complete_and_noop() {
    assert!(FooPatch::new().is_empty());
    assert!(FooPatch::new().is_noop());

    let both = foo().edit(|bound| {
        bound.set_bar(2);
        bound.set_zee("b".into());
    });
    assert!(both.is_complete());
    assert!(!both.is_empty());
    assert!(!both.is_noop());
    assert!(change_bar(1).is_noop());
    assert!(!change_bar(1).is_complete());
    assert_eq!(both.fields(), [FooFields::Bar, FooFields::Zee]);
}

// ------------------------------------------------------------ applying

#[test]
fn a_matching_patch_applies() {
    let mut value = foo();
    value.apply_patch(change_bar(2)).unwrap();
    assert_eq!(value.bar, 2);
    assert_eq!(value.zee, "a");
}

#[test]
fn a_patch_on_a_changed_field_is_rejected_and_handed_back() {
    let mut value = foo();
    value.bar = 32;
    let patch = change_bar(2);

    let rejected = value.apply_patch(patch.clone()).unwrap_err();
    assert_eq!(
        rejected.error,
        ApplyFailure::Merge(Conflict(FooFields::Bar))
    );
    assert_eq!(rejected.value, patch);
    assert_eq!(value.bar, 32);
}

#[test]
fn the_value_must_match_exactly_even_on_fields_the_patch_does_not_write() {
    let mut value = foo();
    value.zee = "changed".into();

    let rejected = value.apply_patch(change_bar(2)).unwrap_err();
    assert_eq!(
        rejected.error,
        ApplyFailure::Merge(Conflict(FooFields::Zee))
    );
    assert_eq!(value.bar, 1);
}

#[test]
fn check_lists_every_difference_without_changing_anything() {
    let mut value = foo();
    assert_eq!(change_bar(2).check(&value), Ok(()));

    value.bar = 9;
    value.zee = "x".into();
    assert_eq!(
        change_bar(2).check(&value),
        Err(vec![Conflict(FooFields::Bar), Conflict(FooFields::Zee)])
    );
    assert_eq!(FooPatch::new().check(&value), Ok(()));
}

#[test]
fn an_empty_patch_applies_to_anything() {
    let mut value = foo();
    value.bar = 9;
    value.apply_patch(FooPatch::new()).unwrap();
    assert_eq!(value.bar, 9);
}

#[test]
fn field_merge_keeps_changes_to_fields_the_patch_does_not_write() {
    let mut value = foo();
    value.zee = "changed".into();
    value
        .apply_patch_with_strategy(change_bar(2), &FieldMerge)
        .unwrap();
    assert_eq!(value.bar, 2);
    assert_eq!(value.zee, "changed");

    // A field the patch writes still conflicts.
    let mut value = foo();
    value.bar = 9;
    let rejected = value
        .apply_patch_with_strategy(change_bar(2), &FieldMerge)
        .unwrap_err();
    assert_eq!(
        rejected.error,
        ApplyFailure::Merge(Conflict(FooFields::Bar))
    );
}

#[test]
fn two_patches_from_the_same_value_need_field_merge_or_merge() {
    let mut value = foo();
    value.apply_patch(change_bar(2)).unwrap();
    assert!(value.apply_patch(change_zee("b")).is_err());
    value
        .apply_patch_with_strategy(change_zee("b"), &FieldMerge)
        .unwrap();

    let mut merged = foo();
    merged
        .apply_patch(change_bar(2).merge(change_zee("b")).unwrap())
        .unwrap();
    assert_eq!(merged, value);
}

#[test]
fn into_partial_keeps_the_values_to_write() {
    let mut value = foo();
    value.bar = 32;
    value.apply_partial(change_bar(2).into_partial()).unwrap();
    assert_eq!(value.bar, 2);
}

// ------------------------------------------------------------ undoing

#[test]
fn invert_undoes_a_patch() {
    let mut value = foo();
    let patch = change_bar(2);
    value.apply_patch(patch.clone()).unwrap();

    // Inverting a struct patch can't fail.
    let Ok(inverse) = patch.invert();
    assert_eq!(inverse.old_value().map(|old| old.bar), Some(2));
    value.apply_patch(inverse).unwrap();
    assert_eq!(value, foo());
}

#[test]
fn apply_patch_get_undo_gives_the_patch_that_undoes_it() {
    let mut value = foo();
    let undo = value.apply_patch_get_undo(change_bar(2)).unwrap();
    assert_eq!(value.bar, 2);
    value.apply_patch(undo).unwrap();
    assert_eq!(value, foo());

    let nothing = value.apply_patch_get_undo(FooPatch::new()).unwrap();
    assert!(nothing.is_empty());
}

#[test]
fn an_undo_also_restores_a_value_the_strategy_chose() {
    let mut value = foo();
    value.bar = 9;
    let undo = value
        .apply_patch_with_get_undo(change_bar(2), &Fixed, &AcceptAll)
        .unwrap();
    assert_eq!(value.bar, 500);
    value.apply_patch(undo).unwrap();
    assert_eq!(value.bar, 9);
}

// ------------------------------------------------------------ merge strategies

/// Keeps the current `bar`. For `zee`, marks a current value longer than the
/// new one with `!`, and otherwise writes the patch's value.
struct Lenient;

impl FooMergeStrategy for Lenient {
    type Error = Conflict<FooFields>;

    fn resolve_bar(
        &self,
        _current: &u32,
        _old: &u32,
        _new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        Ok(Resolution::KeepCurrent)
    }

    fn resolve_zee(
        &self,
        current: &String,
        _old: &String,
        new: Option<&String>,
    ) -> Result<Resolution<String>, Self::Error> {
        if new.is_some_and(|new| current.len() > new.len()) {
            Ok(Resolution::Use(format!("{current}!")))
        } else {
            Ok(Resolution::UseNew)
        }
    }
}

/// Writes 500 into a conflicting `bar`.
struct Fixed;

impl FooMergeStrategy for Fixed {
    type Error = Conflict<FooFields>;

    fn resolve_bar(
        &self,
        _current: &u32,
        _old: &u32,
        _new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        Ok(Resolution::Use(500))
    }
}

#[test]
fn a_merge_strategy_can_keep_the_current_value() {
    let mut value = foo();
    value.bar = 32;
    value
        .apply_patch_with_strategy(change_bar(2), &Lenient)
        .unwrap();
    assert_eq!(value.bar, 32);
}

#[test]
fn a_merge_strategy_can_use_the_new_value_or_its_own() {
    let mut value = foo();
    value.zee = "current".into();
    value
        .apply_patch_with_strategy(change_zee("a-much-longer-value"), &Lenient)
        .unwrap();
    assert_eq!(value.zee, "a-much-longer-value");

    value
        .apply_patch_with_strategy(change_zee("b"), &Lenient)
        .unwrap();
    assert_eq!(value.zee, "a-much-longer-value!");
}

#[test]
fn a_merge_strategy_runs_for_a_field_the_patch_does_not_write() {
    struct Records;
    impl FooMergeStrategy for Records {
        type Error = Conflict<FooFields>;

        fn resolve_zee(
            &self,
            current: &String,
            old: &String,
            new: Option<&String>,
        ) -> Result<Resolution<String>, Self::Error> {
            assert_eq!((current.as_str(), old.as_str(), new), ("x", "a", None));
            // `UseNew` without a new value leaves the field as it is.
            Ok(Resolution::UseNew)
        }
    }

    let mut value = foo();
    value.zee = "x".into();
    value
        .apply_patch_with_strategy(change_bar(2), &Records)
        .unwrap();
    assert_eq!(value.zee, "x");
    assert_eq!(value.bar, 2);
}

#[test]
fn a_merge_strategy_only_runs_on_conflicts() {
    struct Panics;
    impl FooMergeStrategy for Panics {
        type Error = Conflict<FooFields>;

        fn resolve_bar(
            &self,
            _current: &u32,
            _old: &u32,
            _new: Option<&u32>,
        ) -> Result<Resolution<u32>, Self::Error> {
            panic!("the old value matched");
        }
    }

    let mut value = foo();
    value
        .apply_patch_with_strategy(change_bar(2), &Panics)
        .unwrap();
    assert_eq!(value.bar, 2);
}

// ------------------------------------------------------------ validation

struct Rules;

impl FooValidator for Rules {
    type Error = &'static str;

    fn validate_zee(&self, value: &String) -> Result<(), Self::Error> {
        if value.is_empty() {
            Err("empty")
        } else {
            Ok(())
        }
    }

    fn validate(&self, value: &Foo) -> Result<(), Self::Error> {
        if value.bar > 100 {
            Err("too big")
        } else {
            Ok(())
        }
    }
}

#[test]
fn a_field_validation_failure_hands_back_the_whole_patch() {
    let mut value = foo();
    let patch = change_zee("");
    let rejected = value
        .apply_patch_with(patch.clone(), &StrictMerge, &Rules)
        .unwrap_err();
    assert!(matches!(
        rejected.error,
        ApplyFailure::Validation(ValidationErrors::Fields(_))
    ));
    assert_eq!(rejected.value, patch);
    assert_eq!(value, foo());
}

#[test]
fn a_whole_struct_failure_rolls_back_and_hands_back_the_whole_patch() {
    let mut value = foo();
    let patch = foo().edit(|bound| {
        bound.set_bar(200);
        bound.set_zee("b".into());
    });
    let rejected = value
        .apply_patch_with(patch.clone(), &StrictMerge, &Rules)
        .unwrap_err();
    assert_eq!(
        rejected.error,
        ApplyFailure::Validation(ValidationErrors::Struct("too big"))
    );
    assert_eq!(rejected.value, patch);
    assert_eq!(value, foo());
}

#[test]
fn a_rollback_also_restores_a_value_chosen_by_the_strategy() {
    let mut value = foo();
    value.bar = 9;
    let patch = change_bar(2);
    let rejected = value
        .apply_patch_with(patch.clone(), &Fixed, &Rules)
        .unwrap_err();
    assert_eq!(rejected.value, patch);
    assert_eq!(value.bar, 9);
}

#[test]
fn validation_applies_to_the_value_the_strategy_chose() {
    struct Empty;
    impl FooMergeStrategy for Empty {
        type Error = Conflict<FooFields>;

        fn resolve_zee(
            &self,
            _current: &String,
            _old: &String,
            _new: Option<&String>,
        ) -> Result<Resolution<String>, Self::Error> {
            Ok(Resolution::Use(String::new()))
        }
    }

    let mut value = foo();
    value.zee = "stale".into();
    let rejected = value
        .apply_patch_with(change_zee("b"), &Empty, &Rules)
        .unwrap_err();
    assert!(matches!(
        rejected.error,
        ApplyFailure::Validation(ValidationErrors::Fields(_))
    ));
    assert_eq!(value.zee, "stale");
}

// ------------------------------------------------------------ combining

#[test]
fn then_combines_patches_applied_one_after_the_other() {
    let after_first = Foo {
        bar: 2,
        zee: "a".into(),
    };
    let combined = change_bar(2)
        .then(after_first.edit(|bound| bound.set_bar(3)))
        .unwrap();
    assert_eq!(combined, change_bar(3));

    let separate = change_bar(2)
        .then(after_first.edit(|bound| bound.set_zee("b".into())))
        .unwrap();
    assert!(separate.has_bar() && separate.has_zee());
}

#[test]
fn then_rejects_a_patch_that_does_not_continue_from_the_first() {
    let first = change_bar(2);
    // Made against `foo()`, but `first` leaves `bar` at 2.
    let next = change_bar(3);
    let rejected = first.clone().then(next.clone()).unwrap_err();
    assert_eq!(
        rejected.error,
        ChainFailure::Conflict(Conflict(FooFields::Bar))
    );
    assert_eq!(rejected.value, (first, next));
}

#[test]
fn merge_combines_patches_from_the_same_value() {
    let merged = change_bar(2).merge(change_zee("b")).unwrap();
    assert!(merged.is_complete());

    let same = change_bar(2).merge(change_bar(2)).unwrap();
    assert_eq!(same, change_bar(2));
}

#[test]
fn merge_rejects_different_changes_or_different_values() {
    let left = change_bar(2);
    let right = change_bar(3);
    let rejected = left.clone().merge(right.clone()).unwrap_err();
    assert_eq!(rejected.error, Conflict(FooFields::Bar));
    assert_eq!(rejected.value, (left, right));

    let other_value = Foo {
        bar: 1,
        zee: "x".into(),
    };
    let rejected = change_bar(2)
        .merge(other_value.edit(|bound| bound.set_bar(2)))
        .unwrap_err();
    assert_eq!(rejected.error, Conflict(FooFields::Zee));
}

#[test]
fn the_values_to_write_can_be_changed() {
    let mut patch = change_bar(2);
    patch.new_value_mut().unwrap().set_zee("b".into());
    assert_eq!(
        patch.zee(),
        Some(PatchValue::new(&"a".to_owned(), &"b".to_owned()))
    );
    assert!(FooPatch::new().new_value_mut().is_none());
    assert_eq!(FooPatch::default(), FooPatch::new());
}

#[test]
fn the_explicit_form_matches_the_default() {
    let mut value = foo();
    value
        .apply_patch_with(change_bar(2), &StrictMerge, &AcceptAll)
        .unwrap();
    assert_eq!(value.bar, 2);
}
