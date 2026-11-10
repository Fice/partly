//! Validation, which runs whenever a value changes and not only when building.

use partly::prelude::*;
use partly::{FieldError, ValidationErrors};
use partly_fixtures_basic::config::{Config, ConfigFields, ConfigPartial};
use partly_fixtures_basic::foo::{Foo, FooFields, FooPartial, FooValidator};

fn foo() -> Foo {
    Foo {
        bar: 1,
        zee: "a".into(),
    }
}

fn config() -> Config {
    Config {
        name: "web".into(),
        nick: None,
        started_at: 3_600,
        slug: "web-3600".into(),
        retries: 0,
    }
}

struct Rules;

impl FooValidator for Rules {
    type Error = &'static str;

    fn validate_bar(&self, value: &u32) -> Result<(), Self::Error> {
        if *value == 0 { Err("zero") } else { Ok(()) }
    }

    fn validate_zee(&self, value: &String) -> Result<(), Self::Error> {
        if value.is_empty() {
            Err("empty")
        } else {
            Ok(())
        }
    }

    fn validate(&self, value: &Foo) -> Result<(), Self::Error> {
        if value.bar > 100 && value.zee.len() < 2 {
            Err("large bar needs a long zee")
        } else {
            Ok(())
        }
    }
}

#[test]
fn every_field_error_is_collected() {
    let invalid = Foo {
        bar: 0,
        zee: String::new(),
    };
    assert_eq!(
        invalid.validate_with(&Rules),
        Err(ValidationErrors::Fields(vec![
            FieldError {
                field: FooFields::Bar,
                error: "zero",
            },
            FieldError {
                field: FooFields::Zee,
                error: "empty",
            },
        ]))
    );
}

#[test]
fn the_whole_struct_check_only_runs_once_fields_pass() {
    let cross_field = Foo {
        bar: 200,
        zee: "a".into(),
    };
    assert_eq!(
        cross_field.validate_with(&Rules),
        Err(ValidationErrors::Struct("large bar needs a long zee"))
    );

    let both = Foo {
        bar: 200,
        zee: String::new(),
    };
    assert!(matches!(
        both.validate_with(&Rules),
        Err(ValidationErrors::Fields(_))
    ));
}

#[test]
fn a_partial_can_be_validated_on_its_own_and_only_set_fields_are_checked() {
    let valid = FooPartial::new().with_bar(1);
    assert_eq!(Foo::validate_partial_with(&valid, &Rules), Ok(()));

    let invalid = FooPartial::new().with_bar(0);
    assert_eq!(
        Foo::validate_partial_with(&invalid, &Rules),
        Err(ValidationErrors::Fields(vec![FieldError {
            field: FooFields::Bar,
            error: "zero",
        }]))
    );
}

#[test]
fn apply_partial_writes_only_the_fields_that_are_set() {
    let mut value = foo();
    value.apply_partial(FooPartial::new().with_bar(9)).unwrap();
    assert_eq!(value.bar, 9);
    assert_eq!(value.zee, "a");
}

#[test]
fn apply_partial_accepts_anything_that_converts_into_a_partial() {
    let mut value = foo();
    let other = Foo {
        bar: 2,
        zee: "b".into(),
    };
    value.apply_partial(other.clone()).unwrap();
    assert_eq!(value, other);
}

#[test]
fn a_field_failure_rejects_the_whole_partial() {
    let mut value = foo();
    let input = FooPartial::new().with_bar(9).with_zee(String::new());
    let rejected = value.apply_partial_with(input.clone(), &Rules).unwrap_err();
    assert_eq!(value, foo());
    assert_eq!(rejected.value, input);
}

#[test]
fn a_whole_struct_failure_rolls_back() {
    let mut value = foo();
    let input = FooPartial::new().with_bar(200);
    let rejected = value.apply_partial_with(input.clone(), &Rules).unwrap_err();
    assert_eq!(value, foo());
    assert_eq!(rejected.value, input);
    assert_eq!(
        rejected.error,
        ValidationErrors::Struct("large bar needs a long zee")
    );
}

#[test]
fn apply_partial_uses_the_default_validator_from_the_attribute() {
    let mut value = config();
    let mut input = ConfigPartial::new();
    input.set_name(String::new());
    let rejected = value.apply_partial(input).unwrap_err();
    assert_eq!(value, config());
    assert_eq!(
        rejected.error,
        ValidationErrors::Fields(vec![FieldError {
            field: ConfigFields::Name,
            error: "must not be empty",
        }])
    );
}

#[test]
fn apply_partial_checks_the_value_as_a_whole_after_the_change() {
    let mut value = config();
    let mut input = ConfigPartial::new();
    input.set_nick(Some("web".into()));
    let rejected = value.apply_partial(input).unwrap_err();
    assert_eq!(value, config());
    assert_eq!(
        rejected.error,
        ValidationErrors::Struct("nick must differ from name")
    );
}

#[test]
fn apply_partial_does_not_run_the_build_strategy() {
    let mut value = config();
    let mut input = ConfigPartial::new();
    input.set_started_at(7_300);
    value.apply_partial(input).unwrap();
    // `ConfigDefaults` would have rounded this to 7200.
    assert_eq!(value.started_at, 7_300);
}

#[test]
fn validate_uses_the_default_validator() {
    assert_eq!(config().validate(), Ok(()));
    let mut invalid = config();
    invalid.name = String::new();
    assert!(invalid.validate().is_err());
}

#[test]
fn validation_errors_display_the_field_names() {
    let errors = ValidationErrors::Fields(vec![
        FieldError {
            field: FooFields::Bar,
            error: "zero",
        },
        FieldError {
            field: FooFields::Zee,
            error: "empty",
        },
    ]);
    assert_eq!(errors.to_string(), "`bar`: zero; `zee`: empty");
}

#[test]
fn rejected_is_a_real_error_type() {
    let mut value = foo();
    let rejected = value
        .apply_partial_with(FooPartial::new().with_bar(0), &Rules)
        .unwrap_err();
    let error: Box<dyn std::error::Error + Send + Sync + 'static> =
        Box::new(rejected);
    assert_eq!(error.to_string(), "`bar`: zero");
}
