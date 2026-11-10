//! Prototype: a value view and a slot view over every representation of a
//! struct, and a `Debug` hook written once on top of the slot view.
//!
//! The slot view can't be a lifetime GAT: a bound like
//! `for<'a> V::Slot<'a>: Debug` then requires `V: 'static`, so a value that
//! borrows can't be printed. Instead the slot type comes from a helper trait
//! with a lifetime parameter, whose default type `&'a Self` brings `Self: 'a`
//! into every `for<'a>` bound.
#![allow(dead_code)]

use std::fmt::{self, Debug};

// =========================================================== runtime crate
pub mod muck {
    use std::fmt;

    pub trait Shape: Sized {}

    pub trait FieldOf<S: Shape>: 'static {
        type Type;
        const NAME: &'static str;
        fn get(value: &S) -> &Self::Type;
    }

    /// The value view: what is the field?
    pub trait ReadField<S: Shape, F: FieldOf<S>> {
        fn read_field(&self) -> Option<&F::Type>;
    }

    /// The type of the slot of field `F` in `Self`, borrowed for `'a`.
    ///
    /// `Outlives` is never given: its default, `&'a Self`, makes every
    /// `for<'a>` bound on this trait range only over lifetimes `Self`
    /// outlives.
    pub trait SlotType<'a, S: Shape, F: FieldOf<S>, Outlives = &'a Self> {
        type Slot;
    }

    /// The slot view: what does this representation hold for the field?
    pub trait SlotView<S: Shape, F: FieldOf<S>>: for<'a> SlotType<'a, S, F> {
        fn slot_view(&self) -> <Self as SlotType<'_, S, F>>::Slot;
    }

    /// `Debug` for representation `V` of `Self`, written once.
    pub trait DebugShape<V: ?Sized> {
        fn fmt_shape(view: &V, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    }

    /// A patch's slot: what it does to the field.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum FieldChange<T> {
        Unchanged(T),
        Changed { old: T, new: T },
    }

    /// A staged value's slot.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum StagedSlot<T> {
        Unchanged(T),
        Staged(T),
    }
}

use muck::*;

// =========================================== what the base derive generates
pub struct Foo {
    pub bar: u32,
    pub zee: String,
    pub password: String,
}

pub mod foo_field {
    pub struct Bar;
    pub struct Zee;
    pub struct Password;
}

#[derive(Default)]
pub struct FooShape<B, Z, P> {
    pub bar: B,
    pub zee: Z,
    pub password: P,
}

impl Shape for Foo {}

macro_rules! field {
    ($marker:ident, $name:ident, $type:ty, $param:ident) => {
        impl FieldOf<Foo> for foo_field::$marker {
            type Type = $type;
            const NAME: &'static str = stringify!($name);
            fn get(value: &Foo) -> &$type {
                &value.$name
            }
        }

        impl ReadField<Foo, foo_field::$marker> for Foo {
            fn read_field(&self) -> Option<&$type> {
                Some(&self.$name)
            }
        }

        impl<'a> SlotType<'a, Foo, foo_field::$marker> for Foo {
            type Slot = &'a $type;
        }

        impl SlotView<Foo, foo_field::$marker> for Foo {
            fn slot_view(&self) -> &$type {
                &self.$name
            }
        }

        impl<'a, B, Z, P> SlotType<'a, Foo, foo_field::$marker> for FooShape<B, Z, P> {
            type Slot = &'a $param;
        }

        impl<B, Z, P> SlotView<Foo, foo_field::$marker> for FooShape<B, Z, P> {
            fn slot_view(&self) -> &$param {
                &self.$name
            }
        }
    };
}

field!(Bar, bar, u32, B);
field!(Zee, zee, String, Z);
field!(Password, password, String, P);

impl<B: ReadSlot<u32>, Z, P> ReadField<Foo, foo_field::Bar> for FooShape<B, Z, P> {
    fn read_field(&self) -> Option<&u32> {
        self.bar.read_slot()
    }
}
impl<B, Z: ReadSlot<String>, P> ReadField<Foo, foo_field::Zee> for FooShape<B, Z, P> {
    fn read_field(&self) -> Option<&String> {
        self.zee.read_slot()
    }
}
impl<B, Z, P: ReadSlot<String>> ReadField<Foo, foo_field::Password>
    for FooShape<B, Z, P>
{
    fn read_field(&self) -> Option<&String> {
        self.password.read_slot()
    }
}

/// The opt-in to the value view, as in `fieldmuck`.
pub trait ReadSlot<T> {
    fn read_slot(&self) -> Option<&T>;
}
impl<T> ReadSlot<T> for Option<T> {
    fn read_slot(&self) -> Option<&T> {
        self.as_ref()
    }
}

/// The value view of every field, by name.
pub trait FooView:
    ReadField<Foo, foo_field::Bar>
    + ReadField<Foo, foo_field::Zee>
    + ReadField<Foo, foo_field::Password>
{
    fn read_bar(&self) -> Option<&u32> {
        ReadField::<Foo, foo_field::Bar>::read_field(self)
    }
    fn read_zee(&self) -> Option<&String> {
        ReadField::<Foo, foo_field::Zee>::read_field(self)
    }
}
impl<T> FooView for T where
    T: ?Sized
        + ReadField<Foo, foo_field::Bar>
        + ReadField<Foo, foo_field::Zee>
        + ReadField<Foo, foo_field::Password>
{
}

/// The slot view of every field, by name.
pub trait FooSlots:
    SlotView<Foo, foo_field::Bar>
    + SlotView<Foo, foo_field::Zee>
    + SlotView<Foo, foo_field::Password>
{
    fn bar_slot(&self) -> <Self as SlotType<'_, Foo, foo_field::Bar>>::Slot {
        SlotView::<Foo, foo_field::Bar>::slot_view(self)
    }
    fn zee_slot(&self) -> <Self as SlotType<'_, Foo, foo_field::Zee>>::Slot {
        SlotView::<Foo, foo_field::Zee>::slot_view(self)
    }
    fn password_slot(
        &self,
    ) -> <Self as SlotType<'_, Foo, foo_field::Password>>::Slot {
        SlotView::<Foo, foo_field::Password>::slot_view(self)
    }
}
impl<T> FooSlots for T where
    T: ?Sized
        + SlotView<Foo, foo_field::Bar>
        + SlotView<Foo, foo_field::Zee>
        + SlotView<Foo, foo_field::Password>
{
}

// The derive's `Debug` impls only defer to the hook.
impl Debug for Foo
where
    Foo: DebugShape<Foo>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Foo as DebugShape<Foo>>::fmt_shape(self, f)
    }
}

impl<B, Z, P> Debug for FooShape<B, Z, P>
where
    Foo: DebugShape<Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Foo as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ==================================== the user's own hook: hides `password`
impl<V: FooSlots + ?Sized> DebugShape<V> for Foo
where
    for<'a> <V as SlotType<'a, Foo, foo_field::Bar>>::Slot: Debug,
    for<'a> <V as SlotType<'a, Foo, foo_field::Zee>>::Slot: Debug,
{
    fn fmt_shape(view: &V, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Foo")
            .field("bar", &view.bar_slot())
            .field("zee", &view.zee_slot())
            .field("password", &"***")
            .finish()
    }
}

// ============================================ generic library types (partly)
pub type FooPartial = FooShape<Option<u32>, Option<String>, Option<String>>;

/// A patch: the value it was made against, and what it writes.
pub struct Patch {
    pub old: Foo,
    pub new: FooPartial,
}

/// Writes staged over a borrowed value.
pub struct Staged<'s> {
    pub target: &'s Foo,
    pub buffer: FooPartial,
}

macro_rules! library_field {
    ($marker:ident, $name:ident, $type:ty) => {
        impl ReadField<Foo, foo_field::$marker> for Patch {
            fn read_field(&self) -> Option<&$type> {
                self.new.$name.as_ref().or(Some(&self.old.$name))
            }
        }

        impl<'a> SlotType<'a, Foo, foo_field::$marker> for Patch {
            type Slot = FieldChange<&'a $type>;
        }

        impl SlotView<Foo, foo_field::$marker> for Patch {
            fn slot_view(&self) -> FieldChange<&$type> {
                match &self.new.$name {
                    Some(new) => {
                        FieldChange::Changed {
                            old: &self.old.$name,
                            new,
                        }
                    },
                    None => FieldChange::Unchanged(&self.old.$name),
                }
            }
        }

        impl ReadField<Foo, foo_field::$marker> for Staged<'_> {
            fn read_field(&self) -> Option<&$type> {
                self.buffer.$name.as_ref().or(Some(&self.target.$name))
            }
        }

        impl<'a, 's> SlotType<'a, Foo, foo_field::$marker> for Staged<'s> {
            type Slot = StagedSlot<&'a $type>;
        }

        impl SlotView<Foo, foo_field::$marker> for Staged<'_> {
            fn slot_view(&self) -> StagedSlot<&$type> {
                match &self.buffer.$name {
                    Some(new) => StagedSlot::Staged(new),
                    None => StagedSlot::Unchanged(&self.target.$name),
                }
            }
        }
    };
}

library_field!(Bar, bar, u32);
library_field!(Zee, zee, String);
library_field!(Password, password, String);

impl Debug for Patch
where
    Foo: DebugShape<Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Foo as DebugShape<Self>>::fmt_shape(self, f)
    }
}

impl<'s> Debug for Staged<'s>
where
    Foo: DebugShape<Staged<'s>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Foo as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ================================ algorithms written once, on the value view
pub fn average_bar<T: FooView + ?Sized>(items: &[&T]) -> Option<f64> {
    let values: Vec<f64> = items
        .iter()
        .filter_map(|item| item.read_bar())
        .map(|bar| f64::from(*bar))
        .collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn foo(bar: u32) -> Foo {
        Foo {
            bar,
            zee: "a".into(),
            password: "secret".into(),
        }
    }

    #[test]
    fn each_representation_prints_its_own_slots_and_hides_the_password() {
        assert_eq!(
            format!("{:?}", foo(1)),
            r#"Foo { bar: 1, zee: "a", password: "***" }"#
        );

        let partial = FooPartial {
            bar: Some(2),
            ..Default::default()
        };
        assert_eq!(
            format!("{partial:?}"),
            r#"Foo { bar: Some(2), zee: None, password: "***" }"#
        );

        let patch = Patch {
            old: foo(1),
            new: partial,
        };
        assert_eq!(
            format!("{patch:?}"),
            r#"Foo { bar: Changed { old: 1, new: 2 }, zee: Unchanged("a"), password: "***" }"#
        );

        // A value that borrows a local, which a lifetime GAT couldn't print.
        let value = foo(1);
        let staged = Staged {
            target: &value,
            buffer: FooPartial {
                zee: Some("b".into()),
                ..Default::default()
            },
        };
        assert_eq!(
            format!("{staged:?}"),
            r#"Foo { bar: Unchanged(1), zee: Staged("b"), password: "***" }"#
        );
    }

    #[test]
    fn a_slot_the_hook_does_not_print_needs_no_debug() {
        struct NoDebug;
        let shape = FooShape {
            bar: 1_u8,
            zee: "z",
            password: NoDebug,
        };
        assert_eq!(
            format!("{shape:?}"),
            r#"Foo { bar: 1, zee: "z", password: "***" }"#
        );
    }

    #[test]
    fn a_value_algorithm_works_on_every_representation() {
        assert_eq!(average_bar(&[&foo(1), &foo(3)]), Some(2.0));

        let partials = [
            FooPartial {
                bar: Some(4),
                ..Default::default()
            },
            FooPartial::default(),
        ];
        assert_eq!(average_bar(&[&partials[0], &partials[1]]), Some(4.0));

        let patch = Patch {
            old: foo(1),
            new: FooPartial {
                bar: Some(5),
                ..Default::default()
            },
        };
        assert_eq!(average_bar(&[&patch]), Some(5.0));

        let value = foo(7);
        let staged = Staged {
            target: &value,
            buffer: FooPartial::default(),
        };

        // Mixed representations, through a trait object.
        let mixed: [&dyn FooView; 4] = [&foo(1), &partials[0], &patch, &staged];
        assert_eq!(average_bar(&mixed), Some(4.25));
    }
}
