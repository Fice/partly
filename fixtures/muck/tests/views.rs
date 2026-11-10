//! The value view and the slot view: algorithms written once for every
//! representation, and `Debug` written once in a hook.

use fieldmuck::{Optional, Recorder, SetField, Staged, Tracked, With};
use fieldmuck_fixtures::account::{Account, AccountShape, account_field};
use fieldmuck_fixtures::foo::foo_prelude::*;
use fieldmuck_fixtures::foo::{Foo, FooShape};

fn foo(bar: u32) -> Foo {
    Foo {
        bar,
        zee: "a".into(),
        nick: None,
    }
}

fn account() -> Account {
    Account {
        name: "ann".into(),
        password: "secret".into(),
    }
}

/// Written once, against the value view.
fn average_bar<T: FooView + ?Sized>(items: &[&T]) -> Option<f64> {
    let values: Vec<f64> = items
        .iter()
        .filter_map(|item| item.read_bar())
        .map(|bar| f64::from(*bar))
        .collect();
    (!values.is_empty())
        .then(|| values.iter().sum::<f64>() / values.len() as f64)
}

#[test]
fn a_value_algorithm_works_on_every_representation() {
    assert_eq!(average_bar(&[&foo(1), &foo(3)]), Some(2.0));

    let partial: With<Foo, Optional> =
        FooShape::from_slots(Some(6), None, None);
    let empty: With<Foo, Optional> = FooShape::default();
    assert_eq!(average_bar(&[&partial, &empty]), Some(6.0));

    let value = foo(2);
    let mut staged = Staged::new(&value, Recorder::<Foo>::new());
    assert_eq!(average_bar(&[&staged]), Some(2.0));
    staged.set_bar(4);

    let mut tracked_value = foo(10);
    let tracked = Tracked::new(&mut tracked_value, ());

    // Different representations, through a trait object.
    let mixed: [&dyn FooView; 4] = [&foo(1), &partial, &staged, &tracked];
    assert_eq!(average_bar(&mixed), Some(5.25));
}

#[test]
fn the_default_hook_prints_each_representations_own_slots() {
    let partial: With<Foo, Optional> =
        FooShape::from_slots(Some(1), None, Some(None));
    assert_eq!(
        format!("{partial:?}"),
        "FooPartial { bar: Some(1), zee: None, nick: Some(None) }"
    );

    // Built by hand, with no kind.
    let shape: FooShape<u8, &str, ()> = FooShape::from_slots(1, "z", ());
    assert_eq!(
        format!("{shape:?}"),
        r#"FooShape { bar: 1, zee: "z", nick: () }"#
    );

    let value = foo(1);
    let mut staged = Staged::new(&value, Recorder::<Foo>::new());
    staged.set_zee("b".into());
    assert_eq!(
        format!("{staged:?}"),
        r#"FooStaged { bar: Unchanged(1), zee: Staged("b"), nick: Unchanged(None) }"#
    );
}

#[test]
fn a_custom_hook_hides_a_field_in_every_representation() {
    assert_eq!(
        format!("{:?}", account()),
        r#"Account { name: "ann", password: "***" }"#
    );

    let partial: With<Account, Optional> =
        AccountShape::from_slots(Some("bob".to_owned()), None);
    assert_eq!(
        format!("{partial:?}"),
        r#"AccountPartial { name: Some("bob"), password: "***" }"#
    );

    let value = account();
    let mut staged = Staged::new(&value, Recorder::<Account>::new());
    SetField::<Account, account_field::Password>::set_field(
        &mut staged,
        "new secret".into(),
    );
    assert_eq!(
        format!("{staged:?}"),
        r#"AccountStaged { name: Unchanged("ann"), password: "***" }"#
    );
}

#[test]
fn a_slot_the_hook_does_not_print_needs_no_debug() {
    struct NoDebug;
    let shape: AccountShape<_, _> = AccountShape::from_slots("carl", NoDebug);
    assert_eq!(
        format!("{shape:?}"),
        r#"AccountShape { name: "carl", password: "***" }"#
    );
}
