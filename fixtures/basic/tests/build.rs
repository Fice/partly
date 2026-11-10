//! Building, with default and custom strategies.

use std::cell::Cell;

use partly::prelude::*;
use partly::{AcceptAll, BuildFailure, DefaultBuildStrategy, MissingField};
use partly_fixtures_basic::config::{Config, ConfigFields, ConfigPartial};
use partly_fixtures_basic::foo::{
    Foo, FooBuildStrategy, FooFields, FooPartial, FooValidator,
};

fn foo() -> Foo {
    Foo {
        bar: 1,
        zee: "a".into(),
    }
}

#[test]
fn a_complete_partial_builds() {
    let partial = FooPartial::new().with_bar(1).with_zee("a".into());
    assert!(partial.can_build());
    assert_eq!(partial.build().unwrap(), foo());
}

#[test]
fn a_missing_field_names_the_field_with_the_enum() {
    let partial = FooPartial::new().with_bar(1);
    assert!(!partial.can_build());

    let rejected = partial.build().unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingField(FooFields::Zee))
    );
    assert_eq!(rejected.value.bar(), Some(&1));
    assert_eq!(rejected.error.to_string(), "missing field `zee`");
}

#[test]
fn the_first_missing_field_is_reported_and_nothing_is_lost() {
    let rejected = FooPartial::new().with_zee("a".into()).build().unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingField(FooFields::Bar))
    );
    assert_eq!(rejected.value.zee().map(String::as_str), Some("a"));
}

/// Rounds `bar` down to a multiple of ten, and defaults a missing one to ten.
struct Rounding;

impl FooBuildStrategy for Rounding {
    type Error = MissingField<FooFields>;

    fn resolve_bar(&self, value: Option<u32>) -> Result<u32, Self::Error> {
        Ok(value.unwrap_or(10) / 10 * 10)
    }
}

#[test]
fn a_strategy_can_change_a_value_that_was_set() {
    let partial = FooPartial::new().with_bar(17).with_zee("a".into());
    let built = partial.build_with_strategy(&Rounding).unwrap();
    assert_eq!(built.bar, 10);
}

#[test]
fn a_consuming_build_returns_resolved_values_on_failure() {
    let partial = FooPartial::new().with_bar(17);
    let rejected = partial.build_with_strategy(&Rounding).unwrap_err();
    assert_eq!(rejected.value.bar(), Some(&10));
}

#[test]
fn a_non_consuming_build_leaves_the_partial_untouched() {
    let partial = FooPartial::new().with_bar(17);

    let error = partial
        .build_cloned_with(&Rounding, &AcceptAll)
        .unwrap_err();
    assert_eq!(error, BuildFailure::Strategy(MissingField(FooFields::Zee)));
    assert_eq!(partial.bar(), Some(&17));

    let error = partial.build_cloned().unwrap_err();
    assert_eq!(error, BuildFailure::Strategy(MissingField(FooFields::Zee)));
    assert_eq!(partial.bar(), Some(&17));

    let complete = partial.with_zee("a".into());
    assert_eq!(complete.build_cloned().unwrap().bar, 17);
    assert!(complete.is_complete());
}

#[test]
fn build_or_fills_gaps_field_by_field() {
    let partial = FooPartial::new().with_bar(9);
    let built = partial.build_or(foo()).unwrap();
    assert_eq!(built.bar, 9);
    assert_eq!(built.zee, "a");
}

#[test]
fn build_or_else_only_runs_the_fallback_when_a_field_is_missing() {
    let calls = Cell::new(0);
    let fallback = || {
        calls.set(calls.get() + 1);
        foo()
    };

    let complete = FooPartial::new().with_bar(9).with_zee("z".into());
    assert_eq!(complete.build_or_else(fallback).unwrap().zee, "z");
    assert_eq!(calls.get(), 0);

    let gap = FooPartial::new().with_bar(9);
    assert_eq!(gap.build_or_else(fallback).unwrap().zee, "a");
    assert_eq!(calls.get(), 1);
}

#[test]
fn the_default_strategy_can_be_passed_explicitly() {
    let partial = FooPartial::new().with_bar(1).with_zee("a".into());
    let built = partial
        .build_with(&DefaultBuildStrategy, &AcceptAll)
        .unwrap();
    assert_eq!(built, foo());
}

struct NoEmptyZee;

impl FooValidator for NoEmptyZee {
    type Error = &'static str;

    fn validate_zee(&self, value: &String) -> Result<(), Self::Error> {
        if value.is_empty() {
            Err("empty")
        } else {
            Ok(())
        }
    }
}

#[test]
fn building_runs_the_validator_after_the_strategy() {
    let partial = FooPartial::new().with_bar(17).with_zee(String::new());
    let rejected = partial.build_with(&Rounding, &NoEmptyZee).unwrap_err();
    assert!(matches!(rejected.error, BuildFailure::Validation(_)));
    assert_eq!(rejected.value.bar(), Some(&10));
    assert_eq!(rejected.value.zee().map(String::as_str), Some(""));
}

#[test]
fn skipped_fields_are_derived_from_the_resolved_fields() {
    let mut partial = ConfigPartial::new();
    partial.set_name("web".into());
    partial.set_started_at(7_300);

    let config: Config = partial.build().unwrap();
    // `started_at` was rounded before `slug` saw it.
    assert_eq!(config.started_at, 7_200);
    assert_eq!(config.slug, "web-7200");
    // Plain `skip` falls back to `Default`.
    assert_eq!(config.retries, 0);
}

#[test]
fn skipped_fields_do_not_count_as_missing() {
    let mut partial = ConfigPartial::new();
    partial.set_name("web".into());
    partial.set_nick(None);
    partial.set_started_at(3_600);
    assert!(partial.is_complete());
}

#[test]
fn a_strategy_can_supply_a_missing_option_field() {
    let mut partial = ConfigPartial::new();
    partial.set_name("web".into());
    partial.set_started_at(3_600);
    assert!(!partial.can_build());
    assert_eq!(partial.build().unwrap().nick, None);
}

#[test]
fn an_option_field_separates_unset_from_set_to_none() {
    let mut partial = ConfigPartial::new();
    assert_eq!(partial.nick(), None);
    partial.set_nick(None);
    assert_eq!(partial.nick(), Some(&None));
    partial.clear_nick();
    assert_eq!(partial.nick(), None);
}

#[test]
fn the_default_strategy_and_validator_come_from_the_attribute() {
    let mut partial = ConfigPartial::new();
    partial.set_name(String::new());
    partial.set_started_at(3_600);
    let rejected = partial.build().unwrap_err();
    assert!(matches!(rejected.error, BuildFailure::Validation(_)));
}

#[test]
fn missing_field_errors_work_with_custom_strategies() {
    let mut partial = ConfigPartial::new();
    partial.set_name("web".into());
    let rejected = partial.build().unwrap_err();
    assert_eq!(
        rejected.error,
        BuildFailure::Strategy(MissingField(ConfigFields::StartedAt))
    );
}
