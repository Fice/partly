//! Field names that clash with operations, or that are Rust keywords.
//!
//! Only the blocks that differ from `foo.rs`, or that the partial needs, are
//! written out.
//!
//! ```ignore
//! #[derive(Muck, Partly)]
//! pub struct Keywords {
//!     // Without the rename, the macro reports a compile error on this field.
//!     #[muck(rename = "build_count")]
//!     pub build: u32,
//!     pub r#type: String,
//! }
//! ```
//!
//! The rename is a `muck` attribute, so every library's named methods agree:
//! `set_build_count`, `build_count_slot`. It applies to method names only.
//! The field enum, the field type and `name()` keep the field's real name.

use std::convert::Infallible;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use fieldmuck::{
    Accepts, DebugShape, FieldOf, Kind, Optional, Representation, Shape,
    SlotType, SlotView, name_of,
};
use partly::prelude::*;
use partly::{
    AcceptAll, BuildFailure, Field, MissingField, Rejected, ValidationErrors,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keywords {
    pub build:  u32,
    pub r#type: String,
}

// ======================================================= #[derive(Muck)]

// ---------------------------------------------------------------- per struct

/// Names a field of [`Keywords`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordsFields {
    Build,
    Type,
}

impl Field for KeywordsFields {
    const ALL: &'static [Self] = &[Self::Build, Self::Type];

    fn name(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Type => "type",
        }
    }
}

/// The fields of [`Keywords`], as types. Type names are never keywords.
pub mod keywords_field {
    /// The field `build`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Build;
    /// The field `type`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Type;
}

/// [`Keywords`] with a slot of any type in place of each field, of kind `K`.
/// The slots keep the fields' names, raw identifier included.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeywordsShape<B, T, K = ()> {
    pub build: B,
    pub r#type: T,
    __kind: PhantomData<K>,
}

impl<B, T, K> KeywordsShape<B, T, K> {
    /// Creates the companion from its slots.
    pub const fn from_slots(build: B, r#type: T) -> Self
    where
        K: Accepts<u32, B> + Accepts<String, T>,
    {
        Self {
            build,
            r#type,
            __kind: PhantomData,
        }
    }
}

impl<B: Default, T: Default, K> Default for KeywordsShape<B, T, K>
where
    K: Accepts<u32, B> + Accepts<String, T>,
{
    fn default() -> Self {
        Self::from_slots(B::default(), T::default())
    }
}

impl Shape for Keywords {
    type Fields = KeywordsFields;

    const NAME: &'static str = "Keywords";
}

impl<B, T, K: Kind> Representation<Keywords> for KeywordsShape<B, T, K> {
    const KIND: &'static str = K::NAME;
}

// ----------------------------------------------------------------- per field

impl FieldOf<Keywords> for keywords_field::Build {
    type Type = u32;

    const FIELD: KeywordsFields = KeywordsFields::Build;

    fn get(value: &Keywords) -> &u32 {
        &value.build
    }

    fn get_mut(value: &mut Keywords) -> &mut u32 {
        &mut value.build
    }
}

impl FieldOf<Keywords> for keywords_field::Type {
    type Type = String;

    const FIELD: KeywordsFields = KeywordsFields::Type;

    fn get(value: &Keywords) -> &String {
        &value.r#type
    }

    fn get_mut(value: &mut Keywords) -> &mut String {
        &mut value.r#type
    }
}

impl<'a, B, T, K> SlotType<'a, Keywords, keywords_field::Build>
    for KeywordsShape<B, T, K>
{
    type Slot = &'a B;
}

impl<B, T, K> SlotView<Keywords, keywords_field::Build>
    for KeywordsShape<B, T, K>
{
    fn slot_view(&self) -> &B {
        &self.build
    }
}

impl<'a, B, T, K> SlotType<'a, Keywords, keywords_field::Type>
    for KeywordsShape<B, T, K>
{
    type Slot = &'a T;
}

impl<B, T, K> SlotView<Keywords, keywords_field::Type>
    for KeywordsShape<B, T, K>
{
    fn slot_view(&self) -> &T {
        &self.r#type
    }
}

// ---------------------------------------------------------------- per struct
// Method names use the rename, and drop the raw prefix once something is
// added to the name.

/// Has a slot for every field of [`Keywords`]: the slot view, by name.
pub trait KeywordsSlots:
    SlotView<Keywords, keywords_field::Build>
    + SlotView<Keywords, keywords_field::Type>
{
    fn build_count_slot(&self) -> keywords_slot::Build<'_, Self> {
        SlotView::<Keywords, keywords_field::Build>::slot_view(self)
    }

    fn type_slot(&self) -> keywords_slot::Type<'_, Self> {
        SlotView::<Keywords, keywords_field::Type>::slot_view(self)
    }
}

impl<T> KeywordsSlots for T where
    T: ?Sized
        + SlotView<Keywords, keywords_field::Build>
        + SlotView<Keywords, keywords_field::Type>
{
}

/// The type of each field's slot in a representation `V`.
pub mod keywords_slot {
    use fieldmuck::SlotType;

    use super::{Keywords, keywords_field};

    /// The slot of `build` in `V`.
    pub type Build<'a, V> =
        <V as SlotType<'a, Keywords, keywords_field::Build>>::Slot;
    /// The slot of `type` in `V`.
    pub type Type<'a, V> =
        <V as SlotType<'a, Keywords, keywords_field::Type>>::Slot;
}

/// The default hook: prints every slot, under the field's real name.
impl<V: KeywordsSlots + Representation<Keywords> + ?Sized> DebugShape<V>
    for Keywords
where
    for<'a> keywords_slot::Build<'a, V>: Debug,
    for<'a> keywords_slot::Type<'a, V>: Debug,
{
    fn fmt_shape(view: &V, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&name_of::<Keywords, V>())
            .field("build", &view.build_count_slot())
            .field("type", &view.type_slot())
            .finish()
    }
}

impl<B, T, K> Debug for KeywordsShape<B, T, K>
where
    Keywords: DebugShape<Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Keywords as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ===================================================== #[derive(Partly)]

/// A [`Keywords`] whose fields may or may not be set.
pub type KeywordsPartial<B = Option<u32>, T = Option<String>> =
    KeywordsShape<B, T, Optional>;

impl<T> KeywordsPartial<Option<u32>, T> {
    #[must_use]
    pub const fn build_count(&self) -> Option<&u32> {
        self.build.as_ref()
    }

    pub fn set_build_count(&mut self, value: u32) {
        self.build = Some(value);
    }
}

// The getter stays a raw identifier. Anything with a prefix doesn't need it.
impl<B> KeywordsPartial<B, Option<String>> {
    #[must_use]
    pub const fn r#type(&self) -> Option<&String> {
        self.r#type.as_ref()
    }

    pub fn set_type(&mut self, value: String) {
        self.r#type = Some(value);
    }
}

impl From<Keywords> for KeywordsPartial {
    fn from(value: Keywords) -> Self {
        Self::from_slots(Some(value.build), Some(value.r#type))
    }
}

impl Partly for Keywords {
    type DefaultValidator = AcceptAll;
    type Field = KeywordsFields;
    type Partial = KeywordsPartial;
}

impl PartialOps for KeywordsPartial {
    type Field = KeywordsFields;
    type Target = Keywords;

    fn has(&self, field: KeywordsFields) -> bool {
        match field {
            KeywordsFields::Build => self.build.is_some(),
            KeywordsFields::Type => self.r#type.is_some(),
        }
    }

    fn merge(self, other: Self) -> Self {
        Self::from_slots(
            other.build.or(self.build),
            other.r#type.or(self.r#type),
        )
    }
}

pub trait KeywordsValidator {
    type Error;

    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate(&self, _value: &Keywords) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl KeywordsValidator for AcceptAll {
    type Error = Infallible;
}

impl<V: KeywordsValidator> ValidateWith<V> for Keywords {
    type ApplyError = ValidationErrors<KeywordsFields, V::Error>;
    type Error = V::Error;

    fn validate_with(
        &self,
        validator: &V,
    ) -> Result<(), ValidationErrors<KeywordsFields, V::Error>> {
        validator.validate(self).map_err(ValidationErrors::Struct)
    }

    fn validate_partial_with(
        _partial: &KeywordsPartial,
        _validator: &V,
    ) -> Result<(), ValidationErrors<KeywordsFields, V::Error>> {
        Ok(())
    }

    fn apply_partial_with(
        &mut self,
        input: KeywordsPartial,
        _validator: &V,
    ) -> Result<
        (),
        Rejected<KeywordsPartial, ValidationErrors<KeywordsFields, V::Error>>,
    > {
        if let Some(value) = input.build {
            self.build = value;
        }
        if let Some(value) = input.r#type {
            self.r#type = value;
        }
        Ok(())
    }
}

impl Build for KeywordsPartial {
    type Error = BuildFailure<
        MissingField<KeywordsFields>,
        ValidationErrors<KeywordsFields, Infallible>,
    >;
    type Output = Keywords;

    fn can_build(&self) -> bool {
        self.is_complete()
    }

    fn build(self) -> Result<Keywords, Rejected<Self, Self::Error>> {
        match (self.build, self.r#type) {
            (Some(build), Some(r#type)) => Ok(Keywords { build, r#type }),
            (build, r#type) => {
                let field = if build.is_none() {
                    KeywordsFields::Build
                } else {
                    KeywordsFields::Type
                };
                Err(Rejected {
                    value: Self::from_slots(build, r#type),
                    error: BuildFailure::Strategy(MissingField(field)),
                })
            },
        }
    }
}
