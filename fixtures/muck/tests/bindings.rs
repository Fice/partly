//! Staging writes over a borrowed value, collecting what an undo needs, and
//! typestate through replacing one slot.

use fieldmuck::{
    Previous, ReadField, Recorder, ReplaceSlot, Set, SetField, Staged, Tracked,
    Unset,
};
use fieldmuck_dirty::Dirty;
use fieldmuck_fixtures::foo::{
    Foo, FooFields, FooReaders, FooSetters, FooShape, foo_field,
};

fn foo() -> Foo {
    Foo {
        bar:  5,
        zee:  "a".into(),
        nick: None,
    }
}

/// Code that only knows it can read and write `bar`.
fn bump<T>(target: &mut T)
where
    T: ReadField<Foo, foo_field::Bar> + SetField<Foo, foo_field::Bar>,
{
    let bar = target.read_bar().copied().unwrap_or(0);
    target.set_bar(bar + 1);
}

#[test]
fn staging_leaves_the_value_and_reads_the_staged_write() {
    let value = foo();
    let mut staged = Staged::new(&value, Recorder::<Foo>::new());
    assert_eq!(staged.read_bar(), Some(&5));

    bump(&mut staged);
    bump(&mut staged);
    assert_eq!(staged.read_bar(), Some(&7));
    assert_eq!(staged.read_zee().map(String::as_str), Some("a"));

    let (target, buffer) = staged.into_parts();
    assert_eq!(target.bar, 5);
    assert_eq!(buffer.get::<foo_field::Bar>(), Some(&7));
    assert_eq!(buffer.get::<foo_field::Zee>(), None);
}

#[test]
fn any_recorder_can_be_the_buffer() {
    let value = foo();
    let mut staged = Staged::new(&value, (Dirty::<Foo>::new(), ()));
    staged.buffer_mut().0.set_zee("b".into());
    assert_eq!(staged.buffer().0.fields(), [FooFields::Zee]);
}

#[test]
fn previous_values_are_what_an_undo_writes_back() {
    let mut value = foo();
    let mut tracked = Tracked::new(&mut value, Previous::<Foo>::new());
    bump(&mut tracked);
    bump(&mut tracked);
    tracked.set_zee("b".into());
    let previous = tracked.into_observer();

    // The first old value of each written field, nothing else.
    assert_eq!(previous.get::<foo_field::Bar>(), Some(&5));
    assert_eq!(
        previous.get::<foo_field::Zee>().map(String::as_str),
        Some("a")
    );
    assert_eq!(previous.get::<foo_field::Nick>(), None);
    assert_eq!(value.bar, 7);
}

#[test]
fn a_partial_companion_takes_writes_like_the_struct() {
    let mut partial: FooShape<
        Option<u32>,
        Option<String>,
        Option<Option<String>>,
    > = FooShape::default();
    bump(&mut partial);
    assert_eq!(partial.bar, Some(1));
}

#[test]
fn replacing_a_slot_changes_its_type() {
    let empty: FooShape<Unset, Option<String>, Unset> =
        FooShape::from_slots(Unset, None, Unset);
    let (old, with_bar) =
        ReplaceSlot::<foo_field::Bar, Set<u32>>::replace_slot(empty, Set(3));
    assert_eq!(old, Unset);

    // The type now says `bar` is set.
    let with_bar: FooShape<Set<u32>, Option<String>, Unset> = with_bar;
    assert_eq!(with_bar.read_bar(), Some(&3));
    assert_eq!(with_bar.read_nick(), None);
}
