//! TODO! documentation
//!
//! Everything in this file is the runtime part of the crate: code that stays
//! here once the derive macro exists. The code the macro will generate is
//! written by hand in the `fixtures` crate for now.
#![doc = document_features::document_features!()]
#![doc(issue_tracker_base_url = "todo!")]
// The README's code blocks run as doctests too.
#![cfg_attr(doctest, doc = include_str!("../README.md"))]
#![warn(rustdoc::broken_intra_doc_links)]

use std::error::Error;
use std::fmt::{self, Debug, Display};

/// The traits needed to call `partly` operations with method syntax.
pub mod prelude {
    #[cfg(feature = "patches")]
    pub use crate::{ApplyPatchWith, BoundPatch, PatchOps, Patchable};
    pub use crate::{
        Build, BuildError, BuildWith, PartialOps, Partly, Reflect, ValidateWith,
    };
}

/// The partial type of `T`.
pub type Partial<T> = <T as Partly>::Partial;

/// The error list produced by validator `V` on `T`.
pub type ValidationErrorsOf<T, V> =
    ValidationErrors<<T as Partly>::Field, <T as ValidateWith<V>>::Error>;

/// Typed slots, from `fieldmuck`: a partial's type can say a field is set
/// ([`Set`]) or not ([`Unset`]).
pub use fieldmuck::{Set, Unset};

/// Support code for macro output. Not part of the public API.
#[doc(hidden)]
pub mod __private {
    use std::fmt::{self, Debug};

    use crate::{Set, Unset};

    /// Reads the value out of any kind of slot.
    pub trait Slot<T> {
        fn peek(&self) -> Option<&T>;
    }

    impl<T> Slot<T> for Unset {
        fn peek(&self) -> Option<&T> {
            None
        }
    }

    impl<T> Slot<T> for Set<T> {
        fn peek(&self) -> Option<&T> {
            Some(&self.0)
        }
    }

    impl<T> Slot<T> for Option<T> {
        fn peek(&self) -> Option<&T> {
            self.as_ref()
        }
    }

    /// Prints in place of a value whose type isn't `Debug`.
    struct Opaque;

    impl Debug for Opaque {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("<opaque>")
        }
    }

    /// Picks how to print a value, using autoref specialization.
    ///
    /// `(&DebugProbe(value)).as_debug()` prefers [`ViaDebug`], which needs no
    /// extra reference, and only falls back to [`ViaOpaque`] if `T` isn't
    /// `Debug`. The choice is made where the call is written, so it only sees
    /// the field's real type if that type is concrete there, or a type
    /// parameter with a `Debug` bound.
    pub struct DebugProbe<'a, T>(pub &'a T);

    pub trait ViaDebug<'a> {
        fn as_debug(&self) -> &'a (dyn Debug + 'a);
    }

    impl<'a, T: Debug> ViaDebug<'a> for DebugProbe<'a, T> {
        fn as_debug(&self) -> &'a (dyn Debug + 'a) {
            self.0
        }
    }

    pub trait ViaOpaque<'a> {
        fn as_debug(&self) -> &'a (dyn Debug + 'a);
    }

    impl<'a, T> ViaOpaque<'a> for &DebugProbe<'a, T> {
        fn as_debug(&self) -> &'a (dyn Debug + 'a) {
            &Opaque
        }
    }

    /// Settles one field of a patch: the patch's own value if the target
    /// still holds `old`, and otherwise whatever `resolve` decides. `None`
    /// means nothing is written.
    ///
    /// # Errors
    ///
    /// Returns the merge strategy's error.
    #[cfg(feature = "patches")]
    pub fn settle<T: PartialEq, E>(
        current: &T,
        old: &T,
        new: Option<&T>,
        resolve: impl FnOnce(&T, &T, Option<&T>) -> Result<crate::Resolution<T>, E>,
    ) -> Result<Option<crate::Resolution<T>>, E> {
        if current == old {
            Ok(new.is_some().then_some(crate::Resolution::UseNew))
        } else {
            resolve(current, old, new).map(Some)
        }
    }
}

/// Gives a `&dyn Debug` for a `&T`, printing `<opaque>` if `T` isn't `Debug`.
#[doc(hidden)]
#[macro_export]
macro_rules! __debug_value {
    ($value:expr) => {{
        #[allow(unused_imports)]
        use $crate::__private::{ViaDebug as _, ViaOpaque as _};
        (&$crate::__private::DebugProbe($value)).as_debug()
    }};
}

/// Gives an `Option<&dyn Debug>` for a slot of any kind.
#[doc(hidden)]
#[macro_export]
macro_rules! __debug_slot {
    ($slot:expr) => {
        $crate::__private::Slot::peek($slot)
            .map(|value| $crate::__debug_value!(value))
    };
}

/// An enum naming the fields of a struct or enum. This is `fieldmuck`'s,
/// so a shape and its partial name fields with the same enum.
///
/// For an enum, this names the fields of every variant, prefixed with the
/// variant: `MultipleValuesBar`. Positional fields are named by their
/// index, as in `"0"`.
pub use fieldmuck::Field;

/// Validation of field `Self` of `S` by validator `V`.
///
/// The derive implements this on each field's type, for every `V` that
/// implements the type's named validator trait (`validate_bar`). Generic
/// code, such as [`ShapePatch`], validates through it.
pub trait ValidateField<S: fieldmuck::Shape, V>: fieldmuck::FieldOf<S> {
    /// The validator's error.
    type Error;

    /// Checks a value for the field.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_field(
        validator: &V,
        value: &Self::Type,
    ) -> Result<(), Self::Error>;
}

/// The whole-value check of validator `V`, which runs after every field has
/// passed. The derive implements this for every `V` of the named trait.
pub trait ValidateWhole<V> {
    /// The validator's error.
    type Error;

    /// Checks the whole value.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_whole(&self, validator: &V) -> Result<(), Self::Error>;
}

/// A field had no value and the build strategy did not supply one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MissingField<F>(pub F);

impl<F: Field> Display for MissingField<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "missing field `{}`", self.0.name())
    }
}

impl<F: Field> Error for MissingField<F> {}

/// An enum partial had no variant and the build strategy did not supply one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MissingVariant;

impl Display for MissingVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("missing variant")
    }
}

impl Error for MissingVariant {}

/// Why the default build strategy of an enum failed.
///
/// Structs use [`MissingField`] instead, since they can't be missing a variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissingValue<F> {
    /// No variant was chosen.
    Variant,
    /// The chosen variant is missing this field.
    Field(F),
}

impl<F> From<MissingVariant> for MissingValue<F> {
    fn from(MissingVariant: MissingVariant) -> Self {
        Self::Variant
    }
}

impl<F> From<MissingField<F>> for MissingValue<F> {
    fn from(MissingField(field): MissingField<F>) -> Self {
        Self::Field(field)
    }
}

impl<F: Field> Display for MissingValue<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Variant => Display::fmt(&MissingVariant, f),
            Self::Field(field) => Display::fmt(&MissingField(*field), f),
        }
    }
}

impl<F: Field> Error for MissingValue<F> {}

/// What generic code can ask of any build error.
///
/// Every error type is still concrete: a struct's build never returns a
/// "missing variant" that can't happen. Custom strategy errors don't have to
/// implement this.
pub trait BuildError {
    /// The fields of the type being built.
    type Field: Field;

    /// The field that had no value, if that is why the build failed.
    #[must_use]
    fn missing_field(&self) -> Option<Self::Field> {
        None
    }

    /// Returns `true` if the build failed because no variant was chosen.
    #[must_use]
    fn missing_variant(&self) -> bool {
        false
    }
}

impl<F: Field> BuildError for MissingField<F> {
    type Field = F;

    fn missing_field(&self) -> Option<F> {
        Some(self.0)
    }
}

impl<F: Field> BuildError for MissingValue<F> {
    type Field = F;

    fn missing_field(&self) -> Option<F> {
        match self {
            Self::Variant => None,
            Self::Field(field) => Some(*field),
        }
    }

    fn missing_variant(&self) -> bool {
        matches!(self, Self::Variant)
    }
}

/// A validation error for one field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError<F, E> {
    /// The field that failed.
    pub field: F,
    /// The validator's error.
    pub error: E,
}

/// What a validator reported.
///
/// Field validators all run, and every failure is collected. The whole-struct
/// check only runs once they have all passed, because it may rely on them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationErrors<F, E> {
    /// One or more field validators failed. Never empty.
    Fields(Vec<FieldError<F, E>>),
    /// The whole-struct check failed.
    Struct(E),
}

impl<F: Field, E: Display> Display for ValidationErrors<F, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fields(errors) => {
                for (index, item) in errors.iter().enumerate() {
                    if index > 0 {
                        f.write_str("; ")?;
                    }
                    write!(f, "`{}`: {}", item.field.name(), item.error)?;
                }
                Ok(())
            },
            Self::Struct(error) => Display::fmt(error, f),
        }
    }
}

impl<F: Field, E: Debug + Display> Error for ValidationErrors<F, E> {}

/// An operation failed, and this is the value it was given.
///
/// [`BuildWith::build_with`] can return a changed value: fields that were
/// already resolved come back in their resolved form, and the value passed to
/// a strategy method that failed is lost. Use [`Build::build_cloned`] to keep
/// the original untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected<T, E> {
    /// The value handed back to the caller.
    pub value: T,
    /// Why the operation failed.
    pub error: E,
}

impl<T, E> Rejected<T, E> {
    /// Converts the handed-back value, keeping the error.
    #[must_use]
    pub fn map_value<U>(self, convert: impl FnOnce(T) -> U) -> Rejected<U, E> {
        Rejected {
            value: convert(self.value),
            error: self.error,
        }
    }
}

impl<T, E: Display> Display for Rejected<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl<T: Debug, E: Error> Error for Rejected<T, E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

/// Why a build failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildFailure<S, V> {
    /// The build strategy could not resolve a field.
    Strategy(S),
    /// The resolved values were rejected by the validator.
    Validation(V),
}

impl<S: Display, V: Display> Display for BuildFailure<S, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Strategy(error) => Display::fmt(error, f),
            Self::Validation(error) => Display::fmt(error, f),
        }
    }
}

impl<S: Debug + Display, V: Debug + Display> Error for BuildFailure<S, V> {}

/// A failed validation is not a missing value, so only the strategy's error is
/// asked.
impl<S: BuildError, V> BuildError for BuildFailure<S, V> {
    type Field = S::Field;

    fn missing_field(&self) -> Option<S::Field> {
        match self {
            Self::Strategy(error) => error.missing_field(),
            Self::Validation(_) => None,
        }
    }

    fn missing_variant(&self) -> bool {
        match self {
            Self::Strategy(error) => error.missing_variant(),
            Self::Validation(_) => false,
        }
    }
}

impl<T, E: BuildError> BuildError for Rejected<T, E> {
    type Field = E::Field;

    fn missing_field(&self) -> Option<E::Field> {
        self.error.missing_field()
    }

    fn missing_variant(&self) -> bool {
        self.error.missing_variant()
    }
}

/// A build strategy that only accepts values that are already set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DefaultBuildStrategy;

/// A validator that accepts every value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AcceptAll;

/// Implemented for structs and enums that have companion types.
pub trait Partly: Sized {
    /// The enum naming this type's fields.
    type Field: Field;
    /// The dynamic partial of this type.
    type Partial: PartialOps<Target = Self, Field = Self::Field> + From<Self>;
    /// The validator used when none is given.
    type DefaultValidator: Default;

    /// Creates an empty partial.
    #[must_use]
    fn partial() -> Self::Partial {
        Self::Partial::default()
    }

    /// Converts into a partial with every field set. Skipped fields are
    /// dropped.
    #[must_use]
    fn into_partial(self) -> Self::Partial {
        self.into()
    }

    /// Copies into a partial with every field set.
    #[must_use]
    fn to_partial(&self) -> Self::Partial
    where
        Self: Clone,
    {
        self.clone().into()
    }

    /// Runs the default validator on this value.
    ///
    /// # Errors
    ///
    /// Returns everything the validator rejected.
    fn validate(
        &self,
    ) -> Result<(), ValidationErrorsOf<Self, Self::DefaultValidator>>
    where
        Self: ValidateWith<<Self as Partly>::DefaultValidator>,
    {
        self.validate_with(&Self::DefaultValidator::default())
    }

    /// Writes every field set on `input` into `self`, using the default
    /// validator.
    ///
    /// # Errors
    ///
    /// If this fails, `self` is unchanged and `input` is handed back. See
    /// [`ValidateWith::ApplyError`].
    #[expect(clippy::type_complexity, reason = "the bound is the signature")]
    fn apply_partial(
        &mut self,
        input: impl Into<Self::Partial>,
    ) -> Result<
        (),
        Rejected<
            Self::Partial,
            <Self as ValidateWith<Self::DefaultValidator>>::ApplyError,
        >,
    >
    where
        Self: ValidateWith<<Self as Partly>::DefaultValidator>,
    {
        self.apply_partial_with(
            input.into(),
            &Self::DefaultValidator::default(),
        )
    }
}

/// Validation of `Self` by validator `V`.
///
/// The derive implements this for every `V` that implements the struct's
/// generated validator trait.
pub trait ValidateWith<V>: Partly {
    /// The error type of the validator.
    type Error;
    /// Why [`apply_partial_with`](ValidateWith::apply_partial_with) can fail.
    ///
    /// For a struct, this is [`ValidationErrorsOf<Self, V>`]. For an enum, a
    /// partial for another variant replaces the whole value, so it can also be
    /// missing a field: this is a [`BuildFailure`] of [`MissingField`] and the
    /// validation errors.
    type ApplyError;

    /// Runs the field validators and then the whole-struct check.
    ///
    /// # Errors
    ///
    /// Returns everything the validator rejected.
    fn validate_with(
        &self,
        validator: &V,
    ) -> Result<(), ValidationErrorsOf<Self, V>>;

    /// Runs the field validators for the fields set on `partial`.
    ///
    /// # Errors
    ///
    /// Returns every field that was rejected.
    fn validate_partial_with(
        partial: &Self::Partial,
        validator: &V,
    ) -> Result<(), ValidationErrorsOf<Self, V>>;

    /// Writes every field set on `input` into `self`, using `validator`.
    ///
    /// # Errors
    ///
    /// If this fails, `self` is unchanged and `input` is handed back.
    fn apply_partial_with(
        &mut self,
        input: Self::Partial,
        validator: &V,
    ) -> Result<(), Rejected<Self::Partial, Self::ApplyError>>;
}

/// Operations shared by all dynamic partials.
///
/// This covers a type's partial, [`Partly::Partial`], and also the partial of
/// one enum variant, which builds the enum but can't be made from any value of
/// it.
pub trait PartialOps: Sized + Default {
    /// The type this partial builds.
    type Target: Partly;
    /// The enum naming the fields.
    type Field: Field;

    /// Returns `true` if `field` is set.
    #[must_use]
    fn has(&self, field: Self::Field) -> bool;

    /// The fields that must be set for the partial to be complete, in
    /// declaration order.
    ///
    /// For a struct, this is every field. For an enum, it is the fields of the
    /// chosen variant.
    #[must_use]
    fn required_fields(&self) -> &'static [Self::Field] {
        <Self::Field as Field>::ALL
    }

    /// Combines two partials. Where both set a field, `other` wins.
    #[must_use]
    fn merge(self, other: Self) -> Self;

    /// Combines two partials. Where both set a field, `self` wins.
    #[must_use]
    fn or(self, other: Self) -> Self {
        other.merge(self)
    }

    /// Returns `true` if nothing is set. An enum partial with a chosen variant
    /// is not empty.
    #[must_use]
    fn is_empty(&self) -> bool {
        self.set_fields().next().is_none()
    }

    /// Returns `true` if every required field is set. Skipped fields don't
    /// count. An enum partial also needs a chosen variant.
    #[must_use]
    fn is_complete(&self) -> bool {
        self.missing_fields().next().is_none()
    }

    /// The required fields that have a value, in declaration order.
    fn set_fields(&self) -> impl Iterator<Item = Self::Field> {
        self.required_fields()
            .iter()
            .copied()
            .filter(|field| self.has(*field))
    }

    /// The required fields that have no value, in declaration order.
    fn missing_fields(&self) -> impl Iterator<Item = Self::Field> {
        self.required_fields()
            .iter()
            .copied()
            .filter(|field| !self.has(*field))
    }

    /// Builds with `strategy` and the default validator.
    ///
    /// # Errors
    ///
    /// See [`BuildWith::build_with`].
    #[expect(clippy::type_complexity, reason = "the bound is the signature")]
    fn build_with_strategy<S>(
        self,
        strategy: &S,
    ) -> Result<
        Self::Target,
        Rejected<
            Self,
            <Self as BuildWith<
                S,
                <Self::Target as Partly>::DefaultValidator,
            >>::Error,
        >,
    >
    where
        Self: BuildWith<
                S,
                <Self::Target as Partly>::DefaultValidator,
                Output = Self::Target,
            >,
    {
        self.build_with(strategy, &Default::default())
    }
}

/// Building with the struct's default strategy and validator.
///
/// The derive implements this unless told not to, so that it can be written by
/// hand.
pub trait Build: Sized {
    /// The type being built.
    type Output;
    /// Why a build can fail.
    type Error;

    /// Returns `true` if the default strategy expects [`build`](Build::build)
    /// to resolve every field. Validation can still fail.
    #[must_use]
    fn can_build(&self) -> bool;

    /// Builds the value, consuming the partial.
    ///
    /// # Errors
    ///
    /// Hands back the partial, which can be changed. See [`Rejected`].
    fn build(self) -> Result<Self::Output, Rejected<Self, Self::Error>>;

    /// Builds the value, leaving the partial untouched.
    ///
    /// # Errors
    ///
    /// Returns why the build failed.
    fn build_cloned(&self) -> Result<Self::Output, Self::Error>
    where
        Self: Clone,
    {
        self.clone().build().map_err(|rejected| rejected.error)
    }

    /// Builds the value, taking any missing field from `fallback`.
    ///
    /// The result still goes through the default strategy and validator.
    ///
    /// # Errors
    ///
    /// Hands back the partial with the fallback's values filled in.
    fn build_or(
        self,
        fallback: Self::Output,
    ) -> Result<Self::Output, Rejected<Self, Self::Error>>
    where
        Self: PartialOps<Target = Self::Output> + From<Self::Output>,
    {
        self.or(fallback.into()).build()
    }

    /// Like [`build_or`](Build::build_or), but `fallback` only runs if a field
    /// is missing.
    ///
    /// # Errors
    ///
    /// Hands back the partial with any fallback values filled in.
    fn build_or_else(
        self,
        fallback: impl FnOnce() -> Self::Output,
    ) -> Result<Self::Output, Rejected<Self, Self::Error>>
    where
        Self: PartialOps<Target = Self::Output> + From<Self::Output>,
    {
        if self.is_complete() {
            self.build()
        } else {
            self.build_or(fallback())
        }
    }
}

/// Building with a chosen strategy `S` and validator `V`.
///
/// The derive implements this for every `S` and `V` that implement the struct's
/// generated traits.
pub trait BuildWith<S, V>: Sized {
    /// The type being built.
    type Output;
    /// Why a build can fail.
    type Error;

    /// Builds the value, consuming the partial.
    ///
    /// # Errors
    ///
    /// Hands back the partial, which can be changed. See [`Rejected`].
    fn build_with(
        self,
        strategy: &S,
        validator: &V,
    ) -> Result<Self::Output, Rejected<Self, Self::Error>>;

    /// Builds the value, leaving the partial untouched.
    ///
    /// # Errors
    ///
    /// Returns why the build failed.
    fn build_cloned_with(
        &self,
        strategy: &S,
        validator: &V,
    ) -> Result<Self::Output, Self::Error>
    where
        Self: Clone,
    {
        self.clone()
            .build_with(strategy, validator)
            .map_err(|rejected| rejected.error)
    }
}

/// Field-by-field access to a dynamic partial, for code that doesn't know the
/// field names.
pub trait Reflect:
    PartialOps + IntoIterator<Item = Self::Value> + Extend<Self::Value>
{
    /// An owned field and its value.
    type Value;
    /// A borrowed field and its value.
    type ValueRef<'a>
    where
        Self: 'a;

    /// The fields that are set and their values, in declaration order.
    fn iter(&self) -> impl Iterator<Item = Self::ValueRef<'_>>;

    /// Sets a field, returning the value it replaced.
    fn set_field(&mut self, value: Self::Value) -> Option<Self::Value>;

    /// Unsets a field, returning its value.
    fn take_field(&mut self, field: Self::Field) -> Option<Self::Value>;
}

#[cfg(feature = "patches")]
pub use patches::*;

#[cfg(feature = "patches")]
pub mod shape_patch;
#[cfg(feature = "patches")]
pub use shape_patch::{
    ChangeRecorder, FieldChange, MergeField, PatchBase, PatchChanges,
    PatchData, PatchField, PatchMode, Patched, ShapePatch, Skip, Skipped,
};

#[cfg(feature = "patches")]
mod patches {
    use std::error::Error;
    use std::fmt::{self, Debug, Display};

    use super::{
        Field, MissingValue, Partly, Rejected, ValidateWith, ValidationErrorsOf,
    };

    /// The patch type of `T`.
    pub type Patch<T> = <T as Patchable>::Patch;

    /// The old and new value of one field.
    ///
    /// Patch accessors return this with references, as `PatchValue<&T>`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PatchValue<T> {
        /// The value the field had when the patch was made.
        pub old_value: T,
        /// The value to write.
        pub new_value: T,
    }

    impl<T> PatchValue<T> {
        /// Creates a change.
        #[must_use]
        pub const fn new(old_value: T, new_value: T) -> Self {
            Self {
                old_value,
                new_value,
            }
        }

        /// Swaps the old and new values, which undoes the change.
        #[must_use]
        pub fn invert(self) -> Self {
            Self {
                old_value: self.new_value,
                new_value: self.old_value,
            }
        }
    }

    impl<T: PartialEq> PatchValue<T> {
        /// Returns `true` if the change doesn't change anything.
        #[must_use]
        pub fn is_noop(&self) -> bool {
            self.old_value == self.new_value
        }
    }

    impl<T> From<(T, T)> for PatchValue<T> {
        fn from((old_value, new_value): (T, T)) -> Self {
            Self {
                old_value,
                new_value,
            }
        }
    }

    /// A struct's field doesn't match the value a patch was made against, or
    /// two patches disagree about it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Conflict<F>(pub F);

    impl<F: Field> Display for Conflict<F> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "conflict on field `{}`", self.0.name())
        }
    }

    impl<F: Field> Error for Conflict<F> {}

    /// Like [`Conflict`], for an enum, which can also conflict on its variant.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum EnumConflict<F> {
        /// The variants differ.
        Variant,
        /// The variants are the same, and this field differs.
        Field(F),
    }

    impl<F> From<Conflict<F>> for EnumConflict<F> {
        fn from(Conflict(field): Conflict<F>) -> Self {
            Self::Field(field)
        }
    }

    impl<F: Field> Display for EnumConflict<F> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Variant => f.write_str("conflict on the variant"),
                Self::Field(field) => Display::fmt(&Conflict(*field), f),
            }
        }
    }

    impl<F: Field> Error for EnumConflict<F> {}

    /// What generic code can ask of any patch conflict.
    pub trait PatchConflict {
        /// The fields of the type being patched.
        type Field: Field;

        /// The field the conflict is on, if it is on one field.
        #[must_use]
        fn field(&self) -> Option<Self::Field>;

        /// Returns `true` if the conflict is on the variant of an enum.
        #[must_use]
        fn is_variant(&self) -> bool {
            false
        }
    }

    impl<F: Field> PatchConflict for Conflict<F> {
        type Field = F;

        fn field(&self) -> Option<F> {
            Some(self.0)
        }
    }

    impl<F: Field> PatchConflict for EnumConflict<F> {
        type Field = F;

        fn field(&self) -> Option<F> {
            match self {
                Self::Variant => None,
                Self::Field(field) => Some(*field),
            }
        }

        fn is_variant(&self) -> bool {
            matches!(self, Self::Variant)
        }
    }

    /// Why the built-in merge strategies failed on an enum patch.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum EnumMergeError<F> {
        /// A conflict the strategy doesn't settle.
        Conflict(EnumConflict<F>),
        /// The patch writes another variant, and the value to write couldn't
        /// be built from it.
        Build(MissingValue<F>),
    }

    impl<F> From<EnumConflict<F>> for EnumMergeError<F> {
        fn from(conflict: EnumConflict<F>) -> Self {
            Self::Conflict(conflict)
        }
    }

    impl<F> From<MissingValue<F>> for EnumMergeError<F> {
        fn from(missing: MissingValue<F>) -> Self {
            Self::Build(missing)
        }
    }

    impl<F: Field> Display for EnumMergeError<F> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Conflict(conflict) => Display::fmt(conflict, f),
                Self::Build(missing) => Display::fmt(missing, f),
            }
        }
    }

    impl<F: Field> Error for EnumMergeError<F> {}

    /// Why an enum patch conflicts as a whole, given to the merge strategy.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum VariantConflict<F> {
        /// The target is another variant than the patch was made against.
        OtherVariant,
        /// The patch changes the variant, and these fields of the target
        /// differ from the value it was made against. Never empty.
        Fields(Vec<F>),
    }

    /// Why [`PatchOps::then`] failed.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ChainFailure<C, B> {
        /// The second patch doesn't start from what the first one leaves.
        Conflict(C),
        /// Finding what the first patch leaves needed a build, which failed.
        /// For a struct, `B` is [`Infallible`](std::convert::Infallible), so
        /// a `match` doesn't need this arm.
        Build(B),
    }

    impl<C: Display, B: Display> Display for ChainFailure<C, B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Conflict(error) => Display::fmt(error, f),
                Self::Build(error) => Display::fmt(error, f),
            }
        }
    }

    impl<C: Debug + Display, B: Debug + Display> Error for ChainFailure<C, B> {}

    /// Why applying a patch failed.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ApplyFailure<M, V> {
        /// The merge strategy did not resolve a conflict.
        Merge(M),
        /// The new values were rejected by the validator.
        Validation(V),
    }

    impl<M: Display, V: Display> Display for ApplyFailure<M, V> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Merge(error) => Display::fmt(error, f),
                Self::Validation(error) => Display::fmt(error, f),
            }
        }
    }

    impl<M: Debug + Display, V: Debug + Display> Error for ApplyFailure<M, V> {}

    /// How a merge strategy settles a conflict.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Resolution<T> {
        /// Leave the value as it is.
        KeepCurrent,
        /// Write the patch's new value. If the patch doesn't write this field,
        /// this leaves it as it is.
        UseNew,
        /// Write this value instead.
        Use(T),
    }

    impl<T> Resolution<T> {
        /// The value that will be written, given the patch's new value.
        #[must_use]
        pub fn chosen<'a>(&'a self, new: Option<&'a T>) -> Option<&'a T> {
            match self {
                Self::KeepCurrent => None,
                Self::UseNew => new,
                Self::Use(value) => Some(value),
            }
        }

        /// Swaps the chosen value with `current`. Calling this a second time
        /// undoes it, so no field type needs `Clone` to roll back.
        #[doc(hidden)]
        pub fn __swap(&mut self, current: &mut T, new: Option<&mut T>) {
            match (self, new) {
                (Self::UseNew, Some(new)) => std::mem::swap(current, new),
                (Self::Use(value), _) => std::mem::swap(current, value),
                _ => {},
            }
        }

        /// After [`__swap`](Self::__swap), the value that was replaced, if
        /// any.
        #[doc(hidden)]
        pub fn __previous(self, new: Option<T>) -> Option<T> {
            match self {
                Self::KeepCurrent => None,
                Self::UseNew => new,
                Self::Use(value) => Some(value),
            }
        }
    }

    /// A merge strategy that rejects every conflict.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct StrictMerge;

    /// A merge strategy that keeps changes made since the patch to fields the
    /// patch doesn't write, and rejects every other conflict.
    ///
    /// Use it to apply two patches made from the same value one after the
    /// other, when they change different fields.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct FieldMerge;

    /// Implemented for structs and enums that have a patch type.
    ///
    /// A patch holds the complete value it was made against, and a partial of
    /// the values it writes. It only applies to a target equal to that value;
    /// any difference is a conflict for the merge strategy.
    ///
    /// The derive only implements this if every patched field is
    /// `Clone + PartialEq`.
    pub trait Patchable: Partly {
        /// The patch of this type.
        type Patch: PatchOps<Target = Self>;
        /// The patch being built against a borrowed value.
        type Bound<'a>: BoundPatch<'a, Target = Self>
        where
            Self: 'a;
        /// The merge strategy used when none is given.
        type DefaultMergeStrategy: Default;

        /// Starts a patch against `self`.
        #[must_use]
        fn patch(&self) -> Self::Bound<'_>;

        /// Creates a patch against `self` that writes what differs in `new`.
        /// For an enum, a different variant writes all of `new`.
        #[must_use]
        fn diff(&self, new: &Self) -> Self::Patch;

        /// Builds a patch against `self` inside a closure.
        #[must_use]
        fn edit(
            &self,
            change: impl FnOnce(&mut Self::Bound<'_>),
        ) -> Self::Patch {
            let mut bound = self.patch();
            change(&mut bound);
            bound.finish()
        }

        /// Applies a patch with the default merge strategy and validator.
        ///
        /// # Errors
        ///
        /// If this fails, `self` is unchanged and the patch is handed back.
        #[expect(
            clippy::type_complexity,
            reason = "the bound is the signature"
        )]
        fn apply_patch(
            &mut self,
            patch: Self::Patch,
        ) -> Result<
            (),
            Rejected<
                Self::Patch,
                ApplyFailure<
                    <Self as ApplyPatchWith<
                        Self::DefaultMergeStrategy,
                        Self::DefaultValidator,
                    >>::MergeError,
                    ValidationErrorsOf<Self, Self::DefaultValidator>,
                >,
            >,
        >
        where
            Self: ApplyPatchWith<
                    <Self as Patchable>::DefaultMergeStrategy,
                    <Self as Partly>::DefaultValidator,
                >,
        {
            self.apply_patch_with(
                patch,
                &Default::default(),
                &Default::default(),
            )
        }

        /// Like [`apply_patch`](Patchable::apply_patch), and returns a patch
        /// that undoes exactly what was written.
        ///
        /// # Errors
        ///
        /// If this fails, `self` is unchanged and the patch is handed back.
        #[expect(
            clippy::type_complexity,
            reason = "the bound is the signature"
        )]
        fn apply_patch_get_undo(
            &mut self,
            patch: Self::Patch,
        ) -> Result<
            Self::Patch,
            Rejected<
                Self::Patch,
                ApplyFailure<
                    <Self as ApplyPatchWith<
                        Self::DefaultMergeStrategy,
                        Self::DefaultValidator,
                    >>::MergeError,
                    ValidationErrorsOf<Self, Self::DefaultValidator>,
                >,
            >,
        >
        where
            Self: ApplyPatchWith<
                    <Self as Patchable>::DefaultMergeStrategy,
                    <Self as Partly>::DefaultValidator,
                >,
        {
            self.apply_patch_with_get_undo(
                patch,
                &Default::default(),
                &Default::default(),
            )
        }

        /// Applies a patch with `strategy` and the default validator.
        ///
        /// # Errors
        ///
        /// If this fails, `self` is unchanged and the patch is handed back.
        #[expect(
            clippy::type_complexity,
            reason = "the bound is the signature"
        )]
        fn apply_patch_with_strategy<M>(
            &mut self,
            patch: Self::Patch,
            strategy: &M,
        ) -> Result<
            (),
            Rejected<
                Self::Patch,
                ApplyFailure<
                    <Self as ApplyPatchWith<M, Self::DefaultValidator>>::MergeError,
                    ValidationErrorsOf<Self, Self::DefaultValidator>,
                >,
            >,
        >
        where
            Self: ApplyPatchWith<M, <Self as Partly>::DefaultValidator>,
        {
            self.apply_patch_with(patch, strategy, &Default::default())
        }
    }

    /// Applying a patch with merge strategy `M` and validator `V`.
    ///
    /// The derive implements this for every `M` and `V` that implement the
    /// struct's generated traits.
    pub trait ApplyPatchWith<M, V>: Patchable + ValidateWith<V> {
        /// The error type of the merge strategy.
        type MergeError;

        /// Compares the target with the value the patch was made against,
        /// lets the merge strategy settle every difference, validates, and
        /// then writes.
        ///
        /// # Errors
        ///
        /// If this fails, `self` is unchanged and the patch is handed back.
        #[expect(
            clippy::type_complexity,
            reason = "the bound is the signature"
        )]
        fn apply_patch_with(
            &mut self,
            patch: Self::Patch,
            strategy: &M,
            validator: &V,
        ) -> Result<
            (),
            Rejected<
                Self::Patch,
                ApplyFailure<Self::MergeError, ValidationErrorsOf<Self, V>>,
            >,
        >;

        /// Like [`apply_patch_with`](ApplyPatchWith::apply_patch_with), and
        /// returns a patch that undoes exactly what was written, including
        /// values the merge strategy chose.
        ///
        /// # Errors
        ///
        /// If this fails, `self` is unchanged and the patch is handed back.
        #[expect(
            clippy::type_complexity,
            reason = "the bound is the signature"
        )]
        fn apply_patch_with_get_undo(
            &mut self,
            patch: Self::Patch,
            strategy: &M,
            validator: &V,
        ) -> Result<
            Self::Patch,
            Rejected<
                Self::Patch,
                ApplyFailure<Self::MergeError, ValidationErrorsOf<Self, V>>,
            >,
        >;
    }

    /// Operations shared by all dynamic patches.
    ///
    /// A patch is either empty, or holds the value it was made against and a
    /// partial of the values it writes.
    pub trait PatchOps: Sized + Default {
        /// The type this patches.
        type Target: Partly;
        /// What a conflict names: [`Conflict`] for a struct, [`EnumConflict`]
        /// for an enum.
        type Conflict;
        /// Why [`invert`](PatchOps::invert) or [`then`](PatchOps::then) can
        /// fail to build a value. [`Infallible`](std::convert::Infallible)
        /// for a struct.
        type BuildError;

        /// The fields this patch can write, in declaration order. Fields
        /// marked `skip(patch)` are not included. For an enum, these are the
        /// fields of the variant it writes.
        #[must_use]
        fn fields(&self) -> &'static [<Self::Target as Partly>::Field];

        /// Returns `true` if the patch writes `field`.
        #[must_use]
        fn has(&self, field: <Self::Target as Partly>::Field) -> bool;

        /// Returns `true` if every value the patch writes equals the one it
        /// replaces.
        #[must_use]
        fn is_noop(&self) -> bool;

        /// Lists how `target` differs from the value the patch was made
        /// against.
        ///
        /// # Errors
        ///
        /// Returns every conflict.
        fn check(
            &self,
            target: &Self::Target,
        ) -> Result<(), Vec<Self::Conflict>>;

        /// Gives the patch that undoes this one, made against the value this
        /// one leaves.
        ///
        /// # Errors
        ///
        /// For an enum patch that writes another variant, the value it leaves
        /// is built with the default strategy, which can fail. Use
        /// [`Patchable::apply_patch_get_undo`] for an exact undo.
        fn invert(self) -> Result<Self, Rejected<Self, Self::BuildError>>;

        /// Keeps only the values to write.
        #[must_use]
        fn into_partial(self) -> <Self::Target as Partly>::Partial;

        /// Combines patches that apply one after the other. `next` must be
        /// made against exactly the value `self` leaves.
        ///
        /// # Errors
        ///
        /// Hands back both patches if `next` doesn't continue from `self`, or
        /// if finding the value `self` leaves needs a build that fails.
        #[expect(
            clippy::type_complexity,
            reason = "the bound is the signature"
        )]
        fn then(
            self,
            next: Self,
        ) -> Result<
            Self,
            Rejected<
                (Self, Self),
                ChainFailure<Self::Conflict, Self::BuildError>,
            >,
        >;

        /// Combines patches made against the same value.
        ///
        /// # Errors
        ///
        /// Hands back both patches if they were made against different
        /// values, or write a field or variant differently.
        fn merge(
            self,
            other: Self,
        ) -> Result<Self, Rejected<(Self, Self), Self::Conflict>>;

        /// Returns `true` if the patch writes nothing.
        #[must_use]
        fn is_empty(&self) -> bool {
            !self.fields().iter().any(|field| self.has(*field))
        }

        /// Returns `true` if the patch writes every field it can.
        #[must_use]
        fn is_complete(&self) -> bool {
            self.fields().iter().all(|field| self.has(*field))
        }
    }

    /// A patch being built against a borrowed value.
    pub trait BoundPatch<'a> {
        /// The type this patches.
        type Target: Patchable;

        /// Ends the borrow, leaving an owned patch made against a copy of the
        /// value.
        #[must_use]
        fn finish(self) -> <Self::Target as Patchable>::Patch;
    }
}
