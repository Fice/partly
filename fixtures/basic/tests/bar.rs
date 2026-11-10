//! A struct mixing a plain value, a reference, an `Option`, and a type with no
//! traits. `Bar` and `BarPartial` have no `Debug`-based or `Clone`-based
//! shortcuts here, so these tests compare field by field.

use partly::prelude::*;
use partly::{
    ApplyFailure, BuildFailure, Conflict, FieldMerge, MissingField, PatchValue,
    Resolution, StrictMerge, ValidationErrors,
};
use partly_fixtures_basic::bar::bar_prelude::*;
use partly_fixtures_basic::bar::{
    Bar, BarBuildStrategy, BarFieldRef, BarFieldValue, BarFields,
    BarMergeStrategy, BarPartial, BarPatch, BarShape, BarValidator, Resource,
};

fn bar<'a>() -> Bar<'a> {
    Bar {
        count: 1,
        label: "a",
        nick: None,
        resource: Resource(7),
    }
}

fn complete<'a>() -> BarPartial<'a> {
    BarPartial::new()
        .with_count(1)
        .with_label("a")
        .with_nick(Some("n".into()))
        .with_resource(Resource(7))
}

#[test]
fn a_complete_partial_builds_with_every_field_kind() {
    let built = complete().build().unwrap();
    assert_eq!(built.count, 1);
    assert_eq!(built.label, "a");
    assert_eq!(built.nick.as_deref(), Some("n"));
    assert_eq!(built.resource.0, 7);
}

#[test]
fn a_reference_can_borrow_from_a_shorter_lived_value() {
    let owned = String::from("borrowed");
    let partial = complete().with_label(&owned);
    let built = partial.build().unwrap();
    assert_eq!(built.label, "borrowed");
}

#[test]
fn an_option_field_separates_unset_from_set_to_none() {
    let mut partial = BarPartial::new();
    assert!(!partial.has_nick());
    partial.set_nick(None);
    assert!(partial.has_nick());
    assert_eq!(partial.nick(), Some(&None));
    assert_eq!(partial.take_nick(), Some(None));
    assert!(!partial.has_nick());
}

#[test]
fn accessors_work_on_a_field_with_no_traits() {
    let mut partial = BarPartial::new().with_resource(Resource(1));
    assert_eq!(partial.resource().map(|value| value.0), Some(1));
    partial.resource_mut().unwrap().0 = 2;
    assert_eq!(partial.take_resource().map(|value| value.0), Some(2));
    partial.set_resource(Resource(3));
    partial.clear_resource();
    assert!(partial.is_empty());
}

#[test]
fn accessors_work_on_a_reference() {
    let mut partial = BarPartial::new().with_label("a");
    assert_eq!(partial.label(), Some(&"a"));
    *partial.label_mut().unwrap() = "b";
    assert_eq!(partial.take_label(), Some("b"));
    assert!(partial.is_empty());
}

#[test]
fn require_moves_each_kind_to_a_compile_time_guarantee() {
    let typed = complete()
        .require_count()
        .ok()
        .unwrap()
        .require_label()
        .ok()
        .unwrap()
        .require_nick()
        .ok()
        .unwrap()
        .require_resource()
        .ok()
        .unwrap();
    assert_eq!(*typed.count(), 1);
    assert_eq!(*typed.label(), "a");
    assert_eq!(typed.nick().as_deref(), Some("n"));
    assert_eq!(typed.resource().0, 7);
}

#[test]
fn debug_prints_every_field_and_hides_the_one_with_no_traits() {
    // `Bar`'s hook prints `resource` as `<opaque>` in every representation.
    assert_eq!(
        format!("{:?}", complete()),
        r#"BarPartial { count: Some(1), label: Some("a"), nick: Some(Some("n")), resource: <opaque> }"#
    );
    assert_eq!(
        format!("{:?}", BarPartial::new()),
        "BarPartial { count: None, label: None, nick: None, resource: \
         <opaque> }"
    );
}

#[test]
fn a_rejected_build_can_be_printed_and_keeps_the_resource() {
    let partial = BarPartial::new().with_resource(Resource(7));
    let rejected = partial.build().map(|_| ()).unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingField(BarFields::Count))
    );
    assert_eq!(rejected.value.resource().map(|value| value.0), Some(7));
    assert!(format!("{rejected:?}").contains("resource: <opaque>"));
}

#[test]
fn resolved_values_come_back_when_a_later_field_is_missing() {
    let partial = BarPartial::new().with_count(1).with_label("a");
    let rejected = partial.build().map(|_| ()).unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingField(BarFields::Nick))
    );
    assert_eq!(rejected.value.count(), Some(&1));
    assert_eq!(rejected.value.label(), Some(&"a"));
}

/// Creates a resource and a nick when they are missing, and rounds `count`.
struct Fill;

impl<'a> BarBuildStrategy<'a> for Fill {
    type Error = MissingField<BarFields>;

    fn resolve_count(&self, value: Option<u32>) -> Result<u32, Self::Error> {
        Ok(value.unwrap_or(10) / 10 * 10)
    }

    fn resolve_nick(
        &self,
        value: Option<Option<String>>,
    ) -> Result<Option<String>, Self::Error> {
        Ok(value.flatten())
    }

    fn resolve_resource(
        &self,
        value: Option<Resource>,
    ) -> Result<Resource, Self::Error> {
        Ok(value.unwrap_or(Resource(99)))
    }
}

#[test]
fn a_strategy_can_create_a_value_with_no_traits() {
    let partial = BarPartial::new().with_count(17).with_label("a");
    let built = partial.build_with_strategy(&Fill).ok().unwrap();
    assert_eq!(built.count, 10);
    assert_eq!(built.nick, None);
    assert_eq!(built.resource.0, 99);
}

struct Rules;

impl<'a> BarValidator<'a> for Rules {
    type Error = &'static str;

    fn validate_label(&self, value: &&'a str) -> Result<(), Self::Error> {
        if value.is_empty() {
            Err("empty")
        } else {
            Ok(())
        }
    }

    fn validate_resource(&self, value: &Resource) -> Result<(), Self::Error> {
        if value.0 == 0 { Err("zero") } else { Ok(()) }
    }

    fn validate(&self, value: &Bar<'a>) -> Result<(), Self::Error> {
        if value.nick.as_deref() == Some(value.label) {
            Err("nick must differ from label")
        } else {
            Ok(())
        }
    }
}

#[test]
fn every_field_error_is_collected_including_the_no_trait_field() {
    let invalid = complete()
        .with_label("")
        .with_resource(Resource(0))
        .build_with(&partly::DefaultBuildStrategy, &Rules)
        .map(|_| ())
        .unwrap_err();
    let BuildFailure::Validation(ValidationErrors::Fields(errors)) =
        invalid.error
    else {
        panic!("expected field errors");
    };
    let fields: Vec<_> = errors.iter().map(|item| item.field).collect();
    assert_eq!(fields, [BarFields::Label, BarFields::Resource]);
    // The rejected partial still owns the resource.
    assert_eq!(invalid.value.resource().map(|value| value.0), Some(0));
}

#[test]
fn apply_partial_moves_a_value_with_no_traits_in() {
    let mut value = bar();
    value
        .apply_partial(BarPartial::new().with_resource(Resource(8)))
        .unwrap();
    assert_eq!(value.resource.0, 8);
    assert_eq!(value.count, 1);
}

#[test]
fn a_rollback_returns_a_value_with_no_traits_to_both_sides() {
    let mut value = bar();
    let input = BarPartial::new()
        .with_label("same")
        .with_nick(Some("same".into()))
        .with_resource(Resource(8));
    let rejected = value.apply_partial_with(input, &Rules).unwrap_err();
    assert_eq!(
        rejected.error,
        ValidationErrors::Struct("nick must differ from label")
    );
    assert_eq!(value.resource.0, 7);
    assert_eq!(value.label, "a");
    assert_eq!(value.nick, None);
    assert_eq!(rejected.value.resource().map(|item| item.0), Some(8));
    assert_eq!(rejected.value.label(), Some(&"same"));
}

#[test]
fn a_struct_converts_into_a_partial_without_clone() {
    let partial = bar().into_partial();
    assert!(partial.is_complete());
    assert_eq!(partial.resource().map(|value| value.0), Some(7));
}

#[test]
fn merge_moves_values_with_no_traits() {
    let left = BarPartial::new().with_resource(Resource(1)).with_count(1);
    let right = BarPartial::new().with_resource(Resource(2));
    let merged = left.merge(right);
    assert_eq!(merged.resource().map(|value| value.0), Some(2));
    assert_eq!(merged.count(), Some(&1));
}

#[test]
fn reflection_handles_every_field_kind() {
    let partial = complete();
    let fields: Vec<_> =
        partial.iter().map(|item| format!("{item:?}")).collect();
    assert_eq!(fields, [
        "Count(1)",
        r#"Label("a")"#,
        r#"Nick(Some("n"))"#,
        "Resource(<opaque>)",
    ]);
    assert!(matches!(
        partial.iter().next(),
        Some(BarFieldRef::Count(&1))
    ));

    let values: Vec<_> = partial.into_iter().collect();
    assert_eq!(values.len(), 4);
    assert_eq!(values[3].field(), BarFields::Resource);

    let rebuilt: BarPartial<'_> = values.into_iter().collect();
    assert!(rebuilt.is_complete());
}

#[test]
fn field_values_without_the_trait_still_have_the_others() {
    // `Clone` and `PartialEq` aren't available here because of `Resource`, so
    // this checks `Debug` and moving the value through `set_field`.
    let mut partial = BarPartial::new();
    assert!(
        partial
            .set_field(BarFieldValue::Resource(Resource(1)))
            .is_none()
    );
    let replaced = partial.set_field(BarFieldValue::Resource(Resource(2)));
    assert_eq!(format!("{:?}", replaced.unwrap()), "Resource(<opaque>)");
    let taken = partial.take_field(BarFields::Resource).unwrap();
    assert_eq!(taken.field(), BarFields::Resource);
}

/// A patch made against `bar()` that writes `count`.
fn change_count(new: u32) -> BarPatch<'static> {
    bar().edit(|bound| bound.set_count(new))
}

#[test]
fn a_skipped_field_is_not_part_of_a_patch() {
    let patch = bar().edit(|bound| {
        bound.set_count(2);
        bound.set_label("b");
        bound.set_nick(None);
    });
    // Complete, even though `resource` can't be written.
    assert!(patch.is_complete());
    assert!(!patch.has(BarFields::Resource));
    assert_eq!(patch.fields(), [
        BarFields::Count,
        BarFields::Label,
        BarFields::Nick
    ]);
    assert!(!BarPatch::new().is_complete());
    assert!(BarPatch::new().is_empty());
}

#[test]
fn a_patch_stays_clone_when_the_struct_is_not() {
    let patch = change_count(2);
    assert_eq!(patch.clone(), patch);
}

#[test]
fn a_patch_is_made_against_every_field_but_the_skipped_one() {
    let patch = change_count(2);
    let old = patch.old_value().unwrap();
    assert_eq!(old.count, 1);
    assert_eq!(old.label, "a");
    assert_eq!(old.nick, None);
    // The base holds nothing for the skipped field.
    assert_eq!(old.resource, partly::Skip::new());
}

#[test]
fn a_base_can_be_built_by_hand_with_a_skipped_slot() {
    // `PatchBase` accepts both a value and a `Skip`, so this compiles.
    let base: partly::shape_patch::Base<Bar<'_>> =
        BarShape::from_slots(1, "a", None, partly::Skip::new());
    assert_eq!(base.count, 1);
}

#[test]
fn a_skipped_field_can_change_without_a_conflict() {
    let mut value = bar();
    value.resource = Resource(99);
    value.apply_patch(change_count(2)).unwrap();
    assert_eq!(value.count, 2);
    assert_eq!(value.resource.0, 99);
}

#[test]
fn a_patch_leaves_a_field_with_no_traits_alone() {
    let mut value = bar();
    let patch = bar().edit(|bound| {
        bound.set_label("b");
        bound.set_nick(Some("n".into()));
    });
    value.apply_patch(patch).unwrap();
    assert_eq!(value.label, "b");
    assert_eq!(value.nick.as_deref(), Some("n"));
    assert_eq!(value.resource.0, 7);
}

#[test]
fn a_bound_patch_reads_each_old_value() {
    let value = bar();
    let patch = value.edit(|bound| {
        bound.set_count(2);
        bound.set_label("b");
        bound.set_nick(Some("n".into()));
    });
    assert_eq!(patch.count(), Some(PatchValue::new(&1, &2)));
    assert_eq!(patch.label(), Some(PatchValue::new(&"a", &"b")));
    assert_eq!(
        patch.nick(),
        Some(PatchValue::new(&None, &Some("n".into())))
    );
}

#[test]
fn a_bound_patch_prints_through_the_hook() {
    let value = bar();
    let mut bound = value.patch();
    bound.set_count(2);
    assert_eq!(
        format!("{bound:?}"),
        r#"BarStaged { count: Staged(2), label: Unchanged("a"), nick: Unchanged(None), resource: <opaque> }"#
    );
}

#[test]
fn a_patch_prints_what_it_does_and_skips_the_skipped_field() {
    assert_eq!(
        format!("{:?}", change_count(2)),
        r#"BarPatch { count: Changed { old: 1, new: 2 }, label: Unchanged("a"), nick: Unchanged(None), resource: <opaque> }"#
    );
}

#[test]
fn diff_ignores_a_skipped_field() {
    let old = bar();
    let new = Bar {
        count: 2,
        label: "a",
        nick: None,
        resource: Resource(100),
    };
    assert_eq!(old.diff(&new), change_count(2));
}

#[test]
fn into_partial_leaves_a_skipped_field_unset() {
    let partial = change_count(2).into_partial();
    assert_eq!(partial.count(), Some(&2));
    assert!(!partial.has_resource());
}

#[test]
fn a_stale_patch_is_rejected_for_each_kind() {
    let mut value = bar();
    value.label = "x";
    let stale = bar().edit(|bound| bound.set_label("b"));
    let rejected = value.apply_patch(stale.clone()).unwrap_err();
    assert_eq!(
        rejected.error,
        ApplyFailure::Merge(Conflict(BarFields::Label))
    );
    assert_eq!(rejected.value, stale);
    assert_eq!(value.label, "x");

    let mut value = bar();
    value.nick = Some("x".into());
    assert_eq!(
        change_count(2).check(&value),
        Err(vec![Conflict(BarFields::Nick)])
    );
}

struct KeepCurrent;

impl<'a> BarMergeStrategy<'a> for KeepCurrent {
    type Error = Conflict<BarFields>;

    fn resolve_label(
        &self,
        _current: &&'a str,
        _old: &&'a str,
        _new: Option<&&'a str>,
    ) -> Result<Resolution<&'a str>, Self::Error> {
        Ok(Resolution::KeepCurrent)
    }
}

#[test]
fn a_merge_strategy_settles_a_conflict_on_a_reference() {
    let mut value = bar();
    value.label = "x";
    value
        .apply_patch_with_strategy(
            bar().edit(|bound| bound.set_label("b")),
            &KeepCurrent,
        )
        .unwrap();
    assert_eq!(value.label, "x");
}

#[test]
fn field_merge_keeps_a_change_to_a_field_the_patch_does_not_write() {
    let mut value = bar();
    value.nick = Some("x".into());
    value
        .apply_patch_with_strategy(change_count(2), &FieldMerge)
        .unwrap();
    assert_eq!(value.count, 2);
    assert_eq!(value.nick.as_deref(), Some("x"));
}

#[test]
fn a_whole_struct_failure_rolls_back_a_patch() {
    let mut value = bar();
    let patch = bar().edit(|bound| {
        bound.set_label("same");
        bound.set_nick(Some("same".into()));
    });
    let rejected = value
        .apply_patch_with(patch.clone(), &StrictMerge, &Rules)
        .unwrap_err();
    assert_eq!(rejected.value, patch);
    assert_eq!(value.label, "a");
    assert_eq!(value.nick, None);
}

#[test]
fn then_and_merge_work_across_field_kinds() {
    let first = bar().edit(|bound| bound.set_label("b"));
    let mut after_first = bar();
    after_first.label = "b";
    let next = after_first.edit(|bound| bound.set_label("c"));
    let combined = first.clone().then(next).unwrap();
    assert_eq!(combined.label(), Some(PatchValue::new(&"a", &"c")));

    let other = bar().edit(|bound| bound.set_nick(Some("n".into())));
    let merged = first.clone().merge(other).unwrap();
    assert!(merged.has_label() && merged.has_nick());

    let different = bar().edit(|bound| bound.set_label("z"));
    let rejected = first.merge(different).unwrap_err();
    assert_eq!(rejected.error, Conflict(BarFields::Label));
}

#[test]
fn invert_and_undo_work_on_a_struct_without_clone() {
    let mut value = bar();
    let patch = bar().edit(|bound| {
        bound.set_count(2);
        bound.set_nick(Some("n".into()));
    });
    value.apply_patch(patch.clone()).unwrap();
    let Ok(inverse) = patch.invert();
    value.apply_patch(inverse).unwrap();
    assert_eq!(value.count, 1);
    assert_eq!(value.nick, None);

    let undo = value.apply_patch_get_undo(change_count(5)).unwrap();
    assert_eq!(value.count, 5);
    value.apply_patch(undo).unwrap();
    assert_eq!(value.count, 1);
}
