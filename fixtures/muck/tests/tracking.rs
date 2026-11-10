//! Writing fields without knowing what is written to: a value, a tracked
//! value with chained observers, or a recorder with no value at all.

use fieldmuck::{Recorder, SetField, Tracked};
use fieldmuck_dirty::Dirty;
use fieldmuck_fixtures::foo::{
    Foo, FooFields, FooReaders, FooSetters, foo_field,
};
use fieldmuck_fixtures::labeled::{Labeled, LabeledSetters, labeled_field};

fn foo() -> Foo {
    Foo {
        bar:  5,
        zee:  "a".into(),
        nick: None,
    }
}

/// Code that only knows it can write `zee` and `bar`.
fn rename<T>(target: &mut T)
where
    T: SetField<Foo, foo_field::Zee> + SetField<Foo, foo_field::Bar> + ?Sized,
{
    target.set_zee("renamed".into());
    target.set_bar(9);
}

#[test]
fn writes_go_to_the_value() {
    let mut value = foo();
    rename(&mut value);
    assert_eq!(value.zee, "renamed");
    assert_eq!(value.bar, 9);
}

#[test]
fn a_dirty_set_records_which_fields_were_written() {
    let mut dirty = Dirty::<Foo>::new();
    assert!(dirty.is_clean());
    rename(&mut dirty);
    assert!(dirty.is_dirty::<foo_field::Zee>());
    assert!(!dirty.is_dirty::<foo_field::Nick>());
    assert_eq!(dirty.fields(), [FooFields::Bar, FooFields::Zee]);
}

#[test]
fn a_recorder_without_a_value_keeps_what_was_written() {
    let mut recorder = Recorder::<Foo>::new();
    rename(&mut recorder);
    assert_eq!(
        recorder.get::<foo_field::Zee>().map(String::as_str),
        Some("renamed")
    );
    assert_eq!(recorder.get::<foo_field::Nick>(), None);

    // The recorded values are a partial, which reads like the struct.
    let values = recorder.into_values();
    assert_eq!(values.read_bar(), Some(&9));
    assert_eq!(values.read_nick(), None);
}

#[test]
fn observers_chain_on_a_tracked_value() {
    let mut value = foo();
    let mut tracked =
        Tracked::new(&mut value, (Dirty::<Foo>::new(), Recorder::<Foo>::new()));
    rename(&mut tracked);
    assert_eq!(tracked.value().zee, "renamed");

    let (dirty, recorder) = tracked.into_observer();
    assert_eq!(dirty.fields(), [FooFields::Bar, FooFields::Zee]);
    assert_eq!(recorder.get::<foo_field::Bar>(), Some(&9));
    assert_eq!(value.bar, 9);
}

#[test]
fn a_tracked_value_still_reads_like_the_struct() {
    let mut value = foo();
    let mut tracked = Tracked::new(&mut value, ());
    tracked.set_nick(Some("n".into()));
    assert_eq!(tracked.read_nick(), Some(&Some("n".to_owned())));
    assert_eq!(tracked.read_bar(), Some(&5));
}

#[test]
fn tracking_works_on_a_generic_struct() {
    let mut value = Labeled {
        label: "a",
        value: 1_u8,
    };
    let mut tracked = Tracked::new(&mut value, Dirty::<Labeled<'_, u8>>::new());
    tracked.set_value(2);
    let dirty = tracked.into_observer();
    assert!(dirty.is_dirty::<labeled_field::Value>());
    assert!(!dirty.is_dirty::<labeled_field::Label>());
    assert_eq!(value.value, 2);
}
