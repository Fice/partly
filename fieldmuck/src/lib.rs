//! Build companion types of a struct, field by field.
//!
//! `#[derive(Muck)]` describes a struct once: its fields, one type per field,
//! and one generic companion struct with a type parameter per field. Every
//! companion type, such as a partial, a filter or a set of dirty flags, is an
//! instance of that companion struct. Libraries define companions without a
//! macro, by implementing [`SlotFor`] for a *kind*, which picks the type of
//! each field's slot.
//!
//! Work in progress: the derive doesn't exist yet. See the `fixtures/muck`
//! crate for what it will generate.
#![warn(missing_docs)]

use std::fmt::Debug;

/// The traits needed to use companions with method syntax.
pub mod prelude {
    pub use crate::{
        Apply, Make, MakeFrom, ReadField, SetField, Shape, ShapeWith, Visit,
        VisitWithValue,
    };
}

// ------------------------------------------------------------------- shapes

/// An enum naming the fields of a struct.
pub trait Field: Copy + Eq + Debug + 'static {
    /// Every field, in declaration order.
    const ALL: &'static [Self];

    /// The name of the field as written in the struct.
    fn name(self) -> &'static str;
}

/// A struct described field by field. Implemented by `#[derive(Muck)]`.
pub trait Shape: Sized {
    /// The enum naming the fields.
    type Fields: Field;
    /// The name of the struct, as written. Representations print with it:
    /// `Foo`'s partial as `FooPartial`.
    const NAME: &'static str;
}

/// A kind's name, which the companions of that kind print with: the
/// companion of `Foo` of a kind named `Partial` prints as `FooPartial`.
///
/// The companion struct carries its kind as its last type parameter, so it
/// knows what it is.
pub trait Kind {
    /// The kind's name, appended to the struct's.
    const NAME: &'static str;
}

/// No kind: a companion built by hand, which prints as `FooShape`.
impl Kind for () {
    const NAME: &'static str = "Shape";
}

/// Kind `Self` accepts `Slot` as the slot of a field of type `T`.
///
/// Every way to build a companion of kind `K` requires this for each slot, so
/// a companion's kind can't disagree with its slots: a partial's slot for a
/// `u32` field is an `Option<u32>`, `Set<u32>` or `Unset`, never a filter.
/// A kind accepts at least its own [`SlotFor::Slot`], and may accept more,
/// like the typestate slots of a partial.
pub trait Accepts<T, Slot> {}

/// A companion built by hand accepts any slot.
impl<T, Slot> Accepts<T, Slot> for () {}

/// What a representation of `S` is, for printing: `""` for `S` itself,
/// `"Partial"` for its partial, `"Patch"` for a patch.
pub trait Representation<S: Shape> {
    /// Appended to `S`'s name.
    const KIND: &'static str;
}

/// The name a representation `V` of `S` prints with, like `FooPartial`.
#[must_use]
pub fn name_of<S: Shape, V: Representation<S> + ?Sized>() -> String {
    [S::NAME, V::KIND].concat()
}

/// One field of `S`, as a type. `#[derive(Muck)]` makes one per field.
///
/// Per-field behaviour is written as a trait implemented for every
/// `F: FieldOf<S>`, so a library can require bounds on [`FieldOf::Type`]
/// field by field.
pub trait FieldOf<S: Shape>: 'static {
    /// The type of the field.
    type Type;
    /// The field, as a value of the field enum.
    const FIELD: S::Fields;

    /// Borrows the field.
    fn get(value: &S) -> &Self::Type;

    /// Borrows the field mutably.
    fn get_mut(value: &mut S) -> &mut Self::Type;
}

/// A kind of companion: it picks the type of each field's slot.
///
/// The kind sees the field `F`, so it can decide by the field's type, through
/// a trait on [`FieldOf::Type`], or by the field itself, through a trait a
/// thin derive implements for `F`.
pub trait SlotFor<S: Shape, F: FieldOf<S>> {
    /// The type of the field's slot.
    type Slot;
}

/// The companion of `S` of kind `K`. Implemented by `#[derive(Muck)]` for
/// every kind that has a slot for every field.
pub trait ShapeWith<K>: Shape {
    /// The companion struct, with each field's slot.
    type Out;
}

/// The companion of `S` of kind `K`.
pub type With<S, K> = <S as ShapeWith<K>>::Out;

/// Access to the slot of field `Self` in a companion of kind `K`.
pub trait SlotOf<S, K>: FieldOf<S>
where
    S: ShapeWith<K>,
    K: SlotFor<S, Self>,
    Self: Sized,
{
    /// Borrows the slot.
    fn slot(with: &With<S, K>) -> &K::Slot;

    /// Borrows the slot mutably.
    fn slot_mut(with: &mut With<S, K>) -> &mut K::Slot;
}

// -------------------------------------------------------------------- kinds

/// The kind whose slot is the field itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Same;

impl<S: Shape, F: FieldOf<S>> SlotFor<S, F> for Same {
    type Slot = F::Type;
}

/// The kind whose slot may hold the field's value: the plain partial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Optional;

impl<S: Shape, F: FieldOf<S>> SlotFor<S, F> for Optional {
    type Slot = Option<F::Type>;
}

/// The companion of this kind holds exactly the struct's values.
impl Kind for Same {
    const NAME: &'static str = "Values";
}

impl<T> Accepts<T, T> for Same {}

impl Kind for Optional {
    const NAME: &'static str = "Partial";
}

/// A partial's slot may hold the value, or say at compile time that it is
/// set or unset.
impl<T> Accepts<T, Option<T>> for Optional {}
impl<T> Accepts<T, Set<T>> for Optional {}
impl<T> Accepts<T, Unset> for Optional {}

// ---------------------------------------------------------------- typestate
//
// A companion has a type parameter per field, so each slot's type can change
// on its own. `Unset` and `Set` make "this field has a value" a compile-time
// fact, next to `Option` for a runtime one.

/// A slot known to hold no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Unset;

/// A slot known to hold a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Set<T>(pub T);

/// Replaces the slot of field `F` with one of type `New`, which can change the
/// companion's type. Implemented by `#[derive(Muck)]` for every field.
pub trait ReplaceSlot<F, New> {
    /// The slot that was replaced.
    type Old;
    /// The companion with the new slot.
    type Out;

    /// Puts `slot` in place, returning the old slot and the new companion.
    fn replace_slot(self, slot: New) -> (Self::Old, Self::Out);
}

// --------------------------------------------------------------- traversals
//
// `#[derive(Muck)]` implements each shape-level trait for every kind and
// visitor that handle all fields. The visitor implements the field-level trait
// once, generically over the field.

/// Visits one slot.
pub trait VisitField<S: Shape, F: FieldOf<S>, K: SlotFor<S, F>> {
    /// Called once for field `F`.
    fn visit(&mut self, slot: &K::Slot);
}

/// Visits every slot of a companion, in declaration order.
pub trait Visit<K, V>: ShapeWith<K> {
    /// Calls the visitor once per field.
    fn visit(with: &Self::Out, visitor: &mut V);
}

/// Visits one slot, together with the field's value.
pub trait VisitFieldWithValue<S: Shape, F: FieldOf<S>, K: SlotFor<S, F>> {
    /// Called once for field `F`.
    fn visit(&mut self, slot: &K::Slot, value: &F::Type);
}

/// Visits every slot of a companion, together with the fields of a value.
pub trait VisitWithValue<K, V>: ShapeWith<K> {
    /// Calls the visitor once per field.
    fn visit_with_value(with: &Self::Out, value: &Self, visitor: &mut V);
}

/// Makes one slot.
pub trait MakeField<S: Shape, F: FieldOf<S>, K: SlotFor<S, F>> {
    /// Called once for field `F`.
    fn make(&mut self) -> K::Slot;
}

/// Makes a companion, slot by slot.
pub trait Make<K, V>: ShapeWith<K> {
    /// Calls the maker once per field.
    fn make(maker: &mut V) -> Self::Out;
}

/// Makes one slot from the field's value.
pub trait MakeFieldFrom<S: Shape, F: FieldOf<S>, K: SlotFor<S, F>> {
    /// Called once for field `F`.
    fn make_from(&mut self, value: &F::Type) -> K::Slot;
}

/// Makes a companion from a value, slot by slot.
pub trait MakeFrom<K, V>: ShapeWith<K> {
    /// Calls the maker once per field.
    fn make_from(value: &Self, maker: &mut V) -> Self::Out;
}

/// Applies one slot to the field.
pub trait ApplyField<S: Shape, F: FieldOf<S>, K: SlotFor<S, F>> {
    /// Called once for field `F`.
    fn apply(&mut self, slot: K::Slot, target: &mut F::Type);
}

/// Applies a companion to a value, slot by slot.
pub trait Apply<K, V>: ShapeWith<K> {
    /// Calls the visitor once per field.
    fn apply(with: Self::Out, target: &mut Self, visitor: &mut V);
}

/// Called once per field, with nothing but the field's type.
///
/// The visitor holds whatever it works on, and reaches each field through
/// [`FieldOf`] and [`SlotOf`]. This covers operations on several values and
/// companions at once, which the other traversals don't.
pub trait EachField<S: Shape, F: FieldOf<S>> {
    /// Called once for field `F`.
    fn each(&mut self);
}

/// Calls a visitor once per field, in declaration order.
pub trait ForEach<V>: Shape {
    /// Calls the visitor once per field.
    fn for_each(visitor: &mut V);
}

// ------------------------------------------------------------------ interop

/// A slot that may hold the field's value. Implementing this makes a
/// companion readable like the struct, through [`ReadField`].
pub trait ReadSlot<T> {
    /// The value, if the slot holds one.
    fn read_slot(&self) -> Option<&T>;
}

impl<T> ReadSlot<T> for Option<T> {
    fn read_slot(&self) -> Option<&T> {
        self.as_ref()
    }
}

impl<T> ReadSlot<T> for Set<T> {
    fn read_slot(&self) -> Option<&T> {
        Some(&self.0)
    }
}

impl<T> ReadSlot<T> for Unset {
    fn read_slot(&self) -> Option<&T> {
        None
    }
}

/// A slot that can take the field's value. Implementing this makes a
/// companion writable like the struct, through [`SetField`].
pub trait WriteSlot<T> {
    /// Puts `value` in the slot.
    fn write_slot(&mut self, value: T);
}

impl<T> WriteSlot<T> for Option<T> {
    fn write_slot(&mut self, value: T) {
        *self = Some(value);
    }
}

impl<T> WriteSlot<T> for Set<T> {
    fn write_slot(&mut self, value: T) {
        self.0 = value;
    }
}

/// The value view: something that can read field `F` of `S`, whatever it
/// holds for it. `S` itself, a companion whose slot is a [`ReadSlot`], a
/// patch (the value after it), a [`Staged`] or [`Tracked`] value.
///
/// Algorithms that only need values are written against this, and work on
/// every representation.
pub trait ReadField<S: Shape, F: FieldOf<S>> {
    /// The field's value, if there is one.
    fn read_field(&self) -> Option<&F::Type>;
}

// ------------------------------------------------------------- slot views

/// The type of the slot of field `F` in `Self`, borrowed for `'a`. Its only
/// use is [`SlotView`].
///
/// This is a trait with a lifetime, not a lifetime GAT on [`SlotView`]: with
/// a GAT, a bound like `for<'a> V::Slot<'a>: Debug` would require
/// `V: 'static`. `Outlives` is never given; its default, `&'a Self`, limits
/// every `for<'a>` bound on this trait to lifetimes that `Self` outlives.
pub trait SlotType<'a, S: Shape, F: FieldOf<S>, Outlives = &'a Self> {
    /// What the representation holds for the field.
    type Slot;
}

/// The slot view: what this representation holds for field `F` of `S`, in
/// its own type. A partial gives `&Option<T>`, a patch a change, a staged
/// value [`StagedSlot`].
///
/// [`DebugShape`] is written on this, so every representation prints its own
/// kind of slot.
pub trait SlotView<S: Shape, F: FieldOf<S>>:
    for<'a> SlotType<'a, S, F>
{
    /// The slot.
    fn slot_view(&self) -> <Self as SlotType<'_, S, F>>::Slot;
}

/// `Debug` for representation `V` of `Self`, written once on the slot view.
///
/// `#[derive(Muck)]` implements `Debug` for every representation by calling
/// this, and writes a default that prints every slot. With
/// `#[muck(debug = custom)]`, the user writes it instead, for example to hide
/// a field: a slot that isn't printed doesn't need `Debug`.
pub trait DebugShape<V: ?Sized> {
    /// Prints `view`.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error.
    fn fmt_shape(view: &V, f: &mut std::fmt::Formatter<'_>)
    -> std::fmt::Result;
}

/// The slot of a [`Staged`] value: the staged value, or the target's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StagedSlot<T> {
    /// Nothing is staged for the field.
    Unchanged(T),
    /// This value is staged for the field.
    Staged(T),
}

/// Something that accepts writes to field `F` of `S`: `S` itself, a
/// companion whose slot is a [`WriteSlot`], a [`Tracked`] or [`Staged`]
/// value, or a recorder.
pub trait SetField<S: Shape, F: FieldOf<S>> {
    /// Writes the field.
    fn set_field(&mut self, value: F::Type);
}

// Through references too, so the named methods (`read_bar`) also work on a
// `&mut T` in generic code, where method lookup tries `Self = &mut T` first.

impl<S: Shape, F: FieldOf<S>, R: ReadField<S, F> + ?Sized> ReadField<S, F>
    for &R
{
    fn read_field(&self) -> Option<&F::Type> {
        (**self).read_field()
    }
}

impl<S: Shape, F: FieldOf<S>, R: ReadField<S, F> + ?Sized> ReadField<S, F>
    for &mut R
{
    fn read_field(&self) -> Option<&F::Type> {
        (**self).read_field()
    }
}

impl<S: Shape, F: FieldOf<S>, R: SetField<S, F> + ?Sized> SetField<S, F>
    for &mut R
{
    fn set_field(&mut self, value: F::Type) {
        (**self).set_field(value);
    }
}

/// Sees each write to field `F` of a [`Tracked`] value, just before it
/// happens.
///
/// `()` observes nothing, and a pair observes with both, so observers chain.
pub trait Observe<S: Shape, F: FieldOf<S>> {
    /// Called with the current value and the one about to be written.
    fn observe(&mut self, old: &F::Type, new: &F::Type);
}

impl<S: Shape, F: FieldOf<S>> Observe<S, F> for () {
    fn observe(&mut self, _old: &F::Type, _new: &F::Type) {}
}

impl<S, F, A, B> Observe<S, F> for (A, B)
where
    S: Shape,
    F: FieldOf<S>,
    A: Observe<S, F>,
    B: Observe<S, F>,
{
    fn observe(&mut self, old: &F::Type, new: &F::Type) {
        self.0.observe(old, new);
        self.1.observe(old, new);
    }
}

/// A borrowed value whose writes are seen by an observer.
#[derive(Debug)]
pub struct Tracked<'a, S, O> {
    value:    &'a mut S,
    observer: O,
}

impl<'a, S, O> Tracked<'a, S, O> {
    /// Tracks writes to `value`.
    pub const fn new(value: &'a mut S, observer: O) -> Self {
        Self { value, observer }
    }

    /// The value being written.
    #[must_use]
    pub const fn value(&self) -> &S {
        self.value
    }

    /// The observer.
    #[must_use]
    pub const fn observer(&self) -> &O {
        &self.observer
    }

    /// Ends tracking, returning the observer.
    #[must_use]
    pub fn into_observer(self) -> O {
        self.observer
    }
}

impl<S, F, O> SetField<S, F> for Tracked<'_, S, O>
where
    S: Shape,
    F: FieldOf<S>,
    O: Observe<S, F>,
{
    fn set_field(&mut self, value: F::Type) {
        let field = F::get_mut(self.value);
        self.observer.observe(field, &value);
        *field = value;
    }
}

impl<S, F, O> ReadField<S, F> for Tracked<'_, S, O>
where
    S: Shape + ReadField<S, F>,
    F: FieldOf<S>,
{
    fn read_field(&self) -> Option<&F::Type> {
        self.value.read_field()
    }
}

/// A borrowed value with writes staged in a buffer, leaving the value as it
/// is. Reads see the buffer first, then the value.
///
/// With a [`Recorder`] as the buffer, this is a patch being built.
///
/// It prints through `S`'s [`DebugShape`], with a [`StagedSlot`] per field.
pub struct Staged<'a, S, B> {
    target: &'a S,
    buffer: B,
}

impl<'a, S, B> Staged<'a, S, B> {
    /// Stages writes to `target` in `buffer`.
    pub const fn new(target: &'a S, buffer: B) -> Self {
        Self { target, buffer }
    }

    /// The value the writes are staged for.
    #[must_use]
    pub const fn target(&self) -> &'a S {
        self.target
    }

    /// The staged writes.
    #[must_use]
    pub const fn buffer(&self) -> &B {
        &self.buffer
    }

    /// The staged writes, mutably.
    #[must_use]
    pub const fn buffer_mut(&mut self) -> &mut B {
        &mut self.buffer
    }

    /// Ends staging, returning the value and the staged writes.
    #[must_use]
    pub fn into_parts(self) -> (&'a S, B) {
        (self.target, self.buffer)
    }
}

impl<S, B: Clone> Clone for Staged<'_, S, B> {
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            buffer: self.buffer.clone(),
        }
    }
}

impl<S, F, B> SetField<S, F> for Staged<'_, S, B>
where
    S: Shape,
    F: FieldOf<S>,
    B: SetField<S, F>,
{
    fn set_field(&mut self, value: F::Type) {
        self.buffer.set_field(value);
    }
}

impl<S, F, B> ReadField<S, F> for Staged<'_, S, B>
where
    S: Shape + ReadField<S, F>,
    F: FieldOf<S>,
    B: ReadField<S, F>,
{
    fn read_field(&self) -> Option<&F::Type> {
        self.buffer
            .read_field()
            .or_else(|| self.target.read_field())
    }
}

impl<'v, S, F, B> SlotType<'v, S, F> for Staged<'_, S, B>
where
    S: Shape,
    F: FieldOf<S>,
    F::Type: 'v,
{
    type Slot = StagedSlot<&'v F::Type>;
}

impl<S, F, B> SlotView<S, F> for Staged<'_, S, B>
where
    S: Shape,
    F: FieldOf<S>,
    B: ReadField<S, F>,
    Self: for<'v> SlotType<'v, S, F, Slot = StagedSlot<&'v F::Type>>,
{
    fn slot_view(&self) -> StagedSlot<&F::Type> {
        match self.buffer.read_field() {
            Some(staged) => StagedSlot::Staged(staged),
            None => StagedSlot::Unchanged(F::get(self.target)),
        }
    }
}

impl<S: Shape, B> Representation<S> for Staged<'_, S, B> {
    const KIND: &'static str = "Staged";
}

impl<'a, S, B> Debug for Staged<'a, S, B>
where
    S: DebugShape<Staged<'a, S, B>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        S::fmt_shape(self, f)
    }
}

// ---------------------------------------------------------------- recorders

/// Makes every slot of an [`Optional`] companion empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Nothing;

impl<S: Shape, F: FieldOf<S>> MakeField<S, F, Optional> for Nothing {
    fn make(&mut self) -> Option<F::Type> {
        None
    }
}

/// The last value written to each field of an `S`, as a partial.
///
/// Written to directly or as a [`Staged`] buffer, it takes the value. As an
/// observer of a [`Tracked`] value, it copies it.
pub struct Recorder<S: ShapeWith<Optional>> {
    values: With<S, Optional>,
}

/// The first value each field of an `S` had before it was written, as a
/// partial. As an observer of a [`Tracked`] value, it collects exactly what an
/// undo needs to write back.
pub struct Previous<S: ShapeWith<Optional>> {
    values: With<S, Optional>,
}

macro_rules! partial_recorder {
    ($name:ident) => {
        impl<S: ShapeWith<Optional>> Debug for $name<S>
        where
            With<S, Optional>: Debug,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.values)
                    .finish()
            }
        }

        impl<S: ShapeWith<Optional>> Clone for $name<S>
        where
            With<S, Optional>: Clone,
        {
            fn clone(&self) -> Self {
                Self {
                    values: self.values.clone(),
                }
            }
        }

        impl<S: Make<Optional, Nothing>> Default for $name<S> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<S: ShapeWith<Optional>> $name<S> {
            /// Nothing recorded yet.
            #[must_use]
            pub fn new() -> Self
            where
                S: Make<Optional, Nothing>,
            {
                Self {
                    values: S::make(&mut Nothing),
                }
            }

            /// The recorded values.
            #[must_use]
            pub const fn values(&self) -> &With<S, Optional> {
                &self.values
            }

            /// Ends recording, returning the values.
            #[must_use]
            pub fn into_values(self) -> With<S, Optional> {
                self.values
            }

            /// The value recorded for field `F`, if any.
            #[must_use]
            pub fn get<F>(&self) -> Option<&F::Type>
            where
                F: SlotOf<S, Optional>,
            {
                F::slot(&self.values).as_ref()
            }
        }

        impl<S, F> ReadField<S, F> for $name<S>
        where
            S: ShapeWith<Optional>,
            F: SlotOf<S, Optional>,
        {
            fn read_field(&self) -> Option<&F::Type> {
                self.get::<F>()
            }
        }

        impl<'v, S, F> SlotType<'v, S, F> for $name<S>
        where
            S: ShapeWith<Optional>,
            F: FieldOf<S>,
            F::Type: 'v,
        {
            type Slot = Option<&'v F::Type>;
        }

        impl<S, F> SlotView<S, F> for $name<S>
        where
            S: ShapeWith<Optional>,
            F: SlotOf<S, Optional>,
            Self: for<'v> SlotType<'v, S, F, Slot = Option<&'v F::Type>>,
        {
            fn slot_view(&self) -> Option<&F::Type> {
                self.get::<F>()
            }
        }
    };
}

partial_recorder!(Recorder);
partial_recorder!(Previous);

impl<S, F> Observe<S, F> for Recorder<S>
where
    S: ShapeWith<Optional>,
    F: SlotOf<S, Optional>,
    F::Type: Clone,
{
    fn observe(&mut self, _old: &F::Type, new: &F::Type) {
        *F::slot_mut(&mut self.values) = Some(new.clone());
    }
}

impl<S, F> SetField<S, F> for Recorder<S>
where
    S: ShapeWith<Optional>,
    F: SlotOf<S, Optional>,
{
    fn set_field(&mut self, value: F::Type) {
        *F::slot_mut(&mut self.values) = Some(value);
    }
}

impl<S, F> Observe<S, F> for Previous<S>
where
    S: ShapeWith<Optional>,
    F: SlotOf<S, Optional>,
    F::Type: Clone,
{
    fn observe(&mut self, old: &F::Type, _new: &F::Type) {
        let slot = F::slot_mut(&mut self.values);
        if slot.is_none() {
            *slot = Some(old.clone());
        }
    }
}
