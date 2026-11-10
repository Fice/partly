//! A patch for any `fieldmuck` shape, written once instead of per type.
//!
//! A patch is two companions: the value it was made against (its *base*) and
//! the values it writes (its *changes*). Each field has a [`PatchMode`]:
//! [`Patched`] fields have a value in the base and an `Option` in the
//! changes; [`Skipped`] fields (`#[partly(skip(patch))]`) have neither, and
//! need no `Clone` or `PartialEq`.
//!
//! Every operation is a visitor run once per field through [`ForEach`]. The
//! derive only adds thin impls that call into this module, plus named
//! methods.

use std::convert::Infallible;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use fieldmuck::{
    Accepts, DebugShape, EachField, FieldOf, ForEach, Kind, Make, MakeField,
    Nothing, Optional, ReadField, Representation, SetField, Shape, ShapeWith,
    SlotFor, SlotOf, SlotType, SlotView, Staged, With,
};

use crate::{
    ApplyFailure, BoundPatch, ChainFailure, Conflict, FieldError, Partly,
    PatchOps, PatchValue, Patchable, Rejected, Resolution, ValidateField,
    ValidateWhole, ValidationErrors,
};

/// The merge strategy's decision for field `Self` of `S`.
///
/// The derive implements this on each field's type, for every `M` that
/// implements the type's named merge strategy trait (`resolve_bar`). It is
/// never called for a [`Skipped`] field.
pub trait MergeField<S: Shape, M>: FieldOf<S> {
    /// The strategy's error.
    type Error;

    /// Settles a field whose value differs from the one the patch was made
    /// against. `new` is `None` if the patch doesn't write the field.
    ///
    /// # Errors
    ///
    /// Returns the strategy's error if it doesn't settle the conflict.
    fn resolve(
        strategy: &M,
        current: &Self::Type,
        old: &Self::Type,
        new: Option<&Self::Type>,
    ) -> Result<Resolution<Self::Type>, Self::Error>;
}

// -------------------------------------------------------------------- modes

/// How field `Self` of `S` takes part in a patch. The derive implements this
/// for every field: [`Patched`], or [`Skipped`] for `#[partly(skip(patch))]`.
pub trait PatchField<S: Shape>: FieldOf<S> {
    /// The field's mode.
    type Mode: PatchMode<Self::Type>;
}

/// What a patch holds for a field of type `T`, and how it compares and
/// copies values. Every operation on a patch goes through this, so a
/// [`Skipped`] field's type needs no trait at all.
pub trait PatchMode<T> {
    /// The base slot: the value the patch was made against.
    type Base;
    /// The change slot: the value the patch writes.
    type Change;

    /// Copies a value into a base slot.
    fn base(value: &T) -> Self::Base;
    /// The old value in a base slot.
    fn old_value(base: &Self::Base) -> Option<&T>;
    /// The old value in a base slot, mutably.
    fn old_value_mut(base: &mut Self::Base) -> Option<&mut T>;

    /// A change slot that writes nothing.
    fn unchanged() -> Self::Change;
    /// The value a change slot writes.
    fn new_value(change: &Self::Change) -> Option<&T>;
    /// The value a change slot writes, mutably.
    fn new_value_mut(change: &mut Self::Change) -> Option<&mut T>;
    /// Takes the value out of a change slot.
    fn take(change: &mut Self::Change) -> Option<T>;
    /// Puts a value in a change slot.
    fn put(change: &mut Self::Change, value: T);

    /// Copies a value, for a change.
    fn copy(value: &T) -> Option<T>;
    /// Compares two values.
    fn same(left: &T, right: &T) -> bool;
}

/// A field that takes part in patches. Needs `Clone + PartialEq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Patched;

impl<T: Clone + PartialEq> PatchMode<T> for Patched {
    type Base = T;
    type Change = Option<T>;

    fn base(value: &T) -> T {
        value.clone()
    }

    fn old_value(base: &T) -> Option<&T> {
        Some(base)
    }

    fn old_value_mut(base: &mut T) -> Option<&mut T> {
        Some(base)
    }

    fn unchanged() -> Option<T> {
        None
    }

    fn new_value(change: &Option<T>) -> Option<&T> {
        change.as_ref()
    }

    fn new_value_mut(change: &mut Option<T>) -> Option<&mut T> {
        change.as_mut()
    }

    fn take(change: &mut Option<T>) -> Option<T> {
        change.take()
    }

    fn put(change: &mut Option<T>, value: T) {
        *change = Some(value);
    }

    fn copy(value: &T) -> Option<T> {
        Some(value.clone())
    }

    fn same(left: &T, right: &T) -> bool {
        left == right
    }
}

/// A field left out of patches, with `#[partly(skip(patch))]`: a patch
/// neither checks nor writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Skipped;

/// The slot of a [`Skipped`] field of type `T`, in a patch's base and
/// changes. It holds nothing.
///
/// It names `T` so that a kind can accept it next to `T` itself: the impls
/// `Accepts<T, T>` and `Accepts<T, Skip<T>>` can't overlap, since `T` is never
/// `Skip<T>`. With a plain `Unset` they would, at `T = Unset`.
pub struct Skip<T>(PhantomData<fn() -> T>);

impl<T> Skip<T> {
    /// The empty slot.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

// Written out: derives would require the traits of `T`, which a skipped
// field's type usually lacks.

impl<T> Debug for Skip<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Skip")
    }
}

impl<T> Clone for Skip<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Skip<T> {}

impl<T> PartialEq for Skip<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<T> Eq for Skip<T> {}

impl<T> std::hash::Hash for Skip<T> {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl<T> Default for Skip<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PatchMode<T> for Skipped {
    type Base = Skip<T>;
    type Change = Skip<T>;

    fn base(_value: &T) -> Skip<T> {
        Skip::new()
    }

    fn old_value(_base: &Skip<T>) -> Option<&T> {
        None
    }

    fn old_value_mut(_base: &mut Skip<T>) -> Option<&mut T> {
        None
    }

    fn unchanged() -> Skip<T> {
        Skip::new()
    }

    fn new_value(_change: &Skip<T>) -> Option<&T> {
        None
    }

    fn new_value_mut(_change: &mut Skip<T>) -> Option<&mut T> {
        None
    }

    fn take(_change: &mut Skip<T>) -> Option<T> {
        None
    }

    /// Never called: no operation has a value for a skipped field.
    fn put(_change: &mut Skip<T>, _value: T) {}

    fn copy(_value: &T) -> Option<T> {
        None
    }

    fn same(_left: &T, _right: &T) -> bool {
        true
    }
}

/// The mode of field `F` of `S`.
type ModeOf<S, F> = <F as PatchField<S>>::Mode;

/// The fields patches of `Self` can write, in declaration order. The derive
/// implements this with every field but the `skip(patch)` ones.
pub trait PatchShape: Shape {
    /// The [`Patched`] fields.
    const FIELDS: &'static [Self::Fields];
}

// -------------------------------------------------------------------- kinds

/// The kind of a patch's base: the value of each patched field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PatchBase;

impl<S: Shape, F: PatchField<S>> SlotFor<S, F> for PatchBase {
    type Slot = <F::Mode as PatchMode<F::Type>>::Base;
}

impl Kind for PatchBase {
    const NAME: &'static str = "PatchBase";
}

/// A patched field's base slot is its value; a skipped field's is a [`Skip`].
impl<T> Accepts<T, T> for PatchBase {}
impl<T> Accepts<T, Skip<T>> for PatchBase {}

/// The kind of a patch's changes: an optional value for each patched field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PatchChanges;

impl<S: Shape, F: PatchField<S>> SlotFor<S, F> for PatchChanges {
    type Slot = <F::Mode as PatchMode<F::Type>>::Change;
}

impl Kind for PatchChanges {
    const NAME: &'static str = "Changes";
}

/// A patched field's change slot is an `Option`; a skipped field's is a
/// [`Skip`].
impl<T> Accepts<T, Option<T>> for PatchChanges {}
impl<T> Accepts<T, Skip<T>> for PatchChanges {}

/// The base of a patch for `S`.
pub type Base<S> = With<S, PatchBase>;

/// The changes of a patch for `S`.
pub type Changes<S> = With<S, PatchChanges>;

// --------------------------------------------------------------------- data

/// The value a [`ShapePatch`] was made against, and the values it writes.
pub struct PatchData<S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>> {
    old: Base<S>,
    new: Changes<S>,
}

/// The slot of a patch: what it does to the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldChange<T> {
    /// The patch doesn't write the field, which keeps this value.
    Unchanged(T),
    /// The patch writes `new` over `old`.
    Changed {
        /// The value the patch was made against.
        old: T,
        /// The value the patch writes.
        new: T,
    },
    /// The field takes no part in patches.
    Skipped,
}

/// The slot view of a patch: a [`FieldChange`] per field.
impl<'a, S, F> SlotType<'a, S, F> for PatchData<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    F: FieldOf<S>,
    F::Type: 'a,
{
    type Slot = FieldChange<&'a F::Type>;
}

impl<S, F> SlotView<S, F> for PatchData<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    F: PatchField<S> + SlotOf<S, PatchBase> + SlotOf<S, PatchChanges>,
    Self: for<'a> SlotType<'a, S, F, Slot = FieldChange<&'a F::Type>>,
{
    fn slot_view(&self) -> FieldChange<&F::Type> {
        let old = ModeOf::<S, F>::old_value(<F as SlotOf<S, PatchBase>>::slot(
            &self.old,
        ));
        let new = ModeOf::<S, F>::new_value(
            <F as SlotOf<S, PatchChanges>>::slot(&self.new),
        );
        match (old, new) {
            (Some(old), Some(new)) => FieldChange::Changed { old, new },
            (Some(old), None) => FieldChange::Unchanged(old),
            (None, _) => FieldChange::Skipped,
        }
    }
}

/// A patch prints as `FooPatch`.
impl<S> Representation<S> for PatchData<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
{
    const KIND: &'static str = "Patch";
}

/// The value view of a patch: the value after it. Nothing for a skipped
/// field, which the patch doesn't know.
impl<S, F> ReadField<S, F> for PatchData<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    F: PatchField<S> + SlotOf<S, PatchBase> + SlotOf<S, PatchChanges>,
{
    fn read_field(&self) -> Option<&F::Type> {
        ModeOf::<S, F>::new_value(<F as SlotOf<S, PatchChanges>>::slot(
            &self.new,
        ))
        .or_else(|| {
            ModeOf::<S, F>::old_value(<F as SlotOf<S, PatchBase>>::slot(
                &self.old,
            ))
        })
    }
}

/// Prints through `S`'s [`DebugShape`], with a [`FieldChange`] per field.
impl<S> Debug for PatchData<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges> + DebugShape<Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        S::fmt_shape(self, f)
    }
}

impl<S> Clone for PatchData<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    Base<S>: Clone,
    Changes<S>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            old: self.old.clone(),
            new: self.new.clone(),
        }
    }
}

impl<S> PartialEq for PatchData<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    Base<S>: PartialEq,
    Changes<S>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.old == other.old && self.new == other.new
    }
}

impl<S> Eq for PatchData<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    Base<S>: Eq,
    Changes<S>: Eq,
{
}

/// Changes to an `S`, made against the value of every patched field: a base
/// and the changes. An empty patch applies to anything.
pub struct ShapePatch<S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>> {
    inner: Option<PatchData<S>>,
}

/// The value after the patch, or nothing for an empty patch.
impl<S, F> ReadField<S, F> for ShapePatch<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    F: PatchField<S> + SlotOf<S, PatchBase> + SlotOf<S, PatchChanges>,
{
    fn read_field(&self) -> Option<&F::Type> {
        self.inner.as_ref().and_then(ReadField::<S, F>::read_field)
    }
}

/// Prints like its [`PatchData`], or `EmptyPatch`.
impl<S> Debug for ShapePatch<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    PatchData<S>: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            Some(data) => Debug::fmt(data, f),
            None => f.write_str("EmptyPatch"),
        }
    }
}

impl<S> Clone for ShapePatch<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    PatchData<S>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<S> PartialEq for ShapePatch<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    PatchData<S>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<S> Eq for ShapePatch<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
    PatchData<S>: Eq,
{
}

impl<S> Default for ShapePatch<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> ShapePatch<S>
where
    S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
{
    /// Creates an empty patch, which applies to any value.
    #[must_use]
    pub const fn new() -> Self {
        Self { inner: None }
    }

    /// Creates a patch made against `old`, which writes the fields set on
    /// `new`.
    #[must_use]
    pub const fn from_parts(old: Base<S>, new: Changes<S>) -> Self {
        Self {
            inner: Some(PatchData { old, new }),
        }
    }

    /// The value the patch was made against and the values it writes,
    /// unless it is empty.
    #[must_use]
    pub fn into_parts(self) -> Option<(Base<S>, Changes<S>)> {
        self.inner.map(|data| (data.old, data.new))
    }

    /// The value the patch was made against, unless it is empty.
    #[must_use]
    pub const fn old_value(&self) -> Option<&Base<S>> {
        match &self.inner {
            Some(data) => Some(&data.old),
            None => None,
        }
    }

    /// The values the patch writes, unless it is empty.
    #[must_use]
    pub const fn new_value(&self) -> Option<&Changes<S>> {
        match &self.inner {
            Some(data) => Some(&data.new),
            None => None,
        }
    }

    /// The values the patch writes, mutably, unless it is empty.
    #[must_use]
    pub const fn new_value_mut(&mut self) -> Option<&mut Changes<S>> {
        match &mut self.inner {
            Some(data) => Some(&mut data.new),
            None => None,
        }
    }

    /// The value the patch was made against and what it writes, unless it is
    /// empty. Its slot view gives a [`FieldChange`] per field.
    #[must_use]
    pub const fn data(&self) -> Option<&PatchData<S>> {
        self.inner.as_ref()
    }

    /// The old and new value of field `F`, if the patch writes it.
    #[must_use]
    pub fn get<F>(&self) -> Option<PatchValue<&F::Type>>
    where
        F: PatchField<S> + SlotOf<S, PatchBase> + SlotOf<S, PatchChanges>,
    {
        let data = self.inner.as_ref()?;
        let old = ModeOf::<S, F>::old_value(
            <F as SlotOf<S, PatchBase>>::slot(&data.old),
        )?;
        let new = ModeOf::<S, F>::new_value(
            <F as SlotOf<S, PatchChanges>>::slot(&data.new),
        )?;
        Some(PatchValue::new(old, new))
    }

    /// Returns `true` if the patch writes field `F`.
    #[must_use]
    pub fn writes<F>(&self) -> bool
    where
        F: PatchField<S> + SlotOf<S, PatchChanges>,
    {
        self.inner.as_ref().is_some_and(|data| {
            ModeOf::<S, F>::new_value(<F as SlotOf<S, PatchChanges>>::slot(
                &data.new,
            ))
            .is_some()
        })
    }
}

// ---------------------------------------------------------------- recorder

/// The values written to a patch being built. As the buffer of a
/// [`Staged`] value, it is the bound patch: only patched fields can be
/// written.
pub struct ChangeRecorder<S: ShapeWith<PatchChanges>> {
    changes: Changes<S>,
}

impl<S: ShapeWith<PatchChanges>> Debug for ChangeRecorder<S>
where
    Changes<S>: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ChangeRecorder")
            .field(&self.changes)
            .finish()
    }
}

impl<S: ShapeWith<PatchChanges>> Clone for ChangeRecorder<S>
where
    Changes<S>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            changes: self.changes.clone(),
        }
    }
}

impl<S: Make<PatchChanges, visit::NoChange>> Default for ChangeRecorder<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: ShapeWith<PatchChanges>> ChangeRecorder<S> {
    /// Nothing written yet.
    #[must_use]
    pub fn new() -> Self
    where
        S: Make<PatchChanges, visit::NoChange>,
    {
        Self {
            changes: S::make(&mut visit::NoChange),
        }
    }

    /// The values written so far.
    #[must_use]
    pub const fn changes(&self) -> &Changes<S> {
        &self.changes
    }

    /// Ends recording, returning the values.
    #[must_use]
    pub fn into_changes(self) -> Changes<S> {
        self.changes
    }

    /// The value written to field `F`, if any.
    #[must_use]
    pub fn get<F>(&self) -> Option<&F::Type>
    where
        F: PatchField<S> + SlotOf<S, PatchChanges>,
    {
        ModeOf::<S, F>::new_value(<F as SlotOf<S, PatchChanges>>::slot(
            &self.changes,
        ))
    }
}

impl<S, F> ReadField<S, F> for ChangeRecorder<S>
where
    S: ShapeWith<PatchChanges>,
    F: PatchField<S> + SlotOf<S, PatchChanges>,
{
    fn read_field(&self) -> Option<&F::Type> {
        self.get::<F>()
    }
}

/// Only a patched field can be written; a skipped one has no such impl.
impl<S, F> SetField<S, F> for ChangeRecorder<S>
where
    S: ShapeWith<PatchChanges>,
    F: PatchField<S, Mode = Patched> + SlotOf<S, PatchChanges>,
    F::Type: Clone + PartialEq,
{
    fn set_field(&mut self, value: F::Type) {
        Patched::put(
            <F as SlotOf<S, PatchChanges>>::slot_mut(&mut self.changes),
            value,
        );
    }
}

/// A patch being built: writes are staged over the borrowed value, and
/// `finish` copies its patched fields as the base.
impl<'a, S> BoundPatch<'a> for Staged<'a, S, ChangeRecorder<S>>
where
    S: Patchable<Patch = ShapePatch<S>>
        + ShapeWith<PatchBase>
        + ShapeWith<PatchChanges>
        + for<'v> Make<PatchBase, visit::BaseOf<'v, S>>,
{
    type Target = S;

    fn finish(self) -> ShapePatch<S> {
        let (target, buffer) = self.into_parts();
        ShapePatch::from_parts(base_of(target), buffer.into_changes())
    }
}

// ----------------------------------------------------------------- visitors
//
// Public only because they appear in bounds. Each is run once per field, and
// reaches values only through the field's mode.

/// Visitors run by [`ShapePatch`]. Not meant to be used directly.
#[doc(hidden)]
pub mod visit {
    use super::{
        Base, Changes, Conflict, EachField, FieldError, FieldOf, MakeField,
        MergeField, ModeOf, Optional, PatchBase, PatchChanges, PatchData,
        PatchField, PatchMode, Resolution, Shape, ShapeWith, SlotFor, SlotOf,
        ValidateField, With,
    };

    /// The kind whose slot holds how a field was settled.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Resolving;

    impl<S: Shape, F: FieldOf<S>> SlotFor<S, F> for Resolving {
        type Slot = Option<Resolution<F::Type>>;
    }

    impl<T> fieldmuck::Accepts<T, Option<Resolution<T>>> for Resolving {}

    impl fieldmuck::Kind for Resolving {
        const NAME: &'static str = "Resolutions";
    }

    /// Makes every slot unsettled.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Unsettled;

    impl<S: Shape, F: FieldOf<S>> MakeField<S, F, Resolving> for Unsettled {
        fn make(&mut self) -> Option<Resolution<F::Type>> {
            None
        }
    }

    /// Makes every change slot write nothing.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct NoChange;

    impl<S: Shape, F: PatchField<S>> MakeField<S, F, PatchChanges> for NoChange {
        fn make(&mut self) -> <F::Mode as PatchMode<F::Type>>::Change {
            ModeOf::<S, F>::unchanged()
        }
    }

    /// Makes a base from a value.
    pub struct BaseOf<'a, S>(pub &'a S);

    impl<S: Shape, F: PatchField<S>> MakeField<S, F, PatchBase> for BaseOf<'_, S> {
        fn make(&mut self) -> <F::Mode as PatchMode<F::Type>>::Base {
            ModeOf::<S, F>::base(F::get(self.0))
        }
    }

    /// Whether the changes write a given field.
    pub struct Has<'a, S: ShapeWith<PatchChanges>> {
        pub new:   &'a Changes<S>,
        pub field: S::Fields,
        pub found: bool,
    }

    impl<S, F> EachField<S, F> for Has<'_, S>
    where
        S: ShapeWith<PatchChanges>,
        F: PatchField<S> + SlotOf<S, PatchChanges>,
    {
        fn each(&mut self) {
            if F::FIELD == self.field {
                self.found =
                    ModeOf::<S, F>::new_value(F::slot(self.new)).is_some();
            }
        }
    }

    /// Whether the changes write any field.
    pub struct AnySet<'a, S: ShapeWith<PatchChanges>> {
        pub new:   &'a Changes<S>,
        pub found: bool,
    }

    impl<S, F> EachField<S, F> for AnySet<'_, S>
    where
        S: ShapeWith<PatchChanges>,
        F: PatchField<S> + SlotOf<S, PatchChanges>,
    {
        fn each(&mut self) {
            self.found |=
                ModeOf::<S, F>::new_value(F::slot(self.new)).is_some();
        }
    }

    /// Whether every value written equals the one it replaces.
    pub struct IsNoop<'a, S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>> {
        pub data: &'a PatchData<S>,
        pub noop: bool,
    }

    impl<S, F> EachField<S, F> for IsNoop<'_, S>
    where
        S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
        F: PatchField<S> + SlotOf<S, PatchBase> + SlotOf<S, PatchChanges>,
    {
        fn each(&mut self) {
            let old = ModeOf::<S, F>::old_value(
                <F as SlotOf<S, PatchBase>>::slot(&self.data.old),
            );
            let new =
                ModeOf::<S, F>::new_value(
                    <F as SlotOf<S, PatchChanges>>::slot(&self.data.new),
                );
            if let (Some(old), Some(new)) = (old, new) {
                self.noop &= ModeOf::<S, F>::same(old, new);
            }
        }
    }

    /// The fields where a value differs from a base.
    pub struct Differences<'a, S: ShapeWith<PatchBase>> {
        pub base:   &'a Base<S>,
        pub value:  &'a S,
        pub fields: Vec<Conflict<S::Fields>>,
    }

    impl<S, F> EachField<S, F> for Differences<'_, S>
    where
        S: ShapeWith<PatchBase>,
        F: PatchField<S> + SlotOf<S, PatchBase>,
    {
        fn each(&mut self) {
            if ModeOf::<S, F>::old_value(F::slot(self.base)).is_some_and(
                |old| !ModeOf::<S, F>::same(old, F::get(self.value)),
            ) {
                self.fields.push(Conflict(F::FIELD));
            }
        }
    }

    /// The first field where two bases differ.
    pub struct BasesDiffer<'a, S: ShapeWith<PatchBase>> {
        pub left: &'a Base<S>,
        pub right: &'a Base<S>,
        pub conflict: Option<S::Fields>,
    }

    impl<S, F> EachField<S, F> for BasesDiffer<'_, S>
    where
        S: ShapeWith<PatchBase>,
        F: PatchField<S> + SlotOf<S, PatchBase>,
    {
        fn each(&mut self) {
            if self.conflict.is_none()
                && matches!(
                    (
                        ModeOf::<S, F>::old_value(F::slot(self.left)),
                        ModeOf::<S, F>::old_value(F::slot(self.right)),
                    ),
                    (Some(a), Some(b)) if !ModeOf::<S, F>::same(a, b)
                )
            {
                self.conflict = Some(F::FIELD);
            }
        }
    }

    /// The first field where `next` isn't made against what `first` leaves.
    pub struct Continues<'a, S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>> {
        pub first:    &'a PatchData<S>,
        pub next_old: &'a Base<S>,
        pub conflict: Option<S::Fields>,
    }

    impl<S, F> EachField<S, F> for Continues<'_, S>
    where
        S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
        F: PatchField<S> + SlotOf<S, PatchBase> + SlotOf<S, PatchChanges>,
    {
        fn each(&mut self) {
            let left =
                ModeOf::<S, F>::new_value(
                    <F as SlotOf<S, PatchChanges>>::slot(&self.first.new),
                )
                .or_else(|| {
                    ModeOf::<S, F>::old_value(
                        <F as SlotOf<S, PatchBase>>::slot(&self.first.old),
                    )
                });
            let right =
                ModeOf::<S, F>::old_value(<F as SlotOf<S, PatchBase>>::slot(
                    self.next_old,
                ));
            if self.conflict.is_none()
                && matches!((left, right), (Some(a), Some(b)) if !ModeOf::<S, F>::same(a, b))
            {
                self.conflict = Some(F::FIELD);
            }
        }
    }

    /// The first field two sets of changes write differently.
    pub struct WritesDiffer<'a, S: ShapeWith<PatchChanges>> {
        pub left: &'a Changes<S>,
        pub right: &'a Changes<S>,
        pub conflict: Option<S::Fields>,
    }

    impl<S, F> EachField<S, F> for WritesDiffer<'_, S>
    where
        S: ShapeWith<PatchChanges>,
        F: PatchField<S> + SlotOf<S, PatchChanges>,
    {
        fn each(&mut self) {
            if self.conflict.is_none()
                && matches!(
                    (
                        ModeOf::<S, F>::new_value(F::slot(self.left)),
                        ModeOf::<S, F>::new_value(F::slot(self.right)),
                    ),
                    (Some(a), Some(b)) if !ModeOf::<S, F>::same(a, b)
                )
            {
                self.conflict = Some(F::FIELD);
            }
        }
    }

    /// Moves every value `from` writes into `into`.
    pub struct Absorb<'a, S: ShapeWith<PatchChanges>> {
        pub into: &'a mut Changes<S>,
        pub from: &'a mut Changes<S>,
    }

    impl<S, F> EachField<S, F> for Absorb<'_, S>
    where
        S: ShapeWith<PatchChanges>,
        F: PatchField<S> + SlotOf<S, PatchChanges>,
    {
        fn each(&mut self) {
            if let Some(value) = ModeOf::<S, F>::take(F::slot_mut(self.from)) {
                ModeOf::<S, F>::put(F::slot_mut(self.into), value);
            }
        }
    }

    /// Moves the changes into a partial.
    pub struct IntoPartial<'a, S: ShapeWith<PatchChanges> + ShapeWith<Optional>> {
        pub from: &'a mut Changes<S>,
        pub into: &'a mut With<S, Optional>,
    }

    impl<S, F> EachField<S, F> for IntoPartial<'_, S>
    where
        S: ShapeWith<PatchChanges> + ShapeWith<Optional>,
        F: PatchField<S> + SlotOf<S, PatchChanges> + SlotOf<S, Optional>,
    {
        fn each(&mut self) {
            *<F as SlotOf<S, Optional>>::slot_mut(self.into) =
                ModeOf::<S, F>::take(<F as SlotOf<S, PatchChanges>>::slot_mut(
                    self.from,
                ));
        }
    }

    /// Swaps every written value with the old one.
    pub struct Invert<'a, S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>> {
        pub data: &'a mut PatchData<S>,
    }

    impl<S, F> EachField<S, F> for Invert<'_, S>
    where
        S: ShapeWith<PatchBase> + ShapeWith<PatchChanges>,
        F: PatchField<S> + SlotOf<S, PatchBase> + SlotOf<S, PatchChanges>,
    {
        fn each(&mut self) {
            let old =
                ModeOf::<S, F>::old_value_mut(
                    <F as SlotOf<S, PatchBase>>::slot_mut(&mut self.data.old),
                );
            let new = ModeOf::<S, F>::new_value_mut(<F as SlotOf<
                S,
                PatchChanges,
            >>::slot_mut(
                &mut self.data.new
            ));
            if let (Some(old), Some(new)) = (old, new) {
                std::mem::swap(old, new);
            }
        }
    }

    /// Writes the values in which `new` differs from `old`.
    pub struct Diff<'a, S: ShapeWith<PatchChanges>> {
        pub old: &'a S,
        pub new: &'a S,
        pub out: &'a mut Changes<S>,
    }

    impl<S, F> EachField<S, F> for Diff<'_, S>
    where
        S: ShapeWith<PatchChanges>,
        F: PatchField<S> + SlotOf<S, PatchChanges>,
    {
        fn each(&mut self) {
            let new = F::get(self.new);
            if !ModeOf::<S, F>::same(F::get(self.old), new) {
                if let Some(copy) = ModeOf::<S, F>::copy(new) {
                    ModeOf::<S, F>::put(F::slot_mut(self.out), copy);
                }
            }
        }
    }

    /// Phase 1 of applying: settles every patched field, stopping at an
    /// error.
    pub struct Settle<
        'a,
        S: ShapeWith<PatchBase> + ShapeWith<PatchChanges> + ShapeWith<Resolving>,
        M,
        E,
    > {
        pub target: &'a S,
        pub data: &'a PatchData<S>,
        pub strategy: &'a M,
        pub settled: &'a mut With<S, Resolving>,
        pub error: Option<E>,
    }

    impl<S, F, M, E> EachField<S, F> for Settle<'_, S, M, E>
    where
        S: ShapeWith<PatchBase>
            + ShapeWith<PatchChanges>
            + ShapeWith<Resolving>,
        F: PatchField<S>
            + SlotOf<S, PatchBase>
            + SlotOf<S, PatchChanges>
            + SlotOf<S, Resolving>
            + MergeField<S, M, Error = E>,
    {
        fn each(&mut self) {
            if self.error.is_some() {
                return;
            }
            let Some(old) =
                ModeOf::<S, F>::old_value(<F as SlotOf<S, PatchBase>>::slot(
                    &self.data.old,
                ))
            else {
                return;
            };
            let new =
                ModeOf::<S, F>::new_value(
                    <F as SlotOf<S, PatchChanges>>::slot(&self.data.new),
                );
            let current = F::get(self.target);
            let resolution = if ModeOf::<S, F>::same(current, old) {
                new.is_some().then_some(Resolution::UseNew)
            } else {
                match F::resolve(self.strategy, current, old, new) {
                    Ok(resolution) => Some(resolution),
                    Err(error) => {
                        self.error = Some(error);
                        return;
                    },
                }
            };
            *<F as SlotOf<S, Resolving>>::slot_mut(self.settled) = resolution;
        }
    }

    /// Phase 2 of applying: validates every value that would be written.
    pub struct CheckChosen<
        'a,
        S: ShapeWith<PatchBase> + ShapeWith<PatchChanges> + ShapeWith<Resolving>,
        V,
        E,
    > {
        pub data: &'a PatchData<S>,
        pub settled: &'a With<S, Resolving>,
        pub validator: &'a V,
        pub errors: Vec<FieldError<S::Fields, E>>,
    }

    impl<S, F, V, E> EachField<S, F> for CheckChosen<'_, S, V, E>
    where
        S: ShapeWith<PatchBase>
            + ShapeWith<PatchChanges>
            + ShapeWith<Resolving>,
        F: PatchField<S>
            + SlotOf<S, PatchChanges>
            + SlotOf<S, Resolving>
            + ValidateField<S, V, Error = E>,
    {
        fn each(&mut self) {
            let new =
                ModeOf::<S, F>::new_value(
                    <F as SlotOf<S, PatchChanges>>::slot(&self.data.new),
                );
            let chosen = <F as SlotOf<S, Resolving>>::slot(self.settled)
                .as_ref()
                .and_then(|resolution| resolution.chosen(new));
            if let Some(Err(error)) =
                chosen.map(|value| F::validate_field(self.validator, value))
            {
                self.errors.push(FieldError {
                    field: F::FIELD,
                    error,
                });
            }
        }
    }

    /// Phase 3 of applying: swaps every chosen value in. Running it twice
    /// undoes it.
    pub struct Swap<'a, S: ShapeWith<PatchChanges> + ShapeWith<Resolving>> {
        pub target: &'a mut S,
        pub new: &'a mut Changes<S>,
        pub settled: &'a mut With<S, Resolving>,
    }

    impl<S, F> EachField<S, F> for Swap<'_, S>
    where
        S: ShapeWith<PatchChanges> + ShapeWith<Resolving>,
        F: PatchField<S> + SlotOf<S, PatchChanges> + SlotOf<S, Resolving>,
    {
        fn each(&mut self) {
            if let Some(resolution) =
                <F as SlotOf<S, Resolving>>::slot_mut(self.settled)
            {
                resolution.__swap(
                    F::get_mut(self.target),
                    ModeOf::<S, F>::new_value_mut(<F as SlotOf<
                        S,
                        PatchChanges,
                    >>::slot_mut(
                        self.new
                    )),
                );
            }
        }
    }

    /// After a successful swap: collects the values that were replaced.
    pub struct Replaced<'a, S: ShapeWith<PatchChanges> + ShapeWith<Resolving>> {
        pub new: &'a mut Changes<S>,
        pub settled: &'a mut With<S, Resolving>,
        pub out: &'a mut Changes<S>,
    }

    impl<S, F> EachField<S, F> for Replaced<'_, S>
    where
        S: ShapeWith<PatchChanges> + ShapeWith<Resolving>,
        F: PatchField<S> + SlotOf<S, PatchChanges> + SlotOf<S, Resolving>,
    {
        fn each(&mut self) {
            let new = ModeOf::<S, F>::take(
                <F as SlotOf<S, PatchChanges>>::slot_mut(self.new),
            );
            if let Some(previous) =
                <F as SlotOf<S, Resolving>>::slot_mut(self.settled)
                    .take()
                    .and_then(|resolution| resolution.__previous(new))
            {
                ModeOf::<S, F>::put(
                    <F as SlotOf<S, PatchChanges>>::slot_mut(self.out),
                    previous,
                );
            }
        }
    }
}

use visit::{
    Absorb, AnySet, BaseOf, BasesDiffer, CheckChosen, Continues, Diff,
    Differences, Has, IntoPartial, Invert, IsNoop, NoChange, Replaced,
    Resolving, Settle, Swap, Unsettled, WritesDiffer,
};

/// The base of a patch made against `value`: its patched fields.
pub fn base_of<S>(value: &S) -> Base<S>
where
    S: ShapeWith<PatchBase> + for<'v> Make<PatchBase, BaseOf<'v, S>>,
{
    S::make(&mut BaseOf(value))
}

/// Returns `true` if `changes` write any field.
fn any_set<S>(changes: &Changes<S>) -> bool
where
    S: ShapeWith<PatchChanges> + for<'a> ForEach<AnySet<'a, S>>,
{
    let mut visit = AnySet {
        new:   changes,
        found: false,
    };
    S::for_each(&mut visit);
    visit.found
}

/// Moves every value `from` writes into `into`.
fn absorb<S>(into: &mut Changes<S>, mut from: Changes<S>)
where
    S: ShapeWith<PatchChanges> + for<'a> ForEach<Absorb<'a, S>>,
{
    S::for_each(&mut Absorb {
        into,
        from: &mut from,
    });
}

// ---------------------------------------------------------------- PatchOps

impl<S> PatchOps for ShapePatch<S>
where
    S: Partly<Field = <S as Shape>::Fields, Partial = With<S, Optional>>
        + PatchShape
        + ShapeWith<PatchBase>
        + ShapeWith<PatchChanges>
        + ShapeWith<Optional>
        + Make<Optional, Nothing>
        + for<'a> ForEach<Has<'a, S>>
        + for<'a> ForEach<AnySet<'a, S>>
        + for<'a> ForEach<IsNoop<'a, S>>
        + for<'a> ForEach<Differences<'a, S>>
        + for<'a> ForEach<BasesDiffer<'a, S>>
        + for<'a> ForEach<Continues<'a, S>>
        + for<'a> ForEach<WritesDiffer<'a, S>>
        + for<'a> ForEach<Absorb<'a, S>>
        + for<'a> ForEach<IntoPartial<'a, S>>
        + for<'a> ForEach<Invert<'a, S>>,
{
    type BuildError = Infallible;
    type Conflict = Conflict<<S as Shape>::Fields>;
    type Target = S;

    fn fields(&self) -> &'static [<S as Shape>::Fields] {
        <S as PatchShape>::FIELDS
    }

    fn has(&self, field: <S as Shape>::Fields) -> bool {
        self.inner.as_ref().is_some_and(|data| {
            let mut visit = Has {
                new: &data.new,
                field,
                found: false,
            };
            S::for_each(&mut visit);
            visit.found
        })
    }

    fn is_empty(&self) -> bool {
        self.inner
            .as_ref()
            .is_none_or(|data| !any_set::<S>(&data.new))
    }

    fn is_noop(&self) -> bool {
        self.inner.as_ref().is_none_or(|data| {
            let mut visit = IsNoop { data, noop: true };
            S::for_each(&mut visit);
            visit.noop
        })
    }

    fn check(&self, target: &S) -> Result<(), Vec<Self::Conflict>> {
        let Some(data) = &self.inner else {
            return Ok(());
        };
        let mut visit = Differences {
            base:   &data.old,
            value:  target,
            fields: Vec::new(),
        };
        S::for_each(&mut visit);
        if visit.fields.is_empty() {
            Ok(())
        } else {
            Err(visit.fields)
        }
    }

    fn invert(self) -> Result<Self, Rejected<Self, Infallible>> {
        let Some(mut data) = self.inner else {
            return Ok(Self::new());
        };
        // Swapping turns the base into the value this patch leaves, and the
        // changes into the values it replaces.
        S::for_each(&mut Invert { data: &mut data });
        Ok(Self { inner: Some(data) })
    }

    fn into_partial(self) -> With<S, Optional> {
        let mut partial = S::make(&mut Nothing);
        if let Some(mut data) = self.inner {
            S::for_each(&mut IntoPartial {
                from: &mut data.new,
                into: &mut partial,
            });
        }
        partial
    }

    fn then(
        self,
        next: Self,
    ) -> Result<
        Self,
        Rejected<(Self, Self), ChainFailure<Self::Conflict, Infallible>>,
    > {
        let conflict = match (&self.inner, &next.inner) {
            (Some(first), Some(second)) => {
                let mut visit = Continues {
                    first,
                    next_old: &second.old,
                    conflict: None,
                };
                S::for_each(&mut visit);
                visit.conflict
            },
            _ => None,
        };
        if let Some(field) = conflict {
            return Err(Rejected {
                value: (self, next),
                error: ChainFailure::Conflict(Conflict(field)),
            });
        }
        Ok(match (self.inner, next.inner) {
            (Some(mut first), Some(second)) => {
                absorb::<S>(&mut first.new, second.new);
                Self { inner: Some(first) }
            },
            (first, second) => {
                Self {
                    inner: first.or(second),
                }
            },
        })
    }

    fn merge(
        self,
        other: Self,
    ) -> Result<Self, Rejected<(Self, Self), Self::Conflict>> {
        let conflict = match (&self.inner, &other.inner) {
            (Some(left), Some(right)) => {
                let mut bases = BasesDiffer {
                    left: &left.old,
                    right: &right.old,
                    conflict: None,
                };
                S::for_each(&mut bases);
                bases.conflict.or_else(|| {
                    let mut writes = WritesDiffer {
                        left: &left.new,
                        right: &right.new,
                        conflict: None,
                    };
                    S::for_each(&mut writes);
                    writes.conflict
                })
            },
            _ => None,
        };
        if let Some(field) = conflict {
            return Err(Rejected {
                value: (self, other),
                error: Conflict(field),
            });
        }
        Ok(match (self.inner, other.inner) {
            (Some(mut left), Some(right)) => {
                absorb::<S>(&mut left.new, right.new);
                Self { inner: Some(left) }
            },
            (left, right) => {
                Self {
                    inner: left.or(right),
                }
            },
        })
    }
}

// ------------------------------------------------------------- operations

/// Creates a patch against `old` that writes what differs in `new`. The
/// derive's `Patchable::diff` calls this.
#[must_use]
pub fn diff<S>(old: &S, new: &S) -> ShapePatch<S>
where
    S: ShapeWith<PatchBase>
        + ShapeWith<PatchChanges>
        + Make<PatchChanges, NoChange>
        + for<'v> Make<PatchBase, BaseOf<'v, S>>
        + for<'a> ForEach<Diff<'a, S>>,
{
    let mut out = S::make(&mut NoChange);
    S::for_each(&mut Diff {
        old,
        new,
        out: &mut out,
    });
    ShapePatch::from_parts(base_of(old), out)
}

/// What [`apply`] fails with.
pub type ApplyRejected<S, ME, VE> = Rejected<
    ShapePatch<S>,
    ApplyFailure<ME, ValidationErrors<<S as Shape>::Fields, VE>>,
>;

/// Applies `patch` to `target`, and returns the values it replaced. The
/// derive's `ApplyPatchWith` calls this.
///
/// `ME` and `VE` are the errors of the merge strategy and the validator,
/// which every field's [`MergeField`] and [`ValidateField`] impl must share.
///
/// # Errors
///
/// If this fails, `target` is unchanged and the patch is handed back.
pub fn apply<S, M, V, ME, VE>(
    target: &mut S,
    patch: ShapePatch<S>,
    strategy: &M,
    validator: &V,
) -> Result<Changes<S>, ApplyRejected<S, ME, VE>>
where
    S: ShapeWith<PatchBase>
        + ShapeWith<PatchChanges>
        + ShapeWith<Resolving>
        + Make<PatchChanges, NoChange>
        + Make<Resolving, Unsettled>
        + ValidateWhole<V, Error = VE>
        + for<'a> ForEach<Settle<'a, S, M, ME>>
        + for<'a> ForEach<CheckChosen<'a, S, V, VE>>
        + for<'a> ForEach<Swap<'a, S>>
        + for<'a> ForEach<Replaced<'a, S>>,
{
    let Some(mut data) = patch.inner else {
        return Ok(S::make(&mut NoChange));
    };

    // Phase 1: settle every patched field. A field that still holds its old
    // value takes the patch's value, if any, and any other goes to the
    // strategy.
    let mut settled = <S as Make<Resolving, Unsettled>>::make(&mut Unsettled);
    let mut settle = Settle {
        target,
        data: &data,
        strategy,
        settled: &mut settled,
        error: None,
    };
    S::for_each(&mut settle);
    if let Some(error) = settle.error {
        return Err(Rejected {
            value: ShapePatch { inner: Some(data) },
            error: ApplyFailure::Merge(error),
        });
    }

    // Phase 2: validate the values that would be written.
    let mut check = CheckChosen {
        data: &data,
        settled: &settled,
        validator,
        errors: Vec::new(),
    };
    S::for_each(&mut check);
    if !check.errors.is_empty() {
        let errors = check.errors;
        return Err(Rejected {
            value: ShapePatch { inner: Some(data) },
            error: ApplyFailure::Validation(ValidationErrors::Fields(errors)),
        });
    }

    // Phase 3: write, and undo if the whole-value check fails. Swapping twice
    // restores both sides, so the patch comes back intact.
    S::for_each(&mut Swap {
        target,
        new: &mut data.new,
        settled: &mut settled,
    });
    if let Err(error) = target.validate_whole(validator) {
        S::for_each(&mut Swap {
            target,
            new: &mut data.new,
            settled: &mut settled,
        });
        return Err(Rejected {
            value: ShapePatch { inner: Some(data) },
            error: ApplyFailure::Validation(ValidationErrors::Struct(error)),
        });
    }

    // After the swaps, the patch holds the values that were replaced.
    let mut replaced = S::make(&mut NoChange);
    S::for_each(&mut Replaced {
        new: &mut data.new,
        settled: &mut settled,
        out: &mut replaced,
    });
    Ok(replaced)
}

/// Like [`apply`], and returns the patch that undoes exactly what was
/// written.
///
/// # Errors
///
/// If this fails, `target` is unchanged and the patch is handed back.
pub fn apply_get_undo<S, M, V, ME, VE>(
    target: &mut S,
    patch: ShapePatch<S>,
    strategy: &M,
    validator: &V,
) -> Result<ShapePatch<S>, ApplyRejected<S, ME, VE>>
where
    S: ShapeWith<PatchBase>
        + ShapeWith<PatchChanges>
        + ShapeWith<Resolving>
        + Make<PatchChanges, NoChange>
        + Make<Resolving, Unsettled>
        + for<'v> Make<PatchBase, BaseOf<'v, S>>
        + ValidateWhole<V, Error = VE>
        + for<'a> ForEach<Settle<'a, S, M, ME>>
        + for<'a> ForEach<CheckChosen<'a, S, V, VE>>
        + for<'a> ForEach<Swap<'a, S>>
        + for<'a> ForEach<Replaced<'a, S>>
        + for<'a> ForEach<AnySet<'a, S>>,
{
    let replaced = apply(target, patch, strategy, validator)?;
    Ok(if any_set::<S>(&replaced) {
        ShapePatch::from_parts(base_of(target), replaced)
    } else {
        ShapePatch::new()
    })
}
