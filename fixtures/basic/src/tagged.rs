//! A struct with a lifetime and a type parameter.
//!
//! Only the blocks that differ from `foo.rs`, or that the partial needs, are
//! written out.
//!
//! ```ignore
//! #[derive(Muck, Partly)]
//! pub struct Tagged<'a, T> {
//!     pub label: &'a str,
//!     pub value: T,
//! }
//! ```
//!
//! The shape carries the struct's type parameters, first. Without them, an
//! alias couldn't name them: `TaggedPartial<'a, T>` has `T` only in the
//! defaults of its slot parameters, and a type alias must use every type
//! parameter in its body. Lifetimes don't need this, so the slots carry them.

use std::convert::Infallible;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use fieldmuck::{
    Accepts, DebugShape, FieldOf, Kind, Optional, Representation, Shape,
    SlotType, SlotView, name_of,
};
use partly::prelude::*;
use partly::{
    AcceptAll, BuildFailure, DefaultBuildStrategy, Field, MissingField,
    Rejected, ValidationErrors,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tagged<'a, T> {
    pub label: &'a str,
    pub value: T,
}

// ======================================================= #[derive(Muck)]

// ---------------------------------------------------------------- per struct

/// Names a field of [`Tagged`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaggedFields {
    Label,
    Value,
}

impl Field for TaggedFields {
    const ALL: &'static [Self] = &[Self::Label, Self::Value];

    fn name(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::Value => "value",
        }
    }
}

/// The fields of [`Tagged`], as types. They carry no generics: the field of
/// `Tagged<'a, T>` for every `'a` and `T`.
pub mod tagged_field {
    /// The field `label`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Label;
    /// The field `value`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Value;
}

/// [`Tagged`] with a slot of any type in place of each field, of kind `K`.
/// `T` is the struct's own parameter, held only as a marker.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaggedShape<T, L, V, K = ()> {
    pub label: L,
    pub value: V,
    __kind:    PhantomData<(K, fn() -> T)>,
}

impl<T, L, V, K> TaggedShape<T, L, V, K> {
    /// Creates the companion from its slots.
    pub const fn from_slots<'a>(label: L, value: V) -> Self
    where
        K: Accepts<&'a str, L> + Accepts<T, V>,
    {
        Self {
            label,
            value,
            __kind: PhantomData,
        }
    }
}

impl<'a, T, L: Default, V: Default, K> Default for TaggedShape<T, L, V, K>
where
    K: Accepts<&'a str, L> + Accepts<T, V>,
{
    fn default() -> Self {
        Self::from_slots(L::default(), V::default())
    }
}

impl<T> Shape for Tagged<'_, T> {
    type Fields = TaggedFields;

    const NAME: &'static str = "Tagged";
}

impl<'a, T, L, V, K: Kind> Representation<Tagged<'a, T>>
    for TaggedShape<T, L, V, K>
{
    const KIND: &'static str = K::NAME;
}

// ----------------------------------------------------------------- per field

impl<'a, T> FieldOf<Tagged<'a, T>> for tagged_field::Label {
    type Type = &'a str;

    const FIELD: TaggedFields = TaggedFields::Label;

    fn get<'v>(value: &'v Tagged<'a, T>) -> &'v &'a str {
        &value.label
    }

    fn get_mut<'v>(value: &'v mut Tagged<'a, T>) -> &'v mut &'a str {
        &mut value.label
    }
}

impl<T> FieldOf<Tagged<'_, T>> for tagged_field::Value {
    type Type = T;

    const FIELD: TaggedFields = TaggedFields::Value;

    fn get<'v>(value: &'v Tagged<'_, T>) -> &'v T {
        &value.value
    }

    fn get_mut<'v>(value: &'v mut Tagged<'_, T>) -> &'v mut T {
        &mut value.value
    }
}

impl<'v, 'a, T, L, V, K> SlotType<'v, Tagged<'a, T>, tagged_field::Label>
    for TaggedShape<T, L, V, K>
{
    type Slot = &'v L;
}

impl<'a, T, L, V, K> SlotView<Tagged<'a, T>, tagged_field::Label>
    for TaggedShape<T, L, V, K>
{
    fn slot_view(&self) -> &L {
        &self.label
    }
}

impl<'v, 'a, T, L, V, K> SlotType<'v, Tagged<'a, T>, tagged_field::Value>
    for TaggedShape<T, L, V, K>
{
    type Slot = &'v V;
}

impl<'a, T, L, V, K> SlotView<Tagged<'a, T>, tagged_field::Value>
    for TaggedShape<T, L, V, K>
{
    fn slot_view(&self) -> &V {
        &self.value
    }
}

// ---------------------------------------------------------------- per struct

/// Has a slot for every field of [`Tagged`]: the slot view, by name.
pub trait TaggedSlots<'a, T>:
    SlotView<Tagged<'a, T>, tagged_field::Label>
    + SlotView<Tagged<'a, T>, tagged_field::Value>
{
    fn label_slot(&self) -> tagged_slot::Label<'_, 'a, T, Self> {
        SlotView::<Tagged<'a, T>, tagged_field::Label>::slot_view(self)
    }

    fn value_slot(&self) -> tagged_slot::Value<'_, 'a, T, Self> {
        SlotView::<Tagged<'a, T>, tagged_field::Value>::slot_view(self)
    }
}

impl<'a, T, R> TaggedSlots<'a, T> for R where
    R: ?Sized
        + SlotView<Tagged<'a, T>, tagged_field::Label>
        + SlotView<Tagged<'a, T>, tagged_field::Value>
{
}

/// The type of each field's slot in a representation `V` of `Tagged<'a, T>`.
pub mod tagged_slot {
    use fieldmuck::SlotType;

    use super::{Tagged, tagged_field};

    /// The slot of `label`.
    pub type Label<'v, 'a, T, V> =
        <V as SlotType<'v, Tagged<'a, T>, tagged_field::Label>>::Slot;
    /// The slot of `value`.
    pub type Value<'v, 'a, T, V> =
        <V as SlotType<'v, Tagged<'a, T>, tagged_field::Value>>::Slot;
}

/// The default hook: prints every slot. A partial's `value` slot is an
/// `Option<T>`, so a partial prints only if `T` does.
impl<'a, T, V> DebugShape<V> for Tagged<'a, T>
where
    V: TaggedSlots<'a, T> + Representation<Tagged<'a, T>> + ?Sized,
    for<'v> tagged_slot::Label<'v, 'a, T, V>: Debug,
    for<'v> tagged_slot::Value<'v, 'a, T, V>: Debug,
{
    fn fmt_shape(view: &V, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&name_of::<Tagged<'a, T>, V>())
            .field("label", &view.label_slot())
            .field("value", &view.value_slot())
            .finish()
    }
}

impl<'a, T, L, V, K> Debug for TaggedShape<T, L, V, K>
where
    Tagged<'a, T>: DebugShape<Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Tagged<'a, T> as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ===================================================== #[derive(Partly)]

// ---------------------------------------------------------------- per struct
// The user's generics come first, because slot generics have defaults.

/// A [`Tagged`] whose fields may or may not be set.
pub type TaggedPartial<'a, T, L = Option<&'a str>, V = Option<T>> =
    TaggedShape<T, L, V, Optional>;

impl<'a, T, V> TaggedPartial<'a, T, Option<&'a str>, V> {
    #[must_use]
    pub const fn label(&self) -> Option<&&'a str> {
        self.label.as_ref()
    }

    pub fn set_label(&mut self, value: &'a str) {
        self.label = Some(value);
    }
}

impl<T, L> TaggedPartial<'_, T, L, Option<T>> {
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn set_value(&mut self, value: T) {
        self.value = Some(value);
    }
}

impl<'a, T> From<Tagged<'a, T>> for TaggedPartial<'a, T> {
    fn from(value: Tagged<'a, T>) -> Self {
        Self::from_slots(Some(value.label), Some(value.value))
    }
}

impl<'a, T> Partly for Tagged<'a, T> {
    type DefaultValidator = AcceptAll;
    type Field = TaggedFields;
    type Partial = TaggedPartial<'a, T>;
}

impl<'a, T> PartialOps for TaggedPartial<'a, T> {
    type Field = TaggedFields;
    type Target = Tagged<'a, T>;

    fn has(&self, field: TaggedFields) -> bool {
        match field {
            TaggedFields::Label => self.label.is_some(),
            TaggedFields::Value => self.value.is_some(),
        }
    }

    fn merge(self, other: Self) -> Self {
        Self::from_slots(other.label.or(self.label), other.value.or(self.value))
    }
}

pub trait TaggedBuildStrategy<'a, T> {
    type Error: From<MissingField<TaggedFields>>;

    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_label(
        &self,
        value: Option<&'a str>,
    ) -> Result<&'a str, Self::Error> {
        value.ok_or_else(|| MissingField(TaggedFields::Label).into())
    }

    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_value(&self, value: Option<T>) -> Result<T, Self::Error> {
        value.ok_or_else(|| MissingField(TaggedFields::Value).into())
    }
}

impl<T> TaggedBuildStrategy<'_, T> for DefaultBuildStrategy {
    type Error = MissingField<TaggedFields>;
}

pub trait TaggedValidator<'a, T> {
    type Error;

    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate(&self, _value: &Tagged<'a, T>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T> TaggedValidator<'_, T> for AcceptAll {
    type Error = Infallible;
}

impl<'a, T, V: TaggedValidator<'a, T>> ValidateWith<V> for Tagged<'a, T> {
    type ApplyError = ValidationErrors<TaggedFields, V::Error>;
    type Error = V::Error;

    fn validate_with(
        &self,
        validator: &V,
    ) -> Result<(), ValidationErrors<TaggedFields, V::Error>> {
        validator.validate(self).map_err(ValidationErrors::Struct)
    }

    fn validate_partial_with(
        _partial: &TaggedPartial<'a, T>,
        _validator: &V,
    ) -> Result<(), ValidationErrors<TaggedFields, V::Error>> {
        Ok(())
    }

    fn apply_partial_with(
        &mut self,
        mut input: TaggedPartial<'a, T>,
        validator: &V,
    ) -> Result<
        (),
        Rejected<
            TaggedPartial<'a, T>,
            ValidationErrors<TaggedFields, V::Error>,
        >,
    > {
        if let Some(value) = &mut input.label {
            std::mem::swap(&mut self.label, value);
        }
        if let Some(value) = &mut input.value {
            std::mem::swap(&mut self.value, value);
        }
        if let Err(error) = validator.validate(self) {
            if let Some(value) = &mut input.label {
                std::mem::swap(&mut self.label, value);
            }
            if let Some(value) = &mut input.value {
                std::mem::swap(&mut self.value, value);
            }
            return Err(Rejected {
                value: input,
                error: ValidationErrors::Struct(error),
            });
        }
        Ok(())
    }
}

impl<'a, T, S: TaggedBuildStrategy<'a, T>, V: TaggedValidator<'a, T>>
    BuildWith<S, V> for TaggedPartial<'a, T>
{
    type Error =
        BuildFailure<S::Error, ValidationErrors<TaggedFields, V::Error>>;
    type Output = Tagged<'a, T>;

    fn build_with(
        mut self,
        strategy: &S,
        validator: &V,
    ) -> Result<Tagged<'a, T>, Rejected<Self, Self::Error>> {
        let label = match strategy.resolve_label(self.label.take()) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let value = match strategy.resolve_value(self.value.take()) {
            Ok(value) => value,
            Err(error) => {
                self.label = Some(label);
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let tagged = Tagged { label, value };
        match tagged.validate_with(validator) {
            Ok(()) => Ok(tagged),
            Err(error) => {
                Err(Rejected {
                    value: tagged.into(),
                    error: BuildFailure::Validation(error),
                })
            },
        }
    }
}

impl<'a, T> Build for TaggedPartial<'a, T> {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = Tagged<'a, T>;

    fn can_build(&self) -> bool {
        self.is_complete()
    }

    fn build(self) -> Result<Tagged<'a, T>, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}
