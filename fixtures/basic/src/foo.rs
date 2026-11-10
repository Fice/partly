//! The complete dynamic expansion for a struct with no attributes, built on
//! `fieldmuck`.
//!
//! ```ignore
//! #[derive(Muck, Partly)]
//! pub struct Foo {
//!     pub bar: u32,
//!     pub zee: String,
//! }
//! ```
//!
//! The first half is what `#[derive(Muck)]` generates: the shape, which any
//! `fieldmuck` library works with. The second half is what `#[derive(Partly)]`
//! adds on top: named methods and the impls that connect `Foo`'s named traits
//! to the generic code in `partly`. The patch has no per-type code left: it is
//! `partly::ShapePatch<Foo>`.

use std::convert::Infallible;
use std::fmt::Debug;
use std::marker::PhantomData;

use fieldmuck::{
    Accepts, DebugShape, EachField, FieldOf, ForEach, Kind, Make, MakeField,
    Optional, ReadField, ReadSlot, Representation, SetField, Shape, ShapeWith,
    SlotFor, SlotOf, SlotType, SlotView, Staged, WriteSlot, name_of,
};
use partly::prelude::*;
use partly::{
    AcceptAll, ApplyFailure, BuildFailure, ChangeRecorder, Conflict,
    DefaultBuildStrategy, Field, FieldError, FieldMerge, MergeField,
    MissingField, PatchField, PatchValue, Patched, Rejected, Resolution, Set,
    ShapePatch, StrictMerge, ValidateField, ValidateWhole, ValidationErrors,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Foo {
    pub bar: u32,
    pub zee: String,
}

// ======================================================= #[derive(Muck)]

// ---------------------------------------------------------------- per struct

/// Names a field of [`Foo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FooFields {
    Bar,
    Zee,
}

impl Field for FooFields {
    const ALL: &'static [Self] = &[Self::Bar, Self::Zee];

    fn name(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Zee => "zee",
        }
    }
}

/// The fields of [`Foo`], as types.
pub mod foo_field {
    /// The field `bar`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Bar;
    /// The field `zee`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Zee;
}

/// [`Foo`] with a slot of any type in place of each field. Every companion
/// of `Foo`, including its partial, is an instance of this. `K` is the kind,
/// which names it: the partial prints as `FooPartial`. The kind is private
/// and checked by every constructor, so it can't disagree with the slots.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FooShape<B, Z, K = ()> {
    pub bar: B,
    pub zee: Z,
    __kind:  PhantomData<K>,
}

impl<B, Z, K> FooShape<B, Z, K> {
    /// Creates the companion from its slots.
    pub const fn from_slots(bar: B, zee: Z) -> Self
    where
        K: Accepts<u32, B> + Accepts<String, Z>,
    {
        Self {
            bar,
            zee,
            __kind: PhantomData,
        }
    }
}

impl<B: Default, Z: Default, K> Default for FooShape<B, Z, K>
where
    K: Accepts<u32, B> + Accepts<String, Z>,
{
    fn default() -> Self {
        Self::from_slots(B::default(), Z::default())
    }
}

impl Shape for Foo {
    type Fields = FooFields;

    const NAME: &'static str = "Foo";
}

impl Representation<Foo> for Foo {
    const KIND: &'static str = "";
}

impl<B, Z, K: Kind> Representation<Foo> for FooShape<B, Z, K> {
    const KIND: &'static str = K::NAME;
}

impl<K> ShapeWith<K> for Foo
where
    K: SlotFor<Foo, foo_field::Bar> + SlotFor<Foo, foo_field::Zee>,
{
    type Out = FooShape<
        <K as SlotFor<Foo, foo_field::Bar>>::Slot,
        <K as SlotFor<Foo, foo_field::Zee>>::Slot,
        K,
    >;
}

// ----------------------------------------------------------------- per field

impl FieldOf<Foo> for foo_field::Bar {
    type Type = u32;

    const FIELD: FooFields = FooFields::Bar;

    fn get(value: &Foo) -> &u32 {
        &value.bar
    }

    fn get_mut(value: &mut Foo) -> &mut u32 {
        &mut value.bar
    }
}

impl FieldOf<Foo> for foo_field::Zee {
    type Type = String;

    const FIELD: FooFields = FooFields::Zee;

    fn get(value: &Foo) -> &String {
        &value.zee
    }

    fn get_mut(value: &mut Foo) -> &mut String {
        &mut value.zee
    }
}

impl<K> SlotOf<Foo, K> for foo_field::Bar
where
    K: SlotFor<Foo, foo_field::Bar> + SlotFor<Foo, foo_field::Zee>,
{
    fn slot(
        with: &<Foo as ShapeWith<K>>::Out,
    ) -> &<K as SlotFor<Foo, Self>>::Slot {
        &with.bar
    }

    fn slot_mut(
        with: &mut <Foo as ShapeWith<K>>::Out,
    ) -> &mut <K as SlotFor<Foo, Self>>::Slot {
        &mut with.bar
    }
}

impl<K> SlotOf<Foo, K> for foo_field::Zee
where
    K: SlotFor<Foo, foo_field::Bar> + SlotFor<Foo, foo_field::Zee>,
{
    fn slot(
        with: &<Foo as ShapeWith<K>>::Out,
    ) -> &<K as SlotFor<Foo, Self>>::Slot {
        &with.zee
    }

    fn slot_mut(
        with: &mut <Foo as ShapeWith<K>>::Out,
    ) -> &mut <K as SlotFor<Foo, Self>>::Slot {
        &mut with.zee
    }
}

// ---------------------------------------------------------------- per struct
// Only the traversals `partly` uses. The rest follows `fixtures/muck`.

impl<V> ForEach<V> for Foo
where
    V: EachField<Foo, foo_field::Bar> + EachField<Foo, foo_field::Zee>,
{
    fn for_each(visitor: &mut V) {
        EachField::<Foo, foo_field::Bar>::each(visitor);
        EachField::<Foo, foo_field::Zee>::each(visitor);
    }
}

impl<K, V> Make<K, V> for Foo
where
    K: SlotFor<Foo, foo_field::Bar> + SlotFor<Foo, foo_field::Zee>,
    V: MakeField<Foo, foo_field::Bar, K> + MakeField<Foo, foo_field::Zee, K>,
{
    fn make(maker: &mut V) -> Self::Out {
        FooShape {
            bar:    MakeField::<Foo, foo_field::Bar, K>::make(maker),
            zee:    MakeField::<Foo, foo_field::Zee, K>::make(maker),
            __kind: PhantomData,
        }
    }
}

// ----------------------------------------------------------------- per field

impl ReadField<Foo, foo_field::Bar> for Foo {
    fn read_field(&self) -> Option<&u32> {
        Some(&self.bar)
    }
}

impl ReadField<Foo, foo_field::Zee> for Foo {
    fn read_field(&self) -> Option<&String> {
        Some(&self.zee)
    }
}

impl<B: ReadSlot<u32>, Z, K> ReadField<Foo, foo_field::Bar>
    for FooShape<B, Z, K>
{
    fn read_field(&self) -> Option<&u32> {
        self.bar.read_slot()
    }
}

impl<B, Z: ReadSlot<String>, K> ReadField<Foo, foo_field::Zee>
    for FooShape<B, Z, K>
{
    fn read_field(&self) -> Option<&String> {
        self.zee.read_slot()
    }
}

impl SetField<Foo, foo_field::Bar> for Foo {
    fn set_field(&mut self, value: u32) {
        self.bar = value;
    }
}

impl SetField<Foo, foo_field::Zee> for Foo {
    fn set_field(&mut self, value: String) {
        self.zee = value;
    }
}

impl<B: WriteSlot<u32>, Z, K> SetField<Foo, foo_field::Bar>
    for FooShape<B, Z, K>
{
    fn set_field(&mut self, value: u32) {
        self.bar.write_slot(value);
    }
}

impl<B, Z: WriteSlot<String>, K> SetField<Foo, foo_field::Zee>
    for FooShape<B, Z, K>
{
    fn set_field(&mut self, value: String) {
        self.zee.write_slot(value);
    }
}

// ---------------------------------------------------------------- per struct

/// Writes the fields of [`Foo`] by name, to anything that accepts it: `Foo`,
/// a partial, or a patch being built.
pub trait FooSetters {
    fn set_bar(&mut self, value: u32)
    where
        Self: SetField<Foo, foo_field::Bar>,
    {
        SetField::<Foo, foo_field::Bar>::set_field(self, value);
    }

    fn set_zee(&mut self, value: String)
    where
        Self: SetField<Foo, foo_field::Zee>,
    {
        SetField::<Foo, foo_field::Zee>::set_field(self, value);
    }
}

impl<R: ?Sized> FooSetters for R {}

// ---------------------------------------------------------------- per struct
// The slot view, and `Debug` for every representation through one hook.

impl<'a> SlotType<'a, Foo, foo_field::Bar> for Foo {
    type Slot = &'a u32;
}

impl SlotView<Foo, foo_field::Bar> for Foo {
    fn slot_view(&self) -> &u32 {
        &self.bar
    }
}

impl<'a> SlotType<'a, Foo, foo_field::Zee> for Foo {
    type Slot = &'a String;
}

impl SlotView<Foo, foo_field::Zee> for Foo {
    fn slot_view(&self) -> &String {
        &self.zee
    }
}

impl<'a, B, Z, K> SlotType<'a, Foo, foo_field::Bar> for FooShape<B, Z, K> {
    type Slot = &'a B;
}

impl<B, Z, K> SlotView<Foo, foo_field::Bar> for FooShape<B, Z, K> {
    fn slot_view(&self) -> &B {
        &self.bar
    }
}

impl<'a, B, Z, K> SlotType<'a, Foo, foo_field::Zee> for FooShape<B, Z, K> {
    type Slot = &'a Z;
}

impl<B, Z, K> SlotView<Foo, foo_field::Zee> for FooShape<B, Z, K> {
    fn slot_view(&self) -> &Z {
        &self.zee
    }
}

/// Has a slot for every field of [`Foo`]: the slot view, by name.
pub trait FooSlots:
    SlotView<Foo, foo_field::Bar> + SlotView<Foo, foo_field::Zee>
{
    fn bar_slot(&self) -> foo_slot::Bar<'_, Self> {
        SlotView::<Foo, foo_field::Bar>::slot_view(self)
    }

    fn zee_slot(&self) -> foo_slot::Zee<'_, Self> {
        SlotView::<Foo, foo_field::Zee>::slot_view(self)
    }
}

impl<T> FooSlots for T where
    T: ?Sized + SlotView<Foo, foo_field::Bar> + SlotView<Foo, foo_field::Zee>
{
}

/// The type of each field's slot in a representation `V`.
pub mod foo_slot {
    use fieldmuck::SlotType;

    use super::{Foo, foo_field};

    /// The slot of `bar` in `V`.
    pub type Bar<'a, V> = <V as SlotType<'a, Foo, foo_field::Bar>>::Slot;
    /// The slot of `zee` in `V`.
    pub type Zee<'a, V> = <V as SlotType<'a, Foo, foo_field::Zee>>::Slot;
}

/// The default hook: prints every slot.
impl<V: FooSlots + Representation<Foo> + ?Sized> DebugShape<V> for Foo
where
    for<'a> foo_slot::Bar<'a, V>: Debug,
    for<'a> foo_slot::Zee<'a, V>: Debug,
{
    fn fmt_shape(
        view: &V,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct(&name_of::<Foo, V>())
            .field("bar", &view.bar_slot())
            .field("zee", &view.zee_slot())
            .finish()
    }
}

impl<B, Z, K> Debug for FooShape<B, Z, K>
where
    Foo: DebugShape<Self>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Foo as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ===================================================== #[derive(Partly)]

// ---------------------------------------------------------------- per struct
// Reflection, as before.

/// An owned field of [`Foo`] and its value.
///
/// These impls are written out because a plain derive would be a compile error
/// for a field type without the trait. See `bar.rs` for such a field.
pub enum FooFieldValue {
    Bar(u32),
    Zee(String),
}

impl std::fmt::Debug for FooFieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bar(value) => {
                f.debug_tuple("Bar")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
            Self::Zee(value) => {
                f.debug_tuple("Zee")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
        }
    }
}

impl Clone for FooFieldValue
where
    for<'x> u32: Clone,
    for<'x> String: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Bar(value) => Self::Bar(value.clone()),
            Self::Zee(value) => Self::Zee(value.clone()),
        }
    }
}

impl PartialEq for FooFieldValue
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bar(left), Self::Bar(right)) => left == right,
            (Self::Zee(left), Self::Zee(right)) => left == right,
            _ => false,
        }
    }
}

impl FooFieldValue {
    /// The field this value belongs to.
    #[must_use]
    pub const fn field(&self) -> FooFields {
        match self {
            Self::Bar(_) => FooFields::Bar,
            Self::Zee(_) => FooFields::Zee,
        }
    }
}

/// A borrowed field of [`Foo`] and its value.
#[derive(Clone, Copy)]
pub enum FooFieldRef<'a> {
    Bar(&'a u32),
    Zee(&'a String),
}

impl std::fmt::Debug for FooFieldRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bar(value) => {
                f.debug_tuple("Bar")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
            Self::Zee(value) => {
                f.debug_tuple("Zee")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
        }
    }
}

impl PartialEq for FooFieldRef<'_>
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bar(left), Self::Bar(right)) => left == right,
            (Self::Zee(left), Self::Zee(right)) => left == right,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------- per struct
// The partial is the `Optional` instance of the shape. Its slot parameters
// keep the typestate: `FooPartial<Set<u32>, Option<String>>`.

/// A [`Foo`] whose fields may or may not be set.
pub type FooPartial<B = Option<u32>, Z = Option<String>> =
    FooShape<B, Z, Optional>;

/// A partial with every field known to be set.
pub type FooCompletePartial = FooPartial<Set<u32>, Set<String>>;

impl FooPartial {
    /// Creates an empty partial.
    #[must_use]
    pub const fn new() -> Self {
        Self::from_slots(None, None)
    }
}

// ----------------------------------------------------------------- per field

impl<Z> FooPartial<Option<u32>, Z> {
    #[must_use]
    pub const fn bar(&self) -> Option<&u32> {
        self.bar.as_ref()
    }

    #[must_use]
    pub const fn bar_mut(&mut self) -> Option<&mut u32> {
        self.bar.as_mut()
    }

    #[must_use]
    pub const fn has_bar(&self) -> bool {
        self.bar.is_some()
    }

    pub fn set_bar(&mut self, value: u32) {
        self.bar = Some(value);
    }

    #[must_use]
    pub fn with_bar(mut self, value: u32) -> Self {
        self.bar = Some(value);
        self
    }

    pub const fn take_bar(&mut self) -> Option<u32> {
        self.bar.take()
    }

    pub fn clear_bar(&mut self) {
        self.bar = None;
    }

    /// Moves `bar` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if `bar` isn't set.
    pub fn require_bar(self) -> Result<FooPartial<Set<u32>, Z>, Self> {
        // Built directly: `self` already holds a valid `zee` slot, and
        // `Set<u32>` is a partial slot, so nothing needs checking again.
        match self.bar {
            Some(value) => {
                Ok(FooShape {
                    bar:    Set(value),
                    zee:    self.zee,
                    __kind: PhantomData,
                })
            },
            None => Err(self),
        }
    }
}

impl<Z> FooPartial<Set<u32>, Z> {
    #[must_use]
    pub const fn bar(&self) -> &u32 {
        &self.bar.0
    }
}

impl<B> FooPartial<B, Option<String>> {
    #[must_use]
    pub const fn zee(&self) -> Option<&String> {
        self.zee.as_ref()
    }

    #[must_use]
    pub const fn zee_mut(&mut self) -> Option<&mut String> {
        self.zee.as_mut()
    }

    #[must_use]
    pub const fn has_zee(&self) -> bool {
        self.zee.is_some()
    }

    pub fn set_zee(&mut self, value: String) {
        self.zee = Some(value);
    }

    #[must_use]
    pub fn with_zee(mut self, value: String) -> Self {
        self.zee = Some(value);
        self
    }

    pub const fn take_zee(&mut self) -> Option<String> {
        self.zee.take()
    }

    pub fn clear_zee(&mut self) {
        self.zee = None;
    }

    /// Moves `zee` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if `zee` isn't set.
    pub fn require_zee(self) -> Result<FooPartial<B, Set<String>>, Self> {
        match self.zee {
            Some(value) => {
                Ok(FooShape {
                    bar:    self.bar,
                    zee:    Set(value),
                    __kind: PhantomData,
                })
            },
            None => Err(self),
        }
    }
}

impl<B> FooPartial<B, Set<String>> {
    #[must_use]
    pub const fn zee(&self) -> &String {
        &self.zee.0
    }
}

// ---------------------------------------------------------------- per struct
// Conversions.

impl From<Foo> for FooPartial {
    fn from(value: Foo) -> Self {
        Self::from_slots(Some(value.bar), Some(value.zee))
    }
}

impl From<FooCompletePartial> for FooPartial {
    fn from(value: FooCompletePartial) -> Self {
        Self::from_slots(Some(value.bar.0), Some(value.zee.0))
    }
}

// ---------------------------------------------------------------- per struct
// The core traits.

impl Partly for Foo {
    type DefaultValidator = AcceptAll;
    type Field = FooFields;
    type Partial = FooPartial;
}

impl PartialOps for FooPartial {
    type Field = FooFields;
    type Target = Foo;

    fn has(&self, field: FooFields) -> bool {
        match field {
            FooFields::Bar => self.bar.is_some(),
            FooFields::Zee => self.zee.is_some(),
        }
    }

    fn merge(self, other: Self) -> Self {
        Self::from_slots(other.bar.or(self.bar), other.zee.or(self.zee))
    }
}

impl FromIterator<Self> for FooPartial {
    /// Folds partials together. Later partials win.
    fn from_iter<I: IntoIterator<Item = Self>>(iter: I) -> Self {
        iter.into_iter().fold(Self::new(), PartialOps::merge)
    }
}

// ---------------------------------------------------------------- per struct
// The build strategy, with one method per field.

/// Decides how each field of [`Foo`] gets its final value.
pub trait FooBuildStrategy {
    type Error: From<MissingField<FooFields>>;

    /// Returns `true` if the strategy expects to resolve every field.
    fn can_build(&self, partial: &FooPartial) -> bool {
        partial.is_complete()
    }

    /// Produces the final `bar`, given the value that was set.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_bar(&self, value: Option<u32>) -> Result<u32, Self::Error> {
        value.ok_or_else(|| MissingField(FooFields::Bar).into())
    }

    /// Produces the final `zee`, given the value that was set.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_zee(
        &self,
        value: Option<String>,
    ) -> Result<String, Self::Error> {
        value.ok_or_else(|| MissingField(FooFields::Zee).into())
    }
}

impl FooBuildStrategy for DefaultBuildStrategy {
    type Error = MissingField<FooFields>;
}

// ---------------------------------------------------------------- per struct
// The validator, with one method per field, connected to the generic
// `ValidateField` and `ValidateWhole` that `partly`'s generic code uses.

/// Decides whether values of [`Foo`] are valid.
pub trait FooValidator {
    type Error;

    /// Checks a value for `bar`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_bar(&self, _value: &u32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks a value for `zee`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_zee(&self, _value: &String) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks the whole struct, after every field has passed.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate(&self, _value: &Foo) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl FooValidator for AcceptAll {
    type Error = Infallible;
}

impl<V: FooValidator> ValidateField<Foo, V> for foo_field::Bar {
    type Error = V::Error;

    fn validate_field(validator: &V, value: &u32) -> Result<(), V::Error> {
        validator.validate_bar(value)
    }
}

impl<V: FooValidator> ValidateField<Foo, V> for foo_field::Zee {
    type Error = V::Error;

    fn validate_field(validator: &V, value: &String) -> Result<(), V::Error> {
        validator.validate_zee(value)
    }
}

impl<V: FooValidator> ValidateWhole<V> for Foo {
    type Error = V::Error;

    fn validate_whole(&self, validator: &V) -> Result<(), V::Error> {
        validator.validate(self)
    }
}

impl<V: FooValidator> ValidateWith<V> for Foo {
    type ApplyError = ValidationErrors<FooFields, V::Error>;
    type Error = V::Error;

    fn validate_with(
        &self,
        validator: &V,
    ) -> Result<(), ValidationErrors<FooFields, V::Error>> {
        let mut errors = Vec::new();
        if let Err(error) = validator.validate_bar(&self.bar) {
            errors.push(FieldError {
                field: FooFields::Bar,
                error,
            });
        }
        if let Err(error) = validator.validate_zee(&self.zee) {
            errors.push(FieldError {
                field: FooFields::Zee,
                error,
            });
        }
        if !errors.is_empty() {
            return Err(ValidationErrors::Fields(errors));
        }
        validator.validate(self).map_err(ValidationErrors::Struct)
    }

    fn validate_partial_with(
        partial: &FooPartial,
        validator: &V,
    ) -> Result<(), ValidationErrors<FooFields, V::Error>> {
        let mut errors = Vec::new();
        if let Some(Err(error)) = partial
            .bar
            .as_ref()
            .map(|value| validator.validate_bar(value))
        {
            errors.push(FieldError {
                field: FooFields::Bar,
                error,
            });
        }
        if let Some(Err(error)) = partial
            .zee
            .as_ref()
            .map(|value| validator.validate_zee(value))
        {
            errors.push(FieldError {
                field: FooFields::Zee,
                error,
            });
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors::Fields(errors))
        }
    }

    fn apply_partial_with(
        &mut self,
        mut input: FooPartial,
        validator: &V,
    ) -> Result<(), Rejected<FooPartial, ValidationErrors<FooFields, V::Error>>>
    {
        if let Err(error) = Self::validate_partial_with(&input, validator) {
            return Err(Rejected {
                value: input,
                error,
            });
        }
        // Swapping means a rollback needs no `Clone`: swap again.
        if let Some(value) = &mut input.bar {
            std::mem::swap(&mut self.bar, value);
        }
        if let Some(value) = &mut input.zee {
            std::mem::swap(&mut self.zee, value);
        }
        if let Err(error) = validator.validate(self) {
            if let Some(value) = &mut input.bar {
                std::mem::swap(&mut self.bar, value);
            }
            if let Some(value) = &mut input.zee {
                std::mem::swap(&mut self.zee, value);
            }
            return Err(Rejected {
                value: input,
                error: ValidationErrors::Struct(error),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------- per struct
// Building: resolve fields, then skipped fields, then validate.

impl<S: FooBuildStrategy, V: FooValidator> BuildWith<S, V> for FooPartial {
    type Error = BuildFailure<S::Error, ValidationErrors<FooFields, V::Error>>;
    type Output = Foo;

    fn build_with(
        mut self,
        strategy: &S,
        validator: &V,
    ) -> Result<Foo, Rejected<Self, Self::Error>> {
        let bar = match strategy.resolve_bar(self.bar.take()) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let zee = match strategy.resolve_zee(self.zee.take()) {
            Ok(value) => value,
            Err(error) => {
                self.bar = Some(bar);
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let built = Foo { bar, zee };
        match built.validate_with(validator) {
            Ok(()) => Ok(built),
            Err(error) => {
                Err(Rejected {
                    value: built.into(),
                    error: BuildFailure::Validation(error),
                })
            },
        }
    }
}

impl Build for FooPartial {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = Foo;

    fn can_build(&self) -> bool {
        // Fully qualified: other derives in this module implement their own
        // strategy trait for `DefaultBuildStrategy`.
        FooBuildStrategy::can_build(&DefaultBuildStrategy, self)
    }

    fn build(self) -> Result<Foo, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

// ---------------------------------------------------------------- per struct
// Reflection.

impl Reflect for FooPartial {
    type Value = FooFieldValue;
    type ValueRef<'a> = FooFieldRef<'a>;

    fn iter(&self) -> impl Iterator<Item = FooFieldRef<'_>> {
        [
            self.bar.as_ref().map(FooFieldRef::Bar),
            self.zee.as_ref().map(FooFieldRef::Zee),
        ]
        .into_iter()
        .flatten()
    }

    fn set_field(&mut self, value: FooFieldValue) -> Option<FooFieldValue> {
        match value {
            FooFieldValue::Bar(value) => {
                self.bar.replace(value).map(FooFieldValue::Bar)
            },
            FooFieldValue::Zee(value) => {
                self.zee.replace(value).map(FooFieldValue::Zee)
            },
        }
    }

    fn take_field(&mut self, field: FooFields) -> Option<FooFieldValue> {
        match field {
            FooFields::Bar => self.bar.take().map(FooFieldValue::Bar),
            FooFields::Zee => self.zee.take().map(FooFieldValue::Zee),
        }
    }
}

impl IntoIterator for FooPartial {
    type IntoIter =
        std::iter::Flatten<std::array::IntoIter<Option<FooFieldValue>, 2>>;
    type Item = FooFieldValue;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.bar.map(FooFieldValue::Bar),
            self.zee.map(FooFieldValue::Zee),
        ]
        .into_iter()
        .flatten()
    }
}

impl Extend<FooFieldValue> for FooPartial {
    fn extend<I: IntoIterator<Item = FooFieldValue>>(&mut self, iter: I) {
        for value in iter {
            Reflect::set_field(self, value);
        }
    }
}

impl FromIterator<FooFieldValue> for FooPartial {
    fn from_iter<I: IntoIterator<Item = FooFieldValue>>(iter: I) -> Self {
        let mut partial = Self::new();
        partial.extend(iter);
        partial
    }
}

// ---------------------------------------------------------------- per struct
// The patch is generic. The derive names it, picks the bound patch, and adds
// named field access.

/// Changes to a [`Foo`], made against the value of every patched field.
pub type FooPatch = ShapePatch<Foo>;

/// A [`FooPatch`] being built: writes are staged over a borrowed [`Foo`].
pub type BoundFooPatch<'a> = Staged<'a, Foo, ChangeRecorder<Foo>>;

// ----------------------------------------------------------------- per field
// Every field takes part in patches. A `#[partly(skip(patch))]` field would
// be `Skipped`, and need no `Clone` or `PartialEq`. The macro can't tell
// which types have them, so the bound is higher-ranked: for a type without
// them, the impl is absent instead of an error. See `opaque.rs`.

impl PatchField<Foo> for foo_field::Bar
where
    for<'x> u32: Clone + PartialEq,
{
    type Mode = Patched;
}

impl PatchField<Foo> for foo_field::Zee
where
    for<'x> String: Clone + PartialEq,
{
    type Mode = Patched;
}

impl partly::shape_patch::PatchShape for Foo {
    const FIELDS: &'static [FooFields] = FooFields::ALL;
}

/// The old and new value of each field of a [`Foo`] patch, by name.
pub trait FooPatchFields {
    /// The old and new value of `bar`, if it is written.
    fn bar(&self) -> Option<PatchValue<&u32>>;
    /// The old and new value of `zee`, if it is written.
    fn zee(&self) -> Option<PatchValue<&String>>;

    /// Returns `true` if `bar` is written.
    fn has_bar(&self) -> bool {
        self.bar().is_some()
    }

    /// Returns `true` if `zee` is written.
    fn has_zee(&self) -> bool {
        self.zee().is_some()
    }
}

impl FooPatchFields for FooPatch {
    fn bar(&self) -> Option<PatchValue<&u32>> {
        self.get::<foo_field::Bar>()
    }

    fn zee(&self) -> Option<PatchValue<&String>> {
        self.get::<foo_field::Zee>()
    }
}

impl FooPatchFields for BoundFooPatch<'_> {
    fn bar(&self) -> Option<PatchValue<&u32>> {
        self.buffer()
            .get::<foo_field::Bar>()
            .map(|new| PatchValue::new(&self.target().bar, new))
    }

    fn zee(&self) -> Option<PatchValue<&String>> {
        self.buffer()
            .get::<foo_field::Zee>()
            .map(|new| PatchValue::new(&self.target().zee, new))
    }
}

/// The named traits of [`Foo`], from both derives, for one `use`.
pub mod foo_prelude {
    pub use super::{FooPatchFields, FooSetters, FooSlots};
}

impl Patchable for Foo
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type Bound<'a> = BoundFooPatch<'a>;
    type DefaultMergeStrategy = StrictMerge;
    type Patch = FooPatch;

    fn patch(&self) -> BoundFooPatch<'_> {
        Staged::new(self, ChangeRecorder::new())
    }

    fn diff(&self, new: &Self) -> FooPatch {
        partly::shape_patch::diff(self, new)
    }
}

// ---------------------------------------------------------------- per struct
// The merge strategy, with one method per field, connected to the generic
// `MergeField`.

/// Settles conflicts when a [`FooPatch`] is applied.
///
/// A method only runs for a field whose value differs from the one the patch
/// was made against. `new` is the value the patch writes, or `None` if it
/// doesn't write the field.
pub trait FooMergeStrategy {
    type Error: From<Conflict<FooFields>>;

    /// Settles a conflict on `bar`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_bar(
        &self,
        _current: &u32,
        _old: &u32,
        _new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        Err(Conflict(FooFields::Bar).into())
    }

    /// Settles a conflict on `zee`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_zee(
        &self,
        _current: &String,
        _old: &String,
        _new: Option<&String>,
    ) -> Result<Resolution<String>, Self::Error> {
        Err(Conflict(FooFields::Zee).into())
    }
}

impl FooMergeStrategy for StrictMerge {
    type Error = Conflict<FooFields>;
}

impl FooMergeStrategy for FieldMerge {
    type Error = Conflict<FooFields>;

    fn resolve_bar(
        &self,
        _current: &u32,
        _old: &u32,
        new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => Err(Conflict(FooFields::Bar)),
        }
    }

    fn resolve_zee(
        &self,
        _current: &String,
        _old: &String,
        new: Option<&String>,
    ) -> Result<Resolution<String>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => Err(Conflict(FooFields::Zee)),
        }
    }
}

impl<M: FooMergeStrategy> MergeField<Foo, M> for foo_field::Bar {
    type Error = M::Error;

    fn resolve(
        strategy: &M,
        current: &u32,
        old: &u32,
        new: Option<&u32>,
    ) -> Result<Resolution<u32>, M::Error> {
        strategy.resolve_bar(current, old, new)
    }
}

impl<M: FooMergeStrategy> MergeField<Foo, M> for foo_field::Zee {
    type Error = M::Error;

    fn resolve(
        strategy: &M,
        current: &String,
        old: &String,
        new: Option<&String>,
    ) -> Result<Resolution<String>, M::Error> {
        strategy.resolve_zee(current, old, new)
    }
}

impl<M: FooMergeStrategy, V: FooValidator> ApplyPatchWith<M, V> for Foo
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type MergeError = M::Error;

    fn apply_patch_with(
        &mut self,
        patch: FooPatch,
        strategy: &M,
        validator: &V,
    ) -> Result<
        (),
        Rejected<
            FooPatch,
            ApplyFailure<M::Error, ValidationErrors<FooFields, V::Error>>,
        >,
    > {
        partly::shape_patch::apply::<_, _, _, M::Error, V::Error>(
            self, patch, strategy, validator,
        )
        .map(|_replaced| ())
    }

    fn apply_patch_with_get_undo(
        &mut self,
        patch: FooPatch,
        strategy: &M,
        validator: &V,
    ) -> Result<
        FooPatch,
        Rejected<
            FooPatch,
            ApplyFailure<M::Error, ValidationErrors<FooFields, V::Error>>,
        >,
    > {
        partly::shape_patch::apply_get_undo::<_, _, _, M::Error, V::Error>(
            self, patch, strategy, validator,
        )
    }
}
