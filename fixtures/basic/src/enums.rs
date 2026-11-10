//! The dynamic expansion for enums: partials, building, validation,
//! reflection and patches.
//!
//! ```ignore
//! #[derive(Partly)]
//! pub enum EnumStructTest {
//!     NoValue,
//!     SingleValue { foo: u32 },
//!     MultipleValues { bar: u32, zee: String },
//! }
//!
//! #[derive(Partly)]
//! pub enum EnumTupleTest {
//!     NoValue,
//!     SingleValue(u32),
//!     MultipleValues(u32, u32, String),
//! }
//! ```
//!
//! Each variant gets its own partial, which works like a struct partial and
//! builds the enum. `…PartialState` holds one of them, and `…Partial` wraps an
//! `Option` of it, where `None` means no variant is chosen yet.
//!
//! In typed mode, you work on one variant partial directly, so the state enum
//! never needs type parameters.
//!
//! Reflection only exists on `…Partial`. Setting a field of another variant
//! switches to that variant, like the variant methods do, so the result of
//! `Extend` and `FromIterator` depends on the order of the values.
//!
//! A patch holds the complete value it was made against and a partial of what
//! it writes. A partial of the same variant writes its fields, like a struct
//! patch. A partial of another variant replaces the whole value with one the
//! merge strategy builds from it, by default with the default build strategy.

// The expansion binds fields by name, and `foo` is a placeholder name.
#![expect(clippy::disallowed_names)]

use std::convert::Infallible;

use partly::prelude::*;
use partly::{
    AcceptAll, ApplyFailure, BuildFailure, ChainFailure, DefaultBuildStrategy,
    EnumConflict, EnumMergeError, Field, FieldError, FieldMerge, MissingField,
    MissingValue, MissingVariant, Rejected, Resolution, Set, StrictMerge,
    ValidationErrors, VariantConflict,
};

/// The largest number of fields in one variant of [`EnumStructTest`], which is
/// how many values a partial can hold at once.
const ENUM_STRUCT_TEST_MAX_FIELDS: usize = 2;

/// The same for [`EnumTupleTest`].
const ENUM_TUPLE_TEST_MAX_FIELDS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumStructTest {
    NoValue,
    SingleValue { foo: u32 },
    MultipleValues { bar: u32, zee: String },
}

// ------------------------------------------------------------------ per enum
// A variant per enum variant.

/// Names a variant of [`EnumStructTest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnumStructTestVariants {
    NoValue,
    SingleValue,
    MultipleValues,
}

impl EnumStructTestVariants {
    /// Every variant, in declaration order.
    pub const ALL: &'static [Self] =
        &[Self::NoValue, Self::SingleValue, Self::MultipleValues];

    /// The name of the variant as written in the enum.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoValue => "NoValue",
            Self::SingleValue => "SingleValue",
            Self::MultipleValues => "MultipleValues",
        }
    }

    /// The fields of this variant, in declaration order.
    #[must_use]
    pub const fn fields(self) -> &'static [EnumStructTestFields] {
        match self {
            Self::NoValue => &[],
            Self::SingleValue => &[EnumStructTestFields::SingleValueFoo],
            Self::MultipleValues => {
                &[
                    EnumStructTestFields::MultipleValuesBar,
                    EnumStructTestFields::MultipleValuesZee,
                ]
            },
        }
    }
}

impl From<&EnumStructTest> for EnumStructTestVariants {
    fn from(value: &EnumStructTest) -> Self {
        match value {
            EnumStructTest::NoValue => Self::NoValue,
            EnumStructTest::SingleValue { .. } => Self::SingleValue,
            EnumStructTest::MultipleValues { .. } => Self::MultipleValues,
        }
    }
}

// ------------------------------------------------------------------ per enum
// A variant per field of every variant, prefixed with the variant.

/// Names a field of [`EnumStructTest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnumStructTestFields {
    SingleValueFoo,
    MultipleValuesBar,
    MultipleValuesZee,
}

impl Field for EnumStructTestFields {
    const ALL: &'static [Self] = &[
        Self::SingleValueFoo,
        Self::MultipleValuesBar,
        Self::MultipleValuesZee,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::SingleValueFoo => "foo",
            Self::MultipleValuesBar => "bar",
            Self::MultipleValuesZee => "zee",
        }
    }
}

impl EnumStructTestFields {
    /// The variant this field belongs to.
    #[must_use]
    pub const fn variant(self) -> EnumStructTestVariants {
        match self {
            Self::SingleValueFoo => EnumStructTestVariants::SingleValue,
            Self::MultipleValuesBar | Self::MultipleValuesZee => {
                EnumStructTestVariants::MultipleValues
            },
        }
    }
}

// --------------------------------------------------------------- per variant
// A unit variant's partial has no fields, so it is always complete.

/// The partial of [`EnumStructTest::NoValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EnumStructTestNoValuePartial;

impl EnumStructTestNoValuePartial {
    /// Creates the partial.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

// --------------------------------------------------------------- per variant
// A variant with fields gets everything a struct partial has.

/// The partial of [`EnumStructTest::SingleValue`].
#[derive(Clone, PartialEq, Eq)]
pub struct EnumStructTestSingleValuePartial<F = Option<u32>> {
    foo: F,
}

impl<F> std::fmt::Debug for EnumStructTestSingleValuePartial<F>
where
    F: partly::__private::Slot<u32>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnumStructTestSingleValuePartial")
            .field("foo", &partly::__debug_slot!(&self.foo))
            .finish()
    }
}

/// A partial with every field known to be set.
pub type EnumStructTestSingleValueCompletePartial =
    EnumStructTestSingleValuePartial<Set<u32>>;

impl EnumStructTestSingleValuePartial {
    /// Creates an empty partial.
    #[must_use]
    pub const fn new() -> Self {
        Self { foo: None }
    }
}

impl Default for EnumStructTestSingleValuePartial {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------- per field

impl EnumStructTestSingleValuePartial<Option<u32>> {
    #[must_use]
    pub const fn foo(&self) -> Option<&u32> {
        self.foo.as_ref()
    }

    #[must_use]
    pub const fn foo_mut(&mut self) -> Option<&mut u32> {
        self.foo.as_mut()
    }

    #[must_use]
    pub const fn has_foo(&self) -> bool {
        self.foo.is_some()
    }

    pub fn set_foo(&mut self, value: u32) {
        self.foo = Some(value);
    }

    #[must_use]
    pub fn with_foo(mut self, value: u32) -> Self {
        self.foo = Some(value);
        self
    }

    pub const fn take_foo(&mut self) -> Option<u32> {
        self.foo.take()
    }

    pub fn clear_foo(&mut self) {
        self.foo = None;
    }

    /// Moves `foo` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if `foo` isn't set.
    pub fn require_foo(
        self,
    ) -> Result<EnumStructTestSingleValuePartial<Set<u32>>, Self> {
        match self.foo {
            Some(value) => {
                Ok(EnumStructTestSingleValuePartial { foo: Set(value) })
            },
            None => Err(self),
        }
    }
}

impl EnumStructTestSingleValuePartial<Set<u32>> {
    #[must_use]
    pub const fn foo(&self) -> &u32 {
        &self.foo.0
    }
}

impl From<EnumStructTestSingleValueCompletePartial>
    for EnumStructTestSingleValuePartial
{
    fn from(value: EnumStructTestSingleValueCompletePartial) -> Self {
        Self {
            foo: Some(value.foo.0),
        }
    }
}

// --------------------------------------------------------------- per variant

/// The partial of [`EnumStructTest::MultipleValues`].
#[derive(Clone, PartialEq, Eq)]
pub struct EnumStructTestMultipleValuesPartial<
    B = Option<u32>,
    Z = Option<String>,
> {
    bar: B,
    zee: Z,
}

impl<B, Z> std::fmt::Debug for EnumStructTestMultipleValuesPartial<B, Z>
where
    B: partly::__private::Slot<u32>,
    Z: partly::__private::Slot<String>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnumStructTestMultipleValuesPartial")
            .field("bar", &partly::__debug_slot!(&self.bar))
            .field("zee", &partly::__debug_slot!(&self.zee))
            .finish()
    }
}

/// A partial with every field known to be set.
pub type EnumStructTestMultipleValuesCompletePartial =
    EnumStructTestMultipleValuesPartial<Set<u32>, Set<String>>;

impl EnumStructTestMultipleValuesPartial {
    /// Creates an empty partial.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bar: None,
            zee: None,
        }
    }
}

impl Default for EnumStructTestMultipleValuesPartial {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------- per field

impl<Z> EnumStructTestMultipleValuesPartial<Option<u32>, Z> {
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
    pub fn require_bar(
        self,
    ) -> Result<EnumStructTestMultipleValuesPartial<Set<u32>, Z>, Self> {
        match self.bar {
            Some(value) => {
                Ok(EnumStructTestMultipleValuesPartial {
                    bar: Set(value),
                    zee: self.zee,
                })
            },
            None => Err(self),
        }
    }
}

impl<Z> EnumStructTestMultipleValuesPartial<Set<u32>, Z> {
    #[must_use]
    pub const fn bar(&self) -> &u32 {
        &self.bar.0
    }
}

impl<B> EnumStructTestMultipleValuesPartial<B, Option<String>> {
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
    pub fn require_zee(
        self,
    ) -> Result<EnumStructTestMultipleValuesPartial<B, Set<String>>, Self> {
        match self.zee {
            Some(value) => {
                Ok(EnumStructTestMultipleValuesPartial {
                    bar: self.bar,
                    zee: Set(value),
                })
            },
            None => Err(self),
        }
    }
}

impl<B> EnumStructTestMultipleValuesPartial<B, Set<String>> {
    #[must_use]
    pub const fn zee(&self) -> &String {
        &self.zee.0
    }
}

impl From<EnumStructTestMultipleValuesCompletePartial>
    for EnumStructTestMultipleValuesPartial
{
    fn from(value: EnumStructTestMultipleValuesCompletePartial) -> Self {
        Self {
            bar: Some(value.bar.0),
            zee: Some(value.zee.0),
        }
    }
}

// ------------------------------------------------------------------ per enum
// The chosen variant. It holds dynamic variant partials only, so it needs no
// type parameters. The variant partials are `Clone` only if their fields are,
// so these impls are written out with higher-ranked bounds instead of derived.

/// The chosen variant of an [`EnumStructTestPartial`], and its partial.
#[derive(Debug)]
pub enum EnumStructTestPartialState {
    NoValue(EnumStructTestNoValuePartial),
    SingleValue(EnumStructTestSingleValuePartial),
    MultipleValues(EnumStructTestMultipleValuesPartial),
}

impl Clone for EnumStructTestPartialState
where
    for<'x> EnumStructTestSingleValuePartial: Clone,
    for<'x> EnumStructTestMultipleValuesPartial: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::NoValue(partial) => Self::NoValue(partial.clone()),
            Self::SingleValue(partial) => Self::SingleValue(partial.clone()),
            Self::MultipleValues(partial) => {
                Self::MultipleValues(partial.clone())
            },
        }
    }
}

impl PartialEq for EnumStructTestPartialState
where
    for<'x> EnumStructTestSingleValuePartial: PartialEq,
    for<'x> EnumStructTestMultipleValuesPartial: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NoValue(left), Self::NoValue(right)) => left == right,
            (Self::SingleValue(left), Self::SingleValue(right)) => {
                left == right
            },
            (Self::MultipleValues(left), Self::MultipleValues(right)) => {
                left == right
            },
            _ => false,
        }
    }
}

impl Eq for EnumStructTestPartialState
where
    for<'x> EnumStructTestSingleValuePartial: Eq,
    for<'x> EnumStructTestMultipleValuesPartial: Eq,
{
}

impl EnumStructTestPartialState {
    /// The variant this state is for.
    #[must_use]
    pub const fn variant(&self) -> EnumStructTestVariants {
        match self {
            Self::NoValue(_) => EnumStructTestVariants::NoValue,
            Self::SingleValue(_) => EnumStructTestVariants::SingleValue,
            Self::MultipleValues(_) => EnumStructTestVariants::MultipleValues,
        }
    }
}

// ------------------------------------------------------------------ per enum

/// An [`EnumStructTest`] whose variant may not be chosen yet, and whose fields
/// may or may not be set.
#[derive(Debug)]
pub struct EnumStructTestPartial {
    state: Option<EnumStructTestPartialState>,
}

impl Clone for EnumStructTestPartial
where
    for<'x> EnumStructTestPartialState: Clone,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl PartialEq for EnumStructTestPartial
where
    for<'x> EnumStructTestPartialState: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl Eq for EnumStructTestPartial where for<'x> EnumStructTestPartialState: Eq {}

impl EnumStructTestPartial {
    /// Creates a partial with no variant chosen.
    #[must_use]
    pub const fn new() -> Self {
        Self { state: None }
    }

    /// The chosen variant, if any.
    #[must_use]
    pub const fn variant(&self) -> Option<EnumStructTestVariants> {
        match &self.state {
            Some(state) => Some(state.variant()),
            None => None,
        }
    }

    /// The chosen variant and its partial, if any.
    #[must_use]
    pub const fn state(&self) -> Option<&EnumStructTestPartialState> {
        self.state.as_ref()
    }

    #[must_use]
    pub const fn state_mut(
        &mut self,
    ) -> Option<&mut EnumStructTestPartialState> {
        self.state.as_mut()
    }

    /// Removes the chosen variant and its partial, leaving none chosen.
    pub const fn take_state(&mut self) -> Option<EnumStructTestPartialState> {
        self.state.take()
    }

    /// Chooses a variant by name, for code that doesn't know the variants.
    /// Like the variant methods, this keeps the fields if `variant` was
    /// already chosen.
    pub fn set_variant(&mut self, variant: EnumStructTestVariants) {
        match variant {
            EnumStructTestVariants::NoValue => {
                self.no_value();
            },
            EnumStructTestVariants::SingleValue => {
                self.single_value();
            },
            EnumStructTestVariants::MultipleValues => {
                self.multiple_values();
            },
        }
    }

    // ----------------------------------------------------------- per variant
    // Choosing a variant keeps its partial if it was already chosen, and
    // otherwise discards the fields of the other variant.

    /// Chooses [`EnumStructTest::NoValue`] and returns its partial.
    pub fn no_value(&mut self) -> &mut EnumStructTestNoValuePartial {
        if !matches!(self.state, Some(EnumStructTestPartialState::NoValue(_))) {
            self.state = Some(EnumStructTestPartialState::NoValue(
                EnumStructTestNoValuePartial::new(),
            ));
        }
        match &mut self.state {
            Some(EnumStructTestPartialState::NoValue(partial)) => partial,
            _ => unreachable!("the variant was just chosen"),
        }
    }

    /// Chooses [`EnumStructTest::SingleValue`] and returns its partial.
    pub fn single_value(&mut self) -> &mut EnumStructTestSingleValuePartial {
        if !matches!(
            self.state,
            Some(EnumStructTestPartialState::SingleValue(_))
        ) {
            self.state = Some(EnumStructTestPartialState::SingleValue(
                EnumStructTestSingleValuePartial::new(),
            ));
        }
        match &mut self.state {
            Some(EnumStructTestPartialState::SingleValue(partial)) => partial,
            _ => unreachable!("the variant was just chosen"),
        }
    }

    /// Chooses [`EnumStructTest::MultipleValues`] and returns its partial.
    pub fn multiple_values(
        &mut self,
    ) -> &mut EnumStructTestMultipleValuesPartial {
        if !matches!(
            self.state,
            Some(EnumStructTestPartialState::MultipleValues(_))
        ) {
            self.state = Some(EnumStructTestPartialState::MultipleValues(
                EnumStructTestMultipleValuesPartial::new(),
            ));
        }
        match &mut self.state {
            Some(EnumStructTestPartialState::MultipleValues(partial)) => {
                partial
            },
            _ => unreachable!("the variant was just chosen"),
        }
    }
}

impl Default for EnumStructTestPartial {
    fn default() -> Self {
        Self::new()
    }
}

// ------------------------------------------------------------------ per enum
// Conversions.

impl From<EnumStructTest> for EnumStructTestPartial {
    fn from(value: EnumStructTest) -> Self {
        let state = match value {
            EnumStructTest::NoValue => {
                EnumStructTestPartialState::NoValue(
                    EnumStructTestNoValuePartial,
                )
            },
            EnumStructTest::SingleValue { foo } => {
                EnumStructTestPartialState::SingleValue(
                    EnumStructTestSingleValuePartial { foo: Some(foo) },
                )
            },
            EnumStructTest::MultipleValues { bar, zee } => {
                EnumStructTestPartialState::MultipleValues(
                    EnumStructTestMultipleValuesPartial {
                        bar: Some(bar),
                        zee: Some(zee),
                    },
                )
            },
        };
        Self { state: Some(state) }
    }
}

impl From<EnumStructTestPartialState> for EnumStructTestPartial {
    fn from(state: EnumStructTestPartialState) -> Self {
        Self { state: Some(state) }
    }
}

// --------------------------------------------------------------- per variant

impl From<EnumStructTestNoValuePartial> for EnumStructTestPartial {
    fn from(partial: EnumStructTestNoValuePartial) -> Self {
        EnumStructTestPartialState::NoValue(partial).into()
    }
}

impl From<EnumStructTestSingleValuePartial> for EnumStructTestPartial {
    fn from(partial: EnumStructTestSingleValuePartial) -> Self {
        EnumStructTestPartialState::SingleValue(partial).into()
    }
}

impl From<EnumStructTestMultipleValuesPartial> for EnumStructTestPartial {
    fn from(partial: EnumStructTestMultipleValuesPartial) -> Self {
        EnumStructTestPartialState::MultipleValues(partial).into()
    }
}

// ------------------------------------------------------------------ per enum
// The core traits.

impl Partly for EnumStructTest {
    type DefaultValidator = AcceptAll;
    type Field = EnumStructTestFields;
    type Partial = EnumStructTestPartial;
}

impl PartialOps for EnumStructTestPartial {
    type Field = EnumStructTestFields;
    type Target = EnumStructTest;

    fn has(&self, field: EnumStructTestFields) -> bool {
        match &self.state {
            None => false,
            Some(EnumStructTestPartialState::NoValue(partial)) => {
                partial.has(field)
            },
            Some(EnumStructTestPartialState::SingleValue(partial)) => {
                partial.has(field)
            },
            Some(EnumStructTestPartialState::MultipleValues(partial)) => {
                partial.has(field)
            },
        }
    }

    fn required_fields(&self) -> &'static [EnumStructTestFields] {
        match self.variant() {
            Some(variant) => variant.fields(),
            None => &[],
        }
    }

    /// Where both chose the same variant, its fields are merged. Otherwise,
    /// `other`'s choice wins, if it made one.
    fn merge(self, other: Self) -> Self {
        let state = match (self.state, other.state) {
            (
                Some(EnumStructTestPartialState::SingleValue(left)),
                Some(EnumStructTestPartialState::SingleValue(right)),
            ) => {
                Some(EnumStructTestPartialState::SingleValue(left.merge(right)))
            },
            (
                Some(EnumStructTestPartialState::MultipleValues(left)),
                Some(EnumStructTestPartialState::MultipleValues(right)),
            ) => {
                Some(EnumStructTestPartialState::MultipleValues(
                    left.merge(right),
                ))
            },
            (left, right) => right.or(left),
        };
        Self { state }
    }

    fn is_empty(&self) -> bool {
        self.state.is_none()
    }

    fn is_complete(&self) -> bool {
        self.state.is_some() && self.missing_fields().next().is_none()
    }
}

impl FromIterator<Self> for EnumStructTestPartial {
    /// Folds partials together. Later partials win.
    fn from_iter<I: IntoIterator<Item = Self>>(iter: I) -> Self {
        iter.into_iter().fold(Self::new(), PartialOps::merge)
    }
}

// --------------------------------------------------------------- per variant
// A variant partial builds the enum, but has no `From<EnumStructTest>`: the
// value may be another variant.

impl PartialOps for EnumStructTestNoValuePartial {
    type Field = EnumStructTestFields;
    type Target = EnumStructTest;

    fn has(&self, _field: EnumStructTestFields) -> bool {
        false
    }

    fn required_fields(&self) -> &'static [EnumStructTestFields] {
        EnumStructTestVariants::NoValue.fields()
    }

    fn merge(self, _other: Self) -> Self {
        self
    }
}

impl PartialOps for EnumStructTestSingleValuePartial {
    type Field = EnumStructTestFields;
    type Target = EnumStructTest;

    fn has(&self, field: EnumStructTestFields) -> bool {
        match field {
            EnumStructTestFields::SingleValueFoo => self.foo.is_some(),
            _ => false,
        }
    }

    fn required_fields(&self) -> &'static [EnumStructTestFields] {
        EnumStructTestVariants::SingleValue.fields()
    }

    fn merge(self, other: Self) -> Self {
        Self {
            foo: other.foo.or(self.foo),
        }
    }
}

impl PartialOps for EnumStructTestMultipleValuesPartial {
    type Field = EnumStructTestFields;
    type Target = EnumStructTest;

    fn has(&self, field: EnumStructTestFields) -> bool {
        match field {
            EnumStructTestFields::MultipleValuesBar => self.bar.is_some(),
            EnumStructTestFields::MultipleValuesZee => self.zee.is_some(),
            _ => false,
        }
    }

    fn required_fields(&self) -> &'static [EnumStructTestFields] {
        EnumStructTestVariants::MultipleValues.fields()
    }

    fn merge(self, other: Self) -> Self {
        Self {
            bar: other.bar.or(self.bar),
            zee: other.zee.or(self.zee),
        }
    }
}

// ------------------------------------------------------------------ per enum
// The build strategy, with one method per field of every variant.

/// Decides which variant an [`EnumStructTest`] gets, and the final value of
/// each field.
pub trait EnumStructTestBuildStrategy {
    type Error: From<MissingVariant> + From<MissingField<EnumStructTestFields>>;

    /// Returns `true` if the strategy expects to resolve every field.
    fn can_build(&self, partial: &EnumStructTestPartial) -> bool {
        partial.is_complete()
    }

    /// Chooses a variant when the partial has none. Its fields are then
    /// resolved like any others.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_variant(
        &self,
    ) -> Result<EnumStructTestPartialState, Self::Error> {
        Err(MissingVariant.into())
    }

    /// Produces the final `foo` of `SingleValue`, given the value that was set.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_single_value_foo(
        &self,
        value: Option<u32>,
    ) -> Result<u32, Self::Error> {
        value.ok_or_else(|| {
            MissingField(EnumStructTestFields::SingleValueFoo).into()
        })
    }

    /// Produces the final `bar` of `MultipleValues`, given the value that was
    /// set.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_multiple_values_bar(
        &self,
        value: Option<u32>,
    ) -> Result<u32, Self::Error> {
        value.ok_or_else(|| {
            MissingField(EnumStructTestFields::MultipleValuesBar).into()
        })
    }

    /// Produces the final `zee` of `MultipleValues`, given the value that was
    /// set.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_multiple_values_zee(
        &self,
        value: Option<String>,
    ) -> Result<String, Self::Error> {
        value.ok_or_else(|| {
            MissingField(EnumStructTestFields::MultipleValuesZee).into()
        })
    }
}

impl EnumStructTestBuildStrategy for DefaultBuildStrategy {
    type Error = MissingValue<EnumStructTestFields>;
}

// ------------------------------------------------------------------ per enum
// The validator, with one method per field of every variant.

/// Decides whether values of [`EnumStructTest`] are valid.
pub trait EnumStructTestValidator {
    type Error;

    /// Checks a value for `foo` of `SingleValue`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_single_value_foo(
        &self,
        _value: &u32,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks a value for `bar` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_multiple_values_bar(
        &self,
        _value: &u32,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks a value for `zee` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_multiple_values_zee(
        &self,
        _value: &String,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks the whole value, after every field has passed.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate(&self, _value: &EnumStructTest) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl EnumStructTestValidator for AcceptAll {
    type Error = Infallible;
}

/// Swaps the fields set on `input` into `target`, if both are the same variant.
/// Calling it a second time undoes it. Returns `false` if the variants differ.
fn __partly_swap_enum_struct_test(
    target: &mut EnumStructTest,
    input: &mut EnumStructTestPartial,
) -> bool {
    match (target, &mut input.state) {
        (_, None)
        | (
            EnumStructTest::NoValue,
            Some(EnumStructTestPartialState::NoValue(_)),
        ) => true,
        (
            EnumStructTest::SingleValue { foo },
            Some(EnumStructTestPartialState::SingleValue(partial)),
        ) => {
            if let Some(value) = &mut partial.foo {
                std::mem::swap(foo, value);
            }
            true
        },
        (
            EnumStructTest::MultipleValues { bar, zee },
            Some(EnumStructTestPartialState::MultipleValues(partial)),
        ) => {
            if let Some(value) = &mut partial.bar {
                std::mem::swap(bar, value);
            }
            if let Some(value) = &mut partial.zee {
                std::mem::swap(zee, value);
            }
            true
        },
        _ => false,
    }
}

impl<V: EnumStructTestValidator> ValidateWith<V> for EnumStructTest {
    type ApplyError = BuildFailure<
        MissingField<EnumStructTestFields>,
        ValidationErrors<EnumStructTestFields, V::Error>,
    >;
    type Error = V::Error;

    fn validate_with(
        &self,
        validator: &V,
    ) -> Result<(), ValidationErrors<EnumStructTestFields, V::Error>> {
        let mut errors = Vec::new();
        match self {
            Self::NoValue => {},
            Self::SingleValue { foo } => {
                if let Err(error) = validator.validate_single_value_foo(foo) {
                    errors.push(FieldError {
                        field: EnumStructTestFields::SingleValueFoo,
                        error,
                    });
                }
            },
            Self::MultipleValues { bar, zee } => {
                if let Err(error) = validator.validate_multiple_values_bar(bar)
                {
                    errors.push(FieldError {
                        field: EnumStructTestFields::MultipleValuesBar,
                        error,
                    });
                }
                if let Err(error) = validator.validate_multiple_values_zee(zee)
                {
                    errors.push(FieldError {
                        field: EnumStructTestFields::MultipleValuesZee,
                        error,
                    });
                }
            },
        }
        if !errors.is_empty() {
            return Err(ValidationErrors::Fields(errors));
        }
        validator.validate(self).map_err(ValidationErrors::Struct)
    }

    fn validate_partial_with(
        partial: &EnumStructTestPartial,
        validator: &V,
    ) -> Result<(), ValidationErrors<EnumStructTestFields, V::Error>> {
        let mut errors = Vec::new();
        match &partial.state {
            None | Some(EnumStructTestPartialState::NoValue(_)) => {},
            Some(EnumStructTestPartialState::SingleValue(partial)) => {
                if let Some(Err(error)) = partial
                    .foo
                    .as_ref()
                    .map(|value| validator.validate_single_value_foo(value))
                {
                    errors.push(FieldError {
                        field: EnumStructTestFields::SingleValueFoo,
                        error,
                    });
                }
            },
            Some(EnumStructTestPartialState::MultipleValues(partial)) => {
                if let Some(Err(error)) = partial
                    .bar
                    .as_ref()
                    .map(|value| validator.validate_multiple_values_bar(value))
                {
                    errors.push(FieldError {
                        field: EnumStructTestFields::MultipleValuesBar,
                        error,
                    });
                }
                if let Some(Err(error)) = partial
                    .zee
                    .as_ref()
                    .map(|value| validator.validate_multiple_values_zee(value))
                {
                    errors.push(FieldError {
                        field: EnumStructTestFields::MultipleValuesZee,
                        error,
                    });
                }
            },
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors::Fields(errors))
        }
    }

    /// A partial for the current variant writes the fields it sets. A partial
    /// for another variant must be complete, and replaces the value.
    fn apply_partial_with(
        &mut self,
        mut input: EnumStructTestPartial,
        validator: &V,
    ) -> Result<(), Rejected<EnumStructTestPartial, Self::ApplyError>> {
        if let Err(error) = Self::validate_partial_with(&input, validator) {
            return Err(Rejected {
                value: input,
                error: BuildFailure::Validation(error),
            });
        }

        if __partly_swap_enum_struct_test(self, &mut input) {
            if let Err(error) = validator.validate(self) {
                __partly_swap_enum_struct_test(self, &mut input);
                return Err(Rejected {
                    value: input,
                    error: BuildFailure::Validation(ValidationErrors::Struct(
                        error,
                    )),
                });
            }
            return Ok(());
        }

        let missing = input.missing_fields().next();
        if let Some(field) = missing {
            return Err(Rejected {
                value: input,
                error: BuildFailure::Strategy(MissingField(field)),
            });
        }
        let new = match input.state {
            Some(EnumStructTestPartialState::NoValue(_)) => Self::NoValue,
            Some(EnumStructTestPartialState::SingleValue(
                EnumStructTestSingleValuePartial { foo: Some(foo) },
            )) => Self::SingleValue { foo },
            Some(EnumStructTestPartialState::MultipleValues(
                EnumStructTestMultipleValuesPartial {
                    bar: Some(bar),
                    zee: Some(zee),
                },
            )) => Self::MultipleValues { bar, zee },
            _ => unreachable!("the partial is complete"),
        };
        let old = std::mem::replace(self, new);
        if let Err(error) = validator.validate(self) {
            let new = std::mem::replace(self, old);
            return Err(Rejected {
                value: new.into(),
                error: BuildFailure::Validation(ValidationErrors::Struct(
                    error,
                )),
            });
        }
        Ok(())
    }
}

// --------------------------------------------------------------- per variant
// Building a variant: resolve its fields, then validate the whole enum.

impl<S: EnumStructTestBuildStrategy, V: EnumStructTestValidator> BuildWith<S, V>
    for EnumStructTestNoValuePartial
{
    type Error = BuildFailure<
        S::Error,
        ValidationErrors<EnumStructTestFields, V::Error>,
    >;
    type Output = EnumStructTest;

    fn build_with(
        self,
        _strategy: &S,
        validator: &V,
    ) -> Result<EnumStructTest, Rejected<Self, Self::Error>> {
        let built = EnumStructTest::NoValue;
        match built.validate_with(validator) {
            Ok(()) => Ok(built),
            Err(error) => {
                Err(Rejected {
                    value: self,
                    error: BuildFailure::Validation(error),
                })
            },
        }
    }
}

impl<S: EnumStructTestBuildStrategy, V: EnumStructTestValidator> BuildWith<S, V>
    for EnumStructTestSingleValuePartial
{
    type Error = BuildFailure<
        S::Error,
        ValidationErrors<EnumStructTestFields, V::Error>,
    >;
    type Output = EnumStructTest;

    fn build_with(
        mut self,
        strategy: &S,
        validator: &V,
    ) -> Result<EnumStructTest, Rejected<Self, Self::Error>> {
        let foo = match strategy.resolve_single_value_foo(self.foo.take()) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let built = EnumStructTest::SingleValue { foo };
        match built.validate_with(validator) {
            Ok(()) => Ok(built),
            Err(error) => {
                let EnumStructTest::SingleValue { foo } = built else {
                    unreachable!("built as this variant")
                };
                Err(Rejected {
                    value: Self { foo: Some(foo) },
                    error: BuildFailure::Validation(error),
                })
            },
        }
    }
}

impl<S: EnumStructTestBuildStrategy, V: EnumStructTestValidator> BuildWith<S, V>
    for EnumStructTestMultipleValuesPartial
{
    type Error = BuildFailure<
        S::Error,
        ValidationErrors<EnumStructTestFields, V::Error>,
    >;
    type Output = EnumStructTest;

    fn build_with(
        mut self,
        strategy: &S,
        validator: &V,
    ) -> Result<EnumStructTest, Rejected<Self, Self::Error>> {
        let bar = match strategy.resolve_multiple_values_bar(self.bar.take()) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let zee = match strategy.resolve_multiple_values_zee(self.zee.take()) {
            Ok(value) => value,
            Err(error) => {
                self.bar = Some(bar);
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let built = EnumStructTest::MultipleValues { bar, zee };
        match built.validate_with(validator) {
            Ok(()) => Ok(built),
            Err(error) => {
                let EnumStructTest::MultipleValues { bar, zee } = built else {
                    unreachable!("built as this variant")
                };
                Err(Rejected {
                    value: Self {
                        bar: Some(bar),
                        zee: Some(zee),
                    },
                    error: BuildFailure::Validation(error),
                })
            },
        }
    }
}

// ------------------------------------------------------------------ per enum
// Building the enum: choose a variant if none is, then build that variant.

impl<S: EnumStructTestBuildStrategy, V: EnumStructTestValidator> BuildWith<S, V>
    for EnumStructTestPartial
{
    type Error = BuildFailure<
        S::Error,
        ValidationErrors<EnumStructTestFields, V::Error>,
    >;
    type Output = EnumStructTest;

    fn build_with(
        self,
        strategy: &S,
        validator: &V,
    ) -> Result<EnumStructTest, Rejected<Self, Self::Error>> {
        let state = match self.state {
            Some(state) => state,
            None => {
                match strategy.resolve_variant() {
                    Ok(state) => state,
                    Err(error) => {
                        return Err(Rejected {
                            value: Self::new(),
                            error: BuildFailure::Strategy(error),
                        });
                    },
                }
            },
        };
        match state {
            EnumStructTestPartialState::NoValue(partial) => {
                partial
                    .build_with(strategy, validator)
                    .map_err(|rejected| rejected.map_value(Self::from))
            },
            EnumStructTestPartialState::SingleValue(partial) => {
                partial
                    .build_with(strategy, validator)
                    .map_err(|rejected| rejected.map_value(Self::from))
            },
            EnumStructTestPartialState::MultipleValues(partial) => {
                partial
                    .build_with(strategy, validator)
                    .map_err(|rejected| rejected.map_value(Self::from))
            },
        }
    }
}

impl Build for EnumStructTestPartial {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = EnumStructTest;

    fn can_build(&self) -> bool {
        // Fully qualified: other derives in this module implement their own
        // strategy trait for `DefaultBuildStrategy`.
        EnumStructTestBuildStrategy::can_build(&DefaultBuildStrategy, self)
    }

    fn build(self) -> Result<EnumStructTest, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

// --------------------------------------------------------------- per variant
// The default strategy never overrides `can_build`, so `is_complete` is the
// same answer without needing an enum partial.

impl Build for EnumStructTestNoValuePartial {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = EnumStructTest;

    fn can_build(&self) -> bool {
        self.is_complete()
    }

    fn build(self) -> Result<EnumStructTest, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

impl Build for EnumStructTestSingleValuePartial {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = EnumStructTest;

    fn can_build(&self) -> bool {
        self.is_complete()
    }

    fn build(self) -> Result<EnumStructTest, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

impl Build for EnumStructTestMultipleValuesPartial {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = EnumStructTest;

    fn can_build(&self) -> bool {
        self.is_complete()
    }

    fn build(self) -> Result<EnumStructTest, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

// ------------------------------------------------------------------ per enum
// Reflection. The value enums have a variant per field of every variant, like
// the field enum. Their impls are written out for the same reason as `Foo`'s.

/// An owned field of [`EnumStructTest`] and its value.
pub enum EnumStructTestFieldValue {
    SingleValueFoo(u32),
    MultipleValuesBar(u32),
    MultipleValuesZee(String),
}

impl std::fmt::Debug for EnumStructTestFieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SingleValueFoo(value) => {
                f.debug_tuple("SingleValueFoo")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
            Self::MultipleValuesBar(value) => {
                f.debug_tuple("MultipleValuesBar")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
            Self::MultipleValuesZee(value) => {
                f.debug_tuple("MultipleValuesZee")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
        }
    }
}

impl Clone for EnumStructTestFieldValue
where
    for<'x> u32: Clone,
    for<'x> String: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::SingleValueFoo(value) => Self::SingleValueFoo(value.clone()),
            Self::MultipleValuesBar(value) => {
                Self::MultipleValuesBar(value.clone())
            },
            Self::MultipleValuesZee(value) => {
                Self::MultipleValuesZee(value.clone())
            },
        }
    }
}

impl PartialEq for EnumStructTestFieldValue
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SingleValueFoo(left), Self::SingleValueFoo(right)) => {
                left == right
            },
            (Self::MultipleValuesBar(left), Self::MultipleValuesBar(right)) => {
                left == right
            },
            (Self::MultipleValuesZee(left), Self::MultipleValuesZee(right)) => {
                left == right
            },
            _ => false,
        }
    }
}

impl EnumStructTestFieldValue {
    /// The field this value belongs to.
    #[must_use]
    pub const fn field(&self) -> EnumStructTestFields {
        match self {
            Self::SingleValueFoo(_) => EnumStructTestFields::SingleValueFoo,
            Self::MultipleValuesBar(_) => {
                EnumStructTestFields::MultipleValuesBar
            },
            Self::MultipleValuesZee(_) => {
                EnumStructTestFields::MultipleValuesZee
            },
        }
    }
}

/// A borrowed field of [`EnumStructTest`] and its value.
#[derive(Clone, Copy)]
pub enum EnumStructTestFieldRef<'a> {
    SingleValueFoo(&'a u32),
    MultipleValuesBar(&'a u32),
    MultipleValuesZee(&'a String),
}

impl std::fmt::Debug for EnumStructTestFieldRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SingleValueFoo(value) => {
                f.debug_tuple("SingleValueFoo")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
            Self::MultipleValuesBar(value) => {
                f.debug_tuple("MultipleValuesBar")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
            Self::MultipleValuesZee(value) => {
                f.debug_tuple("MultipleValuesZee")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
        }
    }
}

impl PartialEq for EnumStructTestFieldRef<'_>
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SingleValueFoo(left), Self::SingleValueFoo(right)) => {
                left == right
            },
            (Self::MultipleValuesBar(left), Self::MultipleValuesBar(right)) => {
                left == right
            },
            (Self::MultipleValuesZee(left), Self::MultipleValuesZee(right)) => {
                left == right
            },
            _ => false,
        }
    }
}

impl Reflect for EnumStructTestPartial {
    type Value = EnumStructTestFieldValue;
    type ValueRef<'a> = EnumStructTestFieldRef<'a>;

    fn iter(&self) -> impl Iterator<Item = EnumStructTestFieldRef<'_>> {
        let values: [Option<EnumStructTestFieldRef<'_>>;
            ENUM_STRUCT_TEST_MAX_FIELDS] = match &self.state {
            None | Some(EnumStructTestPartialState::NoValue(_)) => [None, None],
            Some(EnumStructTestPartialState::SingleValue(partial)) => {
                [
                    partial
                        .foo
                        .as_ref()
                        .map(EnumStructTestFieldRef::SingleValueFoo),
                    None,
                ]
            },
            Some(EnumStructTestPartialState::MultipleValues(partial)) => {
                [
                    partial
                        .bar
                        .as_ref()
                        .map(EnumStructTestFieldRef::MultipleValuesBar),
                    partial
                        .zee
                        .as_ref()
                        .map(EnumStructTestFieldRef::MultipleValuesZee),
                ]
            },
        };
        values.into_iter().flatten()
    }

    /// Switches to the value's variant first, if another one is chosen.
    fn set_field(
        &mut self,
        value: EnumStructTestFieldValue,
    ) -> Option<EnumStructTestFieldValue> {
        match value {
            EnumStructTestFieldValue::SingleValueFoo(value) => {
                self.single_value()
                    .foo
                    .replace(value)
                    .map(EnumStructTestFieldValue::SingleValueFoo)
            },
            EnumStructTestFieldValue::MultipleValuesBar(value) => {
                self.multiple_values()
                    .bar
                    .replace(value)
                    .map(EnumStructTestFieldValue::MultipleValuesBar)
            },
            EnumStructTestFieldValue::MultipleValuesZee(value) => {
                self.multiple_values()
                    .zee
                    .replace(value)
                    .map(EnumStructTestFieldValue::MultipleValuesZee)
            },
        }
    }

    /// Returns `None` for a field of a variant that isn't chosen.
    fn take_field(
        &mut self,
        field: EnumStructTestFields,
    ) -> Option<EnumStructTestFieldValue> {
        match (&mut self.state, field) {
            (
                Some(EnumStructTestPartialState::SingleValue(partial)),
                EnumStructTestFields::SingleValueFoo,
            ) => {
                partial
                    .foo
                    .take()
                    .map(EnumStructTestFieldValue::SingleValueFoo)
            },
            (
                Some(EnumStructTestPartialState::MultipleValues(partial)),
                EnumStructTestFields::MultipleValuesBar,
            ) => {
                partial
                    .bar
                    .take()
                    .map(EnumStructTestFieldValue::MultipleValuesBar)
            },
            (
                Some(EnumStructTestPartialState::MultipleValues(partial)),
                EnumStructTestFields::MultipleValuesZee,
            ) => {
                partial
                    .zee
                    .take()
                    .map(EnumStructTestFieldValue::MultipleValuesZee)
            },
            _ => None,
        }
    }
}

impl IntoIterator for EnumStructTestPartial {
    type IntoIter = std::iter::Flatten<
        std::array::IntoIter<
            Option<EnumStructTestFieldValue>,
            ENUM_STRUCT_TEST_MAX_FIELDS,
        >,
    >;
    type Item = EnumStructTestFieldValue;

    /// Moves out the fields. The chosen variant is lost, so a unit variant
    /// yields nothing.
    fn into_iter(self) -> Self::IntoIter {
        let values = match self.state {
            None | Some(EnumStructTestPartialState::NoValue(_)) => [None, None],
            Some(EnumStructTestPartialState::SingleValue(partial)) => {
                [
                    partial.foo.map(EnumStructTestFieldValue::SingleValueFoo),
                    None,
                ]
            },
            Some(EnumStructTestPartialState::MultipleValues(partial)) => {
                [
                    partial
                        .bar
                        .map(EnumStructTestFieldValue::MultipleValuesBar),
                    partial
                        .zee
                        .map(EnumStructTestFieldValue::MultipleValuesZee),
                ]
            },
        };
        values.into_iter().flatten()
    }
}

impl Extend<EnumStructTestFieldValue> for EnumStructTestPartial {
    fn extend<I: IntoIterator<Item = EnumStructTestFieldValue>>(
        &mut self,
        iter: I,
    ) {
        for value in iter {
            self.set_field(value);
        }
    }
}

impl FromIterator<EnumStructTestFieldValue> for EnumStructTestPartial {
    fn from_iter<I: IntoIterator<Item = EnumStructTestFieldValue>>(
        iter: I,
    ) -> Self {
        let mut partial = Self::new();
        partial.extend(iter);
        partial
    }
}

// ------------------------------------------------------------------ per enum
// The patch: the complete value it was made against, and a partial of what it
// writes. A partial of the same variant writes the fields set on it, like a
// struct patch. A partial of another variant replaces the whole value, which
// the merge strategy builds from it.
//
// As for structs, the macro can't see whether the enum is `Clone` or
// `PartialEq`, so values are copied and compared field by field.

/// Copies an [`EnumStructTest`] field by field.
fn __partly_clone_enum_struct_test(value: &EnumStructTest) -> EnumStructTest
where
    for<'x> u32: Clone,
    for<'x> String: Clone,
{
    match value {
        EnumStructTest::NoValue => EnumStructTest::NoValue,
        EnumStructTest::SingleValue { foo } => {
            EnumStructTest::SingleValue { foo: foo.clone() }
        },
        EnumStructTest::MultipleValues { bar, zee } => {
            EnumStructTest::MultipleValues {
                bar: bar.clone(),
                zee: zee.clone(),
            }
        },
    }
}

/// Lists how `target` differs from `old`: the variant, or else each field.
fn __partly_compare_enum_struct_test(
    old: &EnumStructTest,
    target: &EnumStructTest,
) -> Vec<EnumConflict<EnumStructTestFields>>
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    let mut conflicts = Vec::new();
    match (old, target) {
        (EnumStructTest::NoValue, EnumStructTest::NoValue) => {},
        (
            EnumStructTest::SingleValue { foo: old },
            EnumStructTest::SingleValue { foo: target },
        ) => {
            if old != target {
                conflicts.push(EnumConflict::Field(
                    EnumStructTestFields::SingleValueFoo,
                ));
            }
        },
        (
            EnumStructTest::MultipleValues {
                bar: old_bar,
                zee: old_zee,
            },
            EnumStructTest::MultipleValues {
                bar: target_bar,
                zee: target_zee,
            },
        ) => {
            if old_bar != target_bar {
                conflicts.push(EnumConflict::Field(
                    EnumStructTestFields::MultipleValuesBar,
                ));
            }
            if old_zee != target_zee {
                conflicts.push(EnumConflict::Field(
                    EnumStructTestFields::MultipleValuesZee,
                ));
            }
        },
        _ => conflicts.push(EnumConflict::Variant),
    }
    conflicts
}

/// Finds where two partials write something different: another variant, or
/// a field both write.
fn __partly_compare_writes_enum_struct_test(
    left: &EnumStructTestPartial,
    right: &EnumStructTestPartial,
) -> Option<EnumConflict<EnumStructTestFields>>
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    match (&left.state, &right.state) {
        (None, _)
        | (_, None)
        | (
            Some(EnumStructTestPartialState::NoValue(_)),
            Some(EnumStructTestPartialState::NoValue(_)),
        ) => None,
        (
            Some(EnumStructTestPartialState::SingleValue(left)),
            Some(EnumStructTestPartialState::SingleValue(right)),
        ) => {
            matches!((&left.foo, &right.foo), (Some(a), Some(b)) if a != b)
                .then_some(EnumConflict::Field(
                    EnumStructTestFields::SingleValueFoo,
                ))
        },
        (
            Some(EnumStructTestPartialState::MultipleValues(left)),
            Some(EnumStructTestPartialState::MultipleValues(right)),
        ) => {
            if matches!((&left.bar, &right.bar), (Some(a), Some(b)) if a != b) {
                Some(EnumConflict::Field(
                    EnumStructTestFields::MultipleValuesBar,
                ))
            } else if matches!((&left.zee, &right.zee), (Some(a), Some(b)) if a != b)
            {
                Some(EnumConflict::Field(
                    EnumStructTestFields::MultipleValuesZee,
                ))
            } else {
                None
            }
        },
        _ => Some(EnumConflict::Variant),
    }
}

/// The value `inner` leaves: `old` with the fields written or, for another
/// variant, the value built from `new` with the default strategy.
fn __partly_patched_enum_struct_test(
    inner: &EnumStructTestPatchInnerData,
) -> Result<EnumStructTest, MissingValue<EnumStructTestFields>>
where
    for<'x> u32: Clone,
    for<'x> String: Clone,
{
    let mut value = __partly_clone_enum_struct_test(&inner.old);
    let mut new = inner.new.clone();
    if __partly_swap_enum_struct_test(&mut value, &mut new) {
        return Ok(value);
    }
    match new.build() {
        Ok(value) => Ok(value),
        Err(rejected) => {
            match rejected.error {
                BuildFailure::Strategy(missing) => Err(missing),
                BuildFailure::Validation(_) => {
                    unreachable!("`AcceptAll` accepts every value")
                },
            }
        },
    }
}

/// The value an [`EnumStructTestPatch`] was made against, and what it writes.
pub struct EnumStructTestPatchInnerData {
    old: EnumStructTest,
    new: EnumStructTestPartial,
}

impl std::fmt::Debug for EnumStructTestPatchInnerData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnumStructTestPatchInnerData")
            .field("old", &partly::__debug_value!(&self.old))
            .field("new", &self.new)
            .finish()
    }
}

impl Clone for EnumStructTestPatchInnerData
where
    for<'x> u32: Clone,
    for<'x> String: Clone,
{
    fn clone(&self) -> Self {
        Self {
            old: __partly_clone_enum_struct_test(&self.old),
            new: self.new.clone(),
        }
    }
}

impl PartialEq for EnumStructTestPatchInnerData
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        __partly_compare_enum_struct_test(&self.old, &other.old).is_empty()
            && self.new == other.new
    }
}

impl Eq for EnumStructTestPatchInnerData
where
    for<'x> u32: Eq,
    for<'x> String: Eq,
{
}

/// Changes to an [`EnumStructTest`], made against a complete value of it.
#[derive(Debug)]
pub struct EnumStructTestPatch {
    inner: Option<EnumStructTestPatchInnerData>,
}

impl Clone for EnumStructTestPatch
where
    for<'x> EnumStructTestPatchInnerData: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl PartialEq for EnumStructTestPatch
where
    for<'x> EnumStructTestPatchInnerData: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for EnumStructTestPatch where for<'x> EnumStructTestPatchInnerData: Eq {}

impl EnumStructTestPatch {
    /// Creates an empty patch, which applies to any value.
    #[must_use]
    pub const fn new() -> Self {
        Self { inner: None }
    }

    /// Creates a patch made against `old`, which writes `new`.
    #[must_use]
    pub const fn from_parts(
        old: EnumStructTest,
        new: EnumStructTestPartial,
    ) -> Self {
        Self {
            inner: Some(EnumStructTestPatchInnerData { old, new }),
        }
    }

    /// The value the patch was made against and what it writes, unless it is
    /// empty.
    #[must_use]
    pub fn into_parts(self) -> Option<(EnumStructTest, EnumStructTestPartial)> {
        self.inner.map(|inner| (inner.old, inner.new))
    }

    /// The value the patch was made against, unless it is empty.
    #[must_use]
    pub const fn old_value(&self) -> Option<&EnumStructTest> {
        match &self.inner {
            Some(inner) => Some(&inner.old),
            None => None,
        }
    }

    /// What the patch writes, unless it is empty. Its variant and fields say
    /// which values are written.
    #[must_use]
    pub const fn new_value(&self) -> Option<&EnumStructTestPartial> {
        match &self.inner {
            Some(inner) => Some(&inner.new),
            None => None,
        }
    }

    #[must_use]
    pub const fn new_value_mut(
        &mut self,
    ) -> Option<&mut EnumStructTestPartial> {
        match &mut self.inner {
            Some(inner) => Some(&mut inner.new),
            None => None,
        }
    }

    /// Returns `true` if the patch writes a variant: the one it was made
    /// against, to change fields, or another one, to replace the value.
    #[must_use]
    pub fn has_variant(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|inner| inner.new.variant().is_some())
    }

    /// Returns `true` if the patch writes another variant than the one it was
    /// made against.
    #[must_use]
    pub fn changes_variant(&self) -> bool {
        self.inner.as_ref().is_some_and(|inner| {
            inner.new.variant().is_some_and(|variant| {
                variant != EnumStructTestVariants::from(&inner.old)
            })
        })
    }
}

impl Default for EnumStructTestPatch {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchOps for EnumStructTestPatch
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type BuildError = MissingValue<EnumStructTestFields>;
    type Conflict = EnumConflict<EnumStructTestFields>;
    type Target = EnumStructTest;

    fn fields(&self) -> &'static [EnumStructTestFields] {
        match self.inner.as_ref().and_then(|inner| inner.new.variant()) {
            Some(variant) => variant.fields(),
            None => &[],
        }
    }

    fn has(&self, field: EnumStructTestFields) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|inner| inner.new.has(field))
    }

    fn is_empty(&self) -> bool {
        self.inner.as_ref().is_none_or(|inner| inner.new.is_empty())
    }

    fn is_complete(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|inner| inner.new.is_complete())
    }

    fn is_noop(&self) -> bool {
        self.inner.as_ref().is_none_or(|inner| {
            match (&inner.old, &inner.new.state) {
                (_, None)
                | (
                    EnumStructTest::NoValue,
                    Some(EnumStructTestPartialState::NoValue(_)),
                ) => true,
                (
                    EnumStructTest::SingleValue { foo },
                    Some(EnumStructTestPartialState::SingleValue(partial)),
                ) => partial.foo.as_ref().is_none_or(|new| new == foo),
                (
                    EnumStructTest::MultipleValues { bar, zee },
                    Some(EnumStructTestPartialState::MultipleValues(partial)),
                ) => {
                    partial.bar.as_ref().is_none_or(|new| new == bar)
                        && partial.zee.as_ref().is_none_or(|new| new == zee)
                },
                _ => false,
            }
        })
    }

    fn check(
        &self,
        target: &EnumStructTest,
    ) -> Result<(), Vec<EnumConflict<EnumStructTestFields>>> {
        let Some(inner) = &self.inner else {
            return Ok(());
        };
        let conflicts = __partly_compare_enum_struct_test(&inner.old, target);
        if conflicts.is_empty() {
            Ok(())
        } else {
            Err(conflicts)
        }
    }

    fn invert(
        self,
    ) -> Result<Self, Rejected<Self, MissingValue<EnumStructTestFields>>> {
        let Some(mut inner) = self.inner else {
            return Ok(Self::new());
        };
        // The same variant: swapping turns `old` into the value this patch
        // leaves, and `new` into the values it replaces.
        if __partly_swap_enum_struct_test(&mut inner.old, &mut inner.new) {
            return Ok(Self { inner: Some(inner) });
        }
        // Another variant: the value left is built, and the undo writes all
        // of `old`.
        match __partly_patched_enum_struct_test(&inner) {
            Ok(left) => Ok(Self::from_parts(left, inner.old.into())),
            Err(error) => {
                Err(Rejected {
                    value: Self { inner: Some(inner) },
                    error,
                })
            },
        }
    }

    fn into_partial(self) -> EnumStructTestPartial {
        self.inner
            .map_or_else(EnumStructTestPartial::new, |inner| inner.new)
    }

    fn then(
        self,
        next: Self,
    ) -> Result<
        Self,
        Rejected<
            (Self, Self),
            ChainFailure<
                EnumConflict<EnumStructTestFields>,
                MissingValue<EnumStructTestFields>,
            >,
        >,
    > {
        let failure = match (&self.inner, &next.inner) {
            (Some(first), Some(second)) => {
                match __partly_patched_enum_struct_test(first) {
                    Ok(left) => {
                        __partly_compare_enum_struct_test(&second.old, &left)
                            .into_iter()
                            .next()
                            .map(ChainFailure::Conflict)
                    },
                    Err(error) => Some(ChainFailure::Build(error)),
                }
            },
            _ => None,
        };
        if let Some(error) = failure {
            return Err(Rejected {
                value: (self, next),
                error,
            });
        }
        Ok(match (self.inner, next.inner) {
            (Some(first), Some(second)) => {
                Self::from_parts(first.old, first.new.merge(second.new))
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
    ) -> Result<Self, Rejected<(Self, Self), EnumConflict<EnumStructTestFields>>>
    {
        let conflict = match (&self.inner, &other.inner) {
            (Some(left), Some(right)) => {
                __partly_compare_enum_struct_test(&left.old, &right.old)
                    .into_iter()
                    .next()
                    .or_else(|| {
                        __partly_compare_writes_enum_struct_test(
                            &left.new, &right.new,
                        )
                    })
            },
            _ => None,
        };
        if let Some(conflict) = conflict {
            return Err(Rejected {
                value: (self, other),
                error: conflict,
            });
        }
        Ok(match (self.inner, other.inner) {
            (Some(left), Some(right)) => {
                Self::from_parts(left.old, left.new.merge(right.new))
            },
            (left, right) => {
                Self {
                    inner: left.or(right),
                }
            },
        })
    }
}

// ------------------------------------------------------------------ per enum
// The patch being built against a borrowed value. Choosing the target's
// variant writes fields; choosing another one replaces the value.

/// An [`EnumStructTestPatch`] being built against a borrowed
/// [`EnumStructTest`].
pub struct BoundEnumStructTestPatch<'a> {
    target: &'a EnumStructTest,
    new:    EnumStructTestPartial,
}

impl std::fmt::Debug for BoundEnumStructTestPatch<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundEnumStructTestPatch")
            .field("target", &partly::__debug_value!(self.target))
            .field("new", &self.new)
            .finish()
    }
}

impl Clone for BoundEnumStructTestPatch<'_>
where
    for<'x> EnumStructTestPartial: Clone,
{
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            new:    self.new.clone(),
        }
    }
}

impl<'a> BoundEnumStructTestPatch<'a> {
    /// Starts a patch against `target`.
    #[must_use]
    pub const fn new(target: &'a EnumStructTest) -> Self {
        Self {
            target,
            new: EnumStructTestPartial::new(),
        }
    }

    /// What to write so far.
    #[must_use]
    pub const fn new_value(&self) -> &EnumStructTestPartial {
        &self.new
    }

    #[must_use]
    pub const fn new_value_mut(&mut self) -> &mut EnumStructTestPartial {
        &mut self.new
    }

    // ----------------------------------------------------------- per variant

    /// Writes [`EnumStructTest::NoValue`].
    pub fn no_value(&mut self) -> &mut EnumStructTestNoValuePartial {
        self.new.no_value()
    }

    /// Writes [`EnumStructTest::SingleValue`], with the fields set on the
    /// returned partial.
    pub fn single_value(&mut self) -> &mut EnumStructTestSingleValuePartial {
        self.new.single_value()
    }

    /// Writes [`EnumStructTest::MultipleValues`], with the fields set on the
    /// returned partial.
    pub fn multiple_values(
        &mut self,
    ) -> &mut EnumStructTestMultipleValuesPartial {
        self.new.multiple_values()
    }
}

impl<'a> BoundPatch<'a> for BoundEnumStructTestPatch<'a>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type Target = EnumStructTest;

    fn finish(self) -> EnumStructTestPatch {
        EnumStructTestPatch::from_parts(
            __partly_clone_enum_struct_test(self.target),
            self.new,
        )
    }
}

impl Patchable for EnumStructTest
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type Bound<'a> = BoundEnumStructTestPatch<'a>;
    type DefaultMergeStrategy = StrictMerge;
    type Patch = EnumStructTestPatch;

    fn patch(&self) -> BoundEnumStructTestPatch<'_> {
        BoundEnumStructTestPatch::new(self)
    }

    fn diff(&self, new: &Self) -> EnumStructTestPatch {
        let changes = match (self, new) {
            (Self::NoValue, Self::NoValue) => EnumStructTestPartial::new(),
            (
                Self::SingleValue { foo: before },
                Self::SingleValue { foo: after },
            ) => {
                let partial = EnumStructTestSingleValuePartial {
                    foo: (before != after).then(|| after.clone()),
                };
                if partial.is_empty() {
                    EnumStructTestPartial::new()
                } else {
                    partial.into()
                }
            },
            (
                Self::MultipleValues {
                    bar: before_bar,
                    zee: before_zee,
                },
                Self::MultipleValues {
                    bar: after_bar,
                    zee: after_zee,
                },
            ) => {
                let partial = EnumStructTestMultipleValuesPartial {
                    bar: (before_bar != after_bar).then(|| after_bar.clone()),
                    zee: (before_zee != after_zee).then(|| after_zee.clone()),
                };
                if partial.is_empty() {
                    EnumStructTestPartial::new()
                } else {
                    partial.into()
                }
            },
            _ => __partly_clone_enum_struct_test(new).into(),
        };
        EnumStructTestPatch::from_parts(
            __partly_clone_enum_struct_test(self),
            changes,
        )
    }
}

// ------------------------------------------------------------------ per enum
// The merge strategy: one method per field of every variant, for the same
// variant, and two for the whole value.

/// Settles conflicts when an [`EnumStructTestPatch`] is applied.
///
/// When the target is the variant the patch was made against and the patch
/// writes that variant, a field method runs for each field whose value
/// differs. `new` is the value the patch writes, or `None` if it doesn't
/// write the field. Every other difference goes to
/// [`resolve_variant`](Self::resolve_variant).
pub trait EnumStructTestMergeStrategy {
    type Error: From<EnumConflict<EnumStructTestFields>>
        + From<MissingValue<EnumStructTestFields>>;

    /// Settles a conflict on `foo` of `SingleValue`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_single_value_foo(
        &self,
        _current: &u32,
        _old: &u32,
        _new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        Err(EnumConflict::Field(EnumStructTestFields::SingleValueFoo).into())
    }

    /// Settles a conflict on `bar` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_multiple_values_bar(
        &self,
        _current: &u32,
        _old: &u32,
        _new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        Err(EnumConflict::Field(EnumStructTestFields::MultipleValuesBar).into())
    }

    /// Settles a conflict on `zee` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_multiple_values_zee(
        &self,
        _current: &String,
        _old: &String,
        _new: Option<&String>,
    ) -> Result<Resolution<String>, Self::Error> {
        Err(EnumConflict::Field(EnumStructTestFields::MultipleValuesZee).into())
    }

    /// Settles a conflict on the whole value, for the reason in `conflict`.
    /// `UseNew` writes what [`build_new`](Self::build_new) builds from `new`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_variant(
        &self,
        _current: &EnumStructTest,
        _old: &EnumStructTest,
        _new: &EnumStructTestPartial,
        conflict: &VariantConflict<EnumStructTestFields>,
    ) -> Result<Resolution<EnumStructTest>, Self::Error> {
        Err(match conflict {
            VariantConflict::OtherVariant => EnumConflict::Variant,
            VariantConflict::Fields(fields) => {
                fields.first().map_or(EnumConflict::Variant, |field| {
                    EnumConflict::Field(*field)
                })
            },
        }
        .into())
    }

    /// Builds the value that replaces the target when the patch writes
    /// another variant.
    ///
    /// # Errors
    ///
    /// By default, builds with the default build strategy, which fails on a
    /// missing field.
    fn build_new(
        &self,
        new: &EnumStructTestPartial,
    ) -> Result<EnumStructTest, Self::Error>
    where
        for<'x> EnumStructTestPartial: Clone,
    {
        match new.clone().build() {
            Ok(value) => Ok(value),
            Err(rejected) => {
                match rejected.error {
                    BuildFailure::Strategy(missing) => Err(missing.into()),
                    BuildFailure::Validation(_) => {
                        unreachable!("`AcceptAll` accepts every value")
                    },
                }
            },
        }
    }
}

impl EnumStructTestMergeStrategy for StrictMerge {
    type Error = EnumMergeError<EnumStructTestFields>;
}

impl EnumStructTestMergeStrategy for FieldMerge {
    type Error = EnumMergeError<EnumStructTestFields>;

    fn resolve_single_value_foo(
        &self,
        _current: &u32,
        _old: &u32,
        new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => {
                Err(EnumConflict::Field(EnumStructTestFields::SingleValueFoo)
                    .into())
            },
        }
    }

    fn resolve_multiple_values_bar(
        &self,
        _current: &u32,
        _old: &u32,
        new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => {
                Err(EnumConflict::Field(
                    EnumStructTestFields::MultipleValuesBar,
                )
                .into())
            },
        }
    }

    fn resolve_multiple_values_zee(
        &self,
        _current: &String,
        _old: &String,
        new: Option<&String>,
    ) -> Result<Resolution<String>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => {
                Err(EnumConflict::Field(
                    EnumStructTestFields::MultipleValuesZee,
                )
                .into())
            },
        }
    }
}

/// Swaps the values chosen for each field with the target's. Calling it a
/// second time undoes it.
fn __partly_swap_resolved_enum_struct_test(
    target: &mut EnumStructTest,
    new: &mut Option<EnumStructTestPartialState>,
    foo: &mut Option<Resolution<u32>>,
    bar: &mut Option<Resolution<u32>>,
    zee: &mut Option<Resolution<String>>,
) {
    let (new_foo, new_bar, new_zee) = match new {
        Some(EnumStructTestPartialState::SingleValue(partial)) => {
            (partial.foo.as_mut(), None, None)
        },
        Some(EnumStructTestPartialState::MultipleValues(partial)) => {
            (None, partial.bar.as_mut(), partial.zee.as_mut())
        },
        _ => (None, None, None),
    };
    match target {
        EnumStructTest::NoValue => {},
        EnumStructTest::SingleValue { foo: current } => {
            if let Some(resolution) = foo {
                resolution.__swap(current, new_foo);
            }
        },
        EnumStructTest::MultipleValues {
            bar: current_bar,
            zee: current_zee,
        } => {
            if let Some(resolution) = bar {
                resolution.__swap(current_bar, new_bar);
            }
            if let Some(resolution) = zee {
                resolution.__swap(current_zee, new_zee);
            }
        },
    }
}

/// Applies a patch whose variant is the target's and the one it was made
/// against, field by field like a struct patch.
#[expect(clippy::type_complexity, reason = "the bound is the signature")]
fn __partly_apply_fields_enum_struct_test<
    M: EnumStructTestMergeStrategy,
    V: EnumStructTestValidator,
>(
    target: &mut EnumStructTest,
    mut inner: EnumStructTestPatchInnerData,
    strategy: &M,
    validator: &V,
) -> Result<
    EnumStructTestPartial,
    Rejected<
        EnumStructTestPatch,
        ApplyFailure<
            M::Error,
            ValidationErrors<EnumStructTestFields, V::Error>,
        >,
    >,
>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    // Phase 1: settle every field of the variant. Nothing has changed yet.
    let mut foo: Option<Resolution<u32>> = None;
    let mut bar: Option<Resolution<u32>> = None;
    let mut zee: Option<Resolution<String>> = None;
    let settled = 'settle: {
        match (&*target, &inner.old, &inner.new.state) {
            (
                EnumStructTest::SingleValue { foo: current },
                EnumStructTest::SingleValue { foo: old },
                new,
            ) => {
                let new = match new {
                    Some(EnumStructTestPartialState::SingleValue(partial)) => {
                        partial.foo.as_ref()
                    },
                    _ => None,
                };
                foo = match partly::__private::settle(
                    current,
                    old,
                    new,
                    |current, old, new| {
                        strategy.resolve_single_value_foo(current, old, new)
                    },
                ) {
                    Ok(resolution) => resolution,
                    Err(error) => break 'settle Err(error),
                };
            },
            (
                EnumStructTest::MultipleValues {
                    bar: current_bar,
                    zee: current_zee,
                },
                EnumStructTest::MultipleValues {
                    bar: old_bar,
                    zee: old_zee,
                },
                new,
            ) => {
                let new = match new {
                    Some(EnumStructTestPartialState::MultipleValues(
                        partial,
                    )) => Some(partial),
                    _ => None,
                };
                bar = match partly::__private::settle(
                    current_bar,
                    old_bar,
                    new.and_then(|partial| partial.bar.as_ref()),
                    |current, old, new| {
                        strategy.resolve_multiple_values_bar(current, old, new)
                    },
                ) {
                    Ok(resolution) => resolution,
                    Err(error) => break 'settle Err(error),
                };
                zee = match partly::__private::settle(
                    current_zee,
                    old_zee,
                    new.and_then(|partial| partial.zee.as_ref()),
                    |current, old, new| {
                        strategy.resolve_multiple_values_zee(current, old, new)
                    },
                ) {
                    Ok(resolution) => resolution,
                    Err(error) => break 'settle Err(error),
                };
            },
            _ => {},
        }
        Ok(())
    };
    if let Err(error) = settled {
        return Err(Rejected {
            value: EnumStructTestPatch { inner: Some(inner) },
            error: ApplyFailure::Merge(error),
        });
    }

    // Phase 2: validate the values that would be written.
    let (new_foo, new_bar, new_zee) = match &inner.new.state {
        Some(EnumStructTestPartialState::SingleValue(partial)) => {
            (partial.foo.as_ref(), None, None)
        },
        Some(EnumStructTestPartialState::MultipleValues(partial)) => {
            (None, partial.bar.as_ref(), partial.zee.as_ref())
        },
        _ => (None, None, None),
    };
    let mut errors = Vec::new();
    if let Some(Err(error)) = foo
        .as_ref()
        .and_then(|resolution| resolution.chosen(new_foo))
        .map(|value| validator.validate_single_value_foo(value))
    {
        errors.push(FieldError {
            field: EnumStructTestFields::SingleValueFoo,
            error,
        });
    }
    if let Some(Err(error)) = bar
        .as_ref()
        .and_then(|resolution| resolution.chosen(new_bar))
        .map(|value| validator.validate_multiple_values_bar(value))
    {
        errors.push(FieldError {
            field: EnumStructTestFields::MultipleValuesBar,
            error,
        });
    }
    if let Some(Err(error)) = zee
        .as_ref()
        .and_then(|resolution| resolution.chosen(new_zee))
        .map(|value| validator.validate_multiple_values_zee(value))
    {
        errors.push(FieldError {
            field: EnumStructTestFields::MultipleValuesZee,
            error,
        });
    }
    if !errors.is_empty() {
        return Err(Rejected {
            value: EnumStructTestPatch { inner: Some(inner) },
            error: ApplyFailure::Validation(ValidationErrors::Fields(errors)),
        });
    }

    // Phase 3: write, and undo if the whole-value check fails.
    __partly_swap_resolved_enum_struct_test(
        target,
        &mut inner.new.state,
        &mut foo,
        &mut bar,
        &mut zee,
    );
    if let Err(error) = validator.validate(target) {
        __partly_swap_resolved_enum_struct_test(
            target,
            &mut inner.new.state,
            &mut foo,
            &mut bar,
            &mut zee,
        );
        return Err(Rejected {
            value: EnumStructTestPatch { inner: Some(inner) },
            error: ApplyFailure::Validation(ValidationErrors::Struct(error)),
        });
    }

    // After the swaps, the patch holds the values that were replaced.
    let (new_foo, new_bar, new_zee) = match inner.new.state {
        Some(EnumStructTestPartialState::SingleValue(partial)) => {
            (partial.foo, None, None)
        },
        Some(EnumStructTestPartialState::MultipleValues(partial)) => {
            (None, partial.bar, partial.zee)
        },
        _ => (None, None, None),
    };
    Ok(match &*target {
        EnumStructTest::NoValue => EnumStructTestPartial::new(),
        EnumStructTest::SingleValue { .. } => {
            let partial = EnumStructTestSingleValuePartial {
                foo: foo.and_then(|resolution| resolution.__previous(new_foo)),
            };
            if partial.is_empty() {
                EnumStructTestPartial::new()
            } else {
                partial.into()
            }
        },
        EnumStructTest::MultipleValues { .. } => {
            let partial = EnumStructTestMultipleValuesPartial {
                bar: bar.and_then(|resolution| resolution.__previous(new_bar)),
                zee: zee.and_then(|resolution| resolution.__previous(new_zee)),
            };
            if partial.is_empty() {
                EnumStructTestPartial::new()
            } else {
                partial.into()
            }
        },
    })
}

/// Applies a patch that replaces the whole value: it writes another variant,
/// or the target is another variant than it was made against.
#[expect(clippy::type_complexity, reason = "the bound is the signature")]
fn __partly_apply_value_enum_struct_test<
    M: EnumStructTestMergeStrategy,
    V: EnumStructTestValidator,
>(
    target: &mut EnumStructTest,
    inner: EnumStructTestPatchInnerData,
    strategy: &M,
    validator: &V,
) -> Result<
    EnumStructTestPartial,
    Rejected<
        EnumStructTestPatch,
        ApplyFailure<
            M::Error,
            ValidationErrors<EnumStructTestFields, V::Error>,
        >,
    >,
>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    // Phase 1: settle the whole value.
    let conflict = if EnumStructTestVariants::from(&*target)
        == EnumStructTestVariants::from(&inner.old)
    {
        let fields: Vec<_> =
            __partly_compare_enum_struct_test(&inner.old, target)
                .into_iter()
                .filter_map(|conflict| {
                    match conflict {
                        EnumConflict::Field(field) => Some(field),
                        EnumConflict::Variant => None,
                    }
                })
                .collect();
        (!fields.is_empty()).then_some(VariantConflict::Fields(fields))
    } else {
        Some(VariantConflict::OtherVariant)
    };
    let resolution = match &conflict {
        None => Resolution::UseNew,
        Some(conflict) => {
            match strategy
                .resolve_variant(target, &inner.old, &inner.new, conflict)
            {
                Ok(resolution) => resolution,
                Err(error) => {
                    return Err(Rejected {
                        value: EnumStructTestPatch { inner: Some(inner) },
                        error: ApplyFailure::Merge(error),
                    });
                },
            }
        },
    };
    let mut value = match resolution {
        Resolution::KeepCurrent => return Ok(EnumStructTestPartial::new()),
        Resolution::UseNew => {
            match strategy.build_new(&inner.new) {
                Ok(value) => value,
                Err(error) => {
                    return Err(Rejected {
                        value: EnumStructTestPatch { inner: Some(inner) },
                        error: ApplyFailure::Merge(error),
                    });
                },
            }
        },
        Resolution::Use(value) => value,
    };

    // Phase 2: validate the value that would be written, fields first.
    if let Err(error) = value.validate_with(validator) {
        return Err(Rejected {
            value: EnumStructTestPatch { inner: Some(inner) },
            error: ApplyFailure::Validation(error),
        });
    }

    // Phase 3: write. The replaced value is what an undo writes back.
    std::mem::swap(target, &mut value);
    Ok(value.into())
}

/// Applies `patch` to `target`, and returns the values it replaced.
#[expect(clippy::type_complexity, reason = "the bound is the signature")]
fn __partly_apply_enum_struct_test_patch<
    M: EnumStructTestMergeStrategy,
    V: EnumStructTestValidator,
>(
    target: &mut EnumStructTest,
    patch: EnumStructTestPatch,
    strategy: &M,
    validator: &V,
) -> Result<
    EnumStructTestPartial,
    Rejected<
        EnumStructTestPatch,
        ApplyFailure<
            M::Error,
            ValidationErrors<EnumStructTestFields, V::Error>,
        >,
    >,
>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    let Some(inner) = patch.inner else {
        return Ok(EnumStructTestPartial::new());
    };
    let old_variant = EnumStructTestVariants::from(&inner.old);
    let same_target = EnumStructTestVariants::from(&*target) == old_variant;
    let same_write = inner
        .new
        .variant()
        .is_none_or(|variant| variant == old_variant);
    if same_target && same_write {
        __partly_apply_fields_enum_struct_test(
            target, inner, strategy, validator,
        )
    } else {
        __partly_apply_value_enum_struct_test(
            target, inner, strategy, validator,
        )
    }
}

impl<M: EnumStructTestMergeStrategy, V: EnumStructTestValidator>
    ApplyPatchWith<M, V> for EnumStructTest
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type MergeError = M::Error;

    fn apply_patch_with(
        &mut self,
        patch: EnumStructTestPatch,
        strategy: &M,
        validator: &V,
    ) -> Result<
        (),
        Rejected<
            EnumStructTestPatch,
            ApplyFailure<
                M::Error,
                ValidationErrors<EnumStructTestFields, V::Error>,
            >,
        >,
    > {
        __partly_apply_enum_struct_test_patch(self, patch, strategy, validator)
            .map(|_replaced| ())
    }

    fn apply_patch_with_get_undo(
        &mut self,
        patch: EnumStructTestPatch,
        strategy: &M,
        validator: &V,
    ) -> Result<
        EnumStructTestPatch,
        Rejected<
            EnumStructTestPatch,
            ApplyFailure<
                M::Error,
                ValidationErrors<EnumStructTestFields, V::Error>,
            >,
        >,
    > {
        let replaced = __partly_apply_enum_struct_test_patch(
            self, patch, strategy, validator,
        )?;
        Ok(if replaced.is_empty() {
            EnumStructTestPatch::new()
        } else {
            EnumStructTestPatch::from_parts(
                __partly_clone_enum_struct_test(self),
                replaced,
            )
        })
    }
}

// ===================================================================== tuple
// The same expansion for tuple variants. Positional fields are named by index:
// `MultipleValues0` in the field enum, and `get_0`, `set_0`, `has_0`, … on
// the partial, since `0` alone isn't a method name.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumTupleTest {
    NoValue,
    SingleValue(u32),
    MultipleValues(u32, u32, String),
}

// ------------------------------------------------------------------ per enum

/// Names a variant of [`EnumTupleTest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnumTupleTestVariants {
    NoValue,
    SingleValue,
    MultipleValues,
}

impl EnumTupleTestVariants {
    /// Every variant, in declaration order.
    pub const ALL: &'static [Self] =
        &[Self::NoValue, Self::SingleValue, Self::MultipleValues];

    /// The name of the variant as written in the enum.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoValue => "NoValue",
            Self::SingleValue => "SingleValue",
            Self::MultipleValues => "MultipleValues",
        }
    }

    /// The fields of this variant, in declaration order.
    #[must_use]
    pub const fn fields(self) -> &'static [EnumTupleTestFields] {
        match self {
            Self::NoValue => &[],
            Self::SingleValue => &[EnumTupleTestFields::SingleValue0],
            Self::MultipleValues => {
                &[
                    EnumTupleTestFields::MultipleValues0,
                    EnumTupleTestFields::MultipleValues1,
                    EnumTupleTestFields::MultipleValues2,
                ]
            },
        }
    }
}

impl From<&EnumTupleTest> for EnumTupleTestVariants {
    fn from(value: &EnumTupleTest) -> Self {
        match value {
            EnumTupleTest::NoValue => Self::NoValue,
            EnumTupleTest::SingleValue(..) => Self::SingleValue,
            EnumTupleTest::MultipleValues(..) => Self::MultipleValues,
        }
    }
}

// ------------------------------------------------------------------ per enum

/// Names a field of [`EnumTupleTest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnumTupleTestFields {
    SingleValue0,
    MultipleValues0,
    MultipleValues1,
    MultipleValues2,
}

impl Field for EnumTupleTestFields {
    const ALL: &'static [Self] = &[
        Self::SingleValue0,
        Self::MultipleValues0,
        Self::MultipleValues1,
        Self::MultipleValues2,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::SingleValue0 | Self::MultipleValues0 => "0",
            Self::MultipleValues1 => "1",
            Self::MultipleValues2 => "2",
        }
    }
}

impl EnumTupleTestFields {
    /// The variant this field belongs to.
    #[must_use]
    pub const fn variant(self) -> EnumTupleTestVariants {
        match self {
            Self::SingleValue0 => EnumTupleTestVariants::SingleValue,
            Self::MultipleValues0
            | Self::MultipleValues1
            | Self::MultipleValues2 => EnumTupleTestVariants::MultipleValues,
        }
    }
}

// --------------------------------------------------------------- per variant

/// The partial of [`EnumTupleTest::NoValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EnumTupleTestNoValuePartial;

impl EnumTupleTestNoValuePartial {
    /// Creates the partial.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

// --------------------------------------------------------------- per variant

/// The partial of [`EnumTupleTest::SingleValue`].
#[derive(Clone, PartialEq, Eq)]
pub struct EnumTupleTestSingleValuePartial<P0 = Option<u32>>(P0);

impl<P0> std::fmt::Debug for EnumTupleTestSingleValuePartial<P0>
where
    P0: partly::__private::Slot<u32>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EnumTupleTestSingleValuePartial")
            .field(&partly::__debug_slot!(&self.0))
            .finish()
    }
}

/// A partial with every field known to be set.
pub type EnumTupleTestSingleValueCompletePartial =
    EnumTupleTestSingleValuePartial<Set<u32>>;

impl EnumTupleTestSingleValuePartial {
    /// Creates an empty partial.
    #[must_use]
    pub const fn new() -> Self {
        Self(None)
    }
}

impl Default for EnumTupleTestSingleValuePartial {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------- per field

impl EnumTupleTestSingleValuePartial<Option<u32>> {
    #[must_use]
    pub const fn get_0(&self) -> Option<&u32> {
        self.0.as_ref()
    }

    #[must_use]
    pub const fn get_0_mut(&mut self) -> Option<&mut u32> {
        self.0.as_mut()
    }

    #[must_use]
    pub const fn has_0(&self) -> bool {
        self.0.is_some()
    }

    pub fn set_0(&mut self, value: u32) {
        self.0 = Some(value);
    }

    #[must_use]
    pub fn with_0(mut self, value: u32) -> Self {
        self.0 = Some(value);
        self
    }

    pub const fn take_0(&mut self) -> Option<u32> {
        self.0.take()
    }

    pub fn clear_0(&mut self) {
        self.0 = None;
    }

    /// Moves field `0` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if field `0` isn't set.
    pub fn require_0(
        self,
    ) -> Result<EnumTupleTestSingleValuePartial<Set<u32>>, Self> {
        match self.0 {
            Some(value) => Ok(EnumTupleTestSingleValuePartial(Set(value))),
            None => Err(self),
        }
    }
}

impl EnumTupleTestSingleValuePartial<Set<u32>> {
    #[must_use]
    pub const fn get_0(&self) -> &u32 {
        &self.0.0
    }
}

impl From<EnumTupleTestSingleValueCompletePartial>
    for EnumTupleTestSingleValuePartial
{
    fn from(value: EnumTupleTestSingleValueCompletePartial) -> Self {
        Self(Some(value.0.0))
    }
}

// --------------------------------------------------------------- per variant

/// The partial of [`EnumTupleTest::MultipleValues`].
#[derive(Clone, PartialEq, Eq)]
pub struct EnumTupleTestMultipleValuesPartial<
    P0 = Option<u32>,
    P1 = Option<u32>,
    P2 = Option<String>,
>(P0, P1, P2);

impl<P0, P1, P2> std::fmt::Debug
    for EnumTupleTestMultipleValuesPartial<P0, P1, P2>
where
    P0: partly::__private::Slot<u32>,
    P1: partly::__private::Slot<u32>,
    P2: partly::__private::Slot<String>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EnumTupleTestMultipleValuesPartial")
            .field(&partly::__debug_slot!(&self.0))
            .field(&partly::__debug_slot!(&self.1))
            .field(&partly::__debug_slot!(&self.2))
            .finish()
    }
}

/// A partial with every field known to be set.
pub type EnumTupleTestMultipleValuesCompletePartial =
    EnumTupleTestMultipleValuesPartial<Set<u32>, Set<u32>, Set<String>>;

impl EnumTupleTestMultipleValuesPartial {
    /// Creates an empty partial.
    #[must_use]
    pub const fn new() -> Self {
        Self(None, None, None)
    }
}

impl Default for EnumTupleTestMultipleValuesPartial {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------- per field

impl<P1, P2> EnumTupleTestMultipleValuesPartial<Option<u32>, P1, P2> {
    #[must_use]
    pub const fn get_0(&self) -> Option<&u32> {
        self.0.as_ref()
    }

    #[must_use]
    pub const fn get_0_mut(&mut self) -> Option<&mut u32> {
        self.0.as_mut()
    }

    #[must_use]
    pub const fn has_0(&self) -> bool {
        self.0.is_some()
    }

    pub fn set_0(&mut self, value: u32) {
        self.0 = Some(value);
    }

    #[must_use]
    pub fn with_0(mut self, value: u32) -> Self {
        self.0 = Some(value);
        self
    }

    pub const fn take_0(&mut self) -> Option<u32> {
        self.0.take()
    }

    pub fn clear_0(&mut self) {
        self.0 = None;
    }

    /// Moves field `0` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if field `0` isn't set.
    pub fn require_0(
        self,
    ) -> Result<EnumTupleTestMultipleValuesPartial<Set<u32>, P1, P2>, Self>
    {
        match self.0 {
            Some(value) => {
                Ok(EnumTupleTestMultipleValuesPartial(
                    Set(value),
                    self.1,
                    self.2,
                ))
            },
            None => Err(self),
        }
    }
}

impl<P1, P2> EnumTupleTestMultipleValuesPartial<Set<u32>, P1, P2> {
    #[must_use]
    pub const fn get_0(&self) -> &u32 {
        &self.0.0
    }
}

impl<P0, P2> EnumTupleTestMultipleValuesPartial<P0, Option<u32>, P2> {
    #[must_use]
    pub const fn get_1(&self) -> Option<&u32> {
        self.1.as_ref()
    }

    #[must_use]
    pub const fn get_1_mut(&mut self) -> Option<&mut u32> {
        self.1.as_mut()
    }

    #[must_use]
    pub const fn has_1(&self) -> bool {
        self.1.is_some()
    }

    pub fn set_1(&mut self, value: u32) {
        self.1 = Some(value);
    }

    #[must_use]
    pub fn with_1(mut self, value: u32) -> Self {
        self.1 = Some(value);
        self
    }

    pub const fn take_1(&mut self) -> Option<u32> {
        self.1.take()
    }

    pub fn clear_1(&mut self) {
        self.1 = None;
    }

    /// Moves field `1` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if field `1` isn't set.
    pub fn require_1(
        self,
    ) -> Result<EnumTupleTestMultipleValuesPartial<P0, Set<u32>, P2>, Self>
    {
        match self.1 {
            Some(value) => {
                Ok(EnumTupleTestMultipleValuesPartial(
                    self.0,
                    Set(value),
                    self.2,
                ))
            },
            None => Err(self),
        }
    }
}

impl<P0, P2> EnumTupleTestMultipleValuesPartial<P0, Set<u32>, P2> {
    #[must_use]
    pub const fn get_1(&self) -> &u32 {
        &self.1.0
    }
}

impl<P0, P1> EnumTupleTestMultipleValuesPartial<P0, P1, Option<String>> {
    #[must_use]
    pub const fn get_2(&self) -> Option<&String> {
        self.2.as_ref()
    }

    #[must_use]
    pub const fn get_2_mut(&mut self) -> Option<&mut String> {
        self.2.as_mut()
    }

    #[must_use]
    pub const fn has_2(&self) -> bool {
        self.2.is_some()
    }

    pub fn set_2(&mut self, value: String) {
        self.2 = Some(value);
    }

    #[must_use]
    pub fn with_2(mut self, value: String) -> Self {
        self.2 = Some(value);
        self
    }

    pub const fn take_2(&mut self) -> Option<String> {
        self.2.take()
    }

    pub fn clear_2(&mut self) {
        self.2 = None;
    }

    /// Moves field `2` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if field `2` isn't set.
    pub fn require_2(
        self,
    ) -> Result<EnumTupleTestMultipleValuesPartial<P0, P1, Set<String>>, Self>
    {
        match self.2 {
            Some(value) => {
                Ok(EnumTupleTestMultipleValuesPartial(
                    self.0,
                    self.1,
                    Set(value),
                ))
            },
            None => Err(self),
        }
    }
}

impl<P0, P1> EnumTupleTestMultipleValuesPartial<P0, P1, Set<String>> {
    #[must_use]
    pub const fn get_2(&self) -> &String {
        &self.2.0
    }
}

impl From<EnumTupleTestMultipleValuesCompletePartial>
    for EnumTupleTestMultipleValuesPartial
{
    fn from(value: EnumTupleTestMultipleValuesCompletePartial) -> Self {
        Self(Some(value.0.0), Some(value.1.0), Some(value.2.0))
    }
}

// ------------------------------------------------------------------ per enum

/// The chosen variant of an [`EnumTupleTestPartial`], and its partial.
#[derive(Debug)]
pub enum EnumTupleTestPartialState {
    NoValue(EnumTupleTestNoValuePartial),
    SingleValue(EnumTupleTestSingleValuePartial),
    MultipleValues(EnumTupleTestMultipleValuesPartial),
}

impl Clone for EnumTupleTestPartialState
where
    for<'x> EnumTupleTestSingleValuePartial: Clone,
    for<'x> EnumTupleTestMultipleValuesPartial: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::NoValue(partial) => Self::NoValue(partial.clone()),
            Self::SingleValue(partial) => Self::SingleValue(partial.clone()),
            Self::MultipleValues(partial) => {
                Self::MultipleValues(partial.clone())
            },
        }
    }
}

impl PartialEq for EnumTupleTestPartialState
where
    for<'x> EnumTupleTestSingleValuePartial: PartialEq,
    for<'x> EnumTupleTestMultipleValuesPartial: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NoValue(left), Self::NoValue(right)) => left == right,
            (Self::SingleValue(left), Self::SingleValue(right)) => {
                left == right
            },
            (Self::MultipleValues(left), Self::MultipleValues(right)) => {
                left == right
            },
            _ => false,
        }
    }
}

impl Eq for EnumTupleTestPartialState
where
    for<'x> EnumTupleTestSingleValuePartial: Eq,
    for<'x> EnumTupleTestMultipleValuesPartial: Eq,
{
}

impl EnumTupleTestPartialState {
    /// The variant this state is for.
    #[must_use]
    pub const fn variant(&self) -> EnumTupleTestVariants {
        match self {
            Self::NoValue(_) => EnumTupleTestVariants::NoValue,
            Self::SingleValue(_) => EnumTupleTestVariants::SingleValue,
            Self::MultipleValues(_) => EnumTupleTestVariants::MultipleValues,
        }
    }
}

// ------------------------------------------------------------------ per enum

/// An [`EnumTupleTest`] whose variant may not be chosen yet, and whose fields
/// may or may not be set.
#[derive(Debug)]
pub struct EnumTupleTestPartial {
    state: Option<EnumTupleTestPartialState>,
}

impl Clone for EnumTupleTestPartial
where
    for<'x> EnumTupleTestPartialState: Clone,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl PartialEq for EnumTupleTestPartial
where
    for<'x> EnumTupleTestPartialState: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl Eq for EnumTupleTestPartial where for<'x> EnumTupleTestPartialState: Eq {}

impl EnumTupleTestPartial {
    /// Creates a partial with no variant chosen.
    #[must_use]
    pub const fn new() -> Self {
        Self { state: None }
    }

    /// The chosen variant, if any.
    #[must_use]
    pub const fn variant(&self) -> Option<EnumTupleTestVariants> {
        match &self.state {
            Some(state) => Some(state.variant()),
            None => None,
        }
    }

    /// The chosen variant and its partial, if any.
    #[must_use]
    pub const fn state(&self) -> Option<&EnumTupleTestPartialState> {
        self.state.as_ref()
    }

    #[must_use]
    pub const fn state_mut(
        &mut self,
    ) -> Option<&mut EnumTupleTestPartialState> {
        self.state.as_mut()
    }

    /// Removes the chosen variant and its partial, leaving none chosen.
    pub const fn take_state(&mut self) -> Option<EnumTupleTestPartialState> {
        self.state.take()
    }

    /// Chooses a variant by name, for code that doesn't know the variants.
    /// Like the variant methods, this keeps the fields if `variant` was
    /// already chosen.
    pub fn set_variant(&mut self, variant: EnumTupleTestVariants) {
        match variant {
            EnumTupleTestVariants::NoValue => {
                self.no_value();
            },
            EnumTupleTestVariants::SingleValue => {
                self.single_value();
            },
            EnumTupleTestVariants::MultipleValues => {
                self.multiple_values();
            },
        }
    }

    // ----------------------------------------------------------- per variant

    /// Chooses [`EnumTupleTest::NoValue`] and returns its partial.
    pub fn no_value(&mut self) -> &mut EnumTupleTestNoValuePartial {
        if !matches!(self.state, Some(EnumTupleTestPartialState::NoValue(_))) {
            self.state = Some(EnumTupleTestPartialState::NoValue(
                EnumTupleTestNoValuePartial::new(),
            ));
        }
        match &mut self.state {
            Some(EnumTupleTestPartialState::NoValue(partial)) => partial,
            _ => unreachable!("the variant was just chosen"),
        }
    }

    /// Chooses [`EnumTupleTest::SingleValue`] and returns its partial.
    pub fn single_value(&mut self) -> &mut EnumTupleTestSingleValuePartial {
        if !matches!(
            self.state,
            Some(EnumTupleTestPartialState::SingleValue(_))
        ) {
            self.state = Some(EnumTupleTestPartialState::SingleValue(
                EnumTupleTestSingleValuePartial::new(),
            ));
        }
        match &mut self.state {
            Some(EnumTupleTestPartialState::SingleValue(partial)) => partial,
            _ => unreachable!("the variant was just chosen"),
        }
    }

    /// Chooses [`EnumTupleTest::MultipleValues`] and returns its partial.
    pub fn multiple_values(
        &mut self,
    ) -> &mut EnumTupleTestMultipleValuesPartial {
        if !matches!(
            self.state,
            Some(EnumTupleTestPartialState::MultipleValues(_))
        ) {
            self.state = Some(EnumTupleTestPartialState::MultipleValues(
                EnumTupleTestMultipleValuesPartial::new(),
            ));
        }
        match &mut self.state {
            Some(EnumTupleTestPartialState::MultipleValues(partial)) => partial,
            _ => unreachable!("the variant was just chosen"),
        }
    }
}

impl Default for EnumTupleTestPartial {
    fn default() -> Self {
        Self::new()
    }
}

// ------------------------------------------------------------------ per enum

impl From<EnumTupleTest> for EnumTupleTestPartial {
    fn from(value: EnumTupleTest) -> Self {
        let state = match value {
            EnumTupleTest::NoValue => {
                EnumTupleTestPartialState::NoValue(EnumTupleTestNoValuePartial)
            },
            EnumTupleTest::SingleValue(field_0) => {
                EnumTupleTestPartialState::SingleValue(
                    EnumTupleTestSingleValuePartial(Some(field_0)),
                )
            },
            EnumTupleTest::MultipleValues(field_0, field_1, field_2) => {
                EnumTupleTestPartialState::MultipleValues(
                    EnumTupleTestMultipleValuesPartial(
                        Some(field_0),
                        Some(field_1),
                        Some(field_2),
                    ),
                )
            },
        };
        Self { state: Some(state) }
    }
}

impl From<EnumTupleTestPartialState> for EnumTupleTestPartial {
    fn from(state: EnumTupleTestPartialState) -> Self {
        Self { state: Some(state) }
    }
}

// --------------------------------------------------------------- per variant

impl From<EnumTupleTestNoValuePartial> for EnumTupleTestPartial {
    fn from(partial: EnumTupleTestNoValuePartial) -> Self {
        EnumTupleTestPartialState::NoValue(partial).into()
    }
}

impl From<EnumTupleTestSingleValuePartial> for EnumTupleTestPartial {
    fn from(partial: EnumTupleTestSingleValuePartial) -> Self {
        EnumTupleTestPartialState::SingleValue(partial).into()
    }
}

impl From<EnumTupleTestMultipleValuesPartial> for EnumTupleTestPartial {
    fn from(partial: EnumTupleTestMultipleValuesPartial) -> Self {
        EnumTupleTestPartialState::MultipleValues(partial).into()
    }
}

// ------------------------------------------------------------------ per enum

impl Partly for EnumTupleTest {
    type DefaultValidator = AcceptAll;
    type Field = EnumTupleTestFields;
    type Partial = EnumTupleTestPartial;
}

impl PartialOps for EnumTupleTestPartial {
    type Field = EnumTupleTestFields;
    type Target = EnumTupleTest;

    fn has(&self, field: EnumTupleTestFields) -> bool {
        match &self.state {
            None => false,
            Some(EnumTupleTestPartialState::NoValue(partial)) => {
                partial.has(field)
            },
            Some(EnumTupleTestPartialState::SingleValue(partial)) => {
                partial.has(field)
            },
            Some(EnumTupleTestPartialState::MultipleValues(partial)) => {
                partial.has(field)
            },
        }
    }

    fn required_fields(&self) -> &'static [EnumTupleTestFields] {
        match self.variant() {
            Some(variant) => variant.fields(),
            None => &[],
        }
    }

    /// Where both chose the same variant, its fields are merged. Otherwise,
    /// `other`'s choice wins, if it made one.
    fn merge(self, other: Self) -> Self {
        let state = match (self.state, other.state) {
            (
                Some(EnumTupleTestPartialState::SingleValue(left)),
                Some(EnumTupleTestPartialState::SingleValue(right)),
            ) => {
                Some(EnumTupleTestPartialState::SingleValue(left.merge(right)))
            },
            (
                Some(EnumTupleTestPartialState::MultipleValues(left)),
                Some(EnumTupleTestPartialState::MultipleValues(right)),
            ) => {
                Some(EnumTupleTestPartialState::MultipleValues(
                    left.merge(right),
                ))
            },
            (left, right) => right.or(left),
        };
        Self { state }
    }

    fn is_empty(&self) -> bool {
        self.state.is_none()
    }

    fn is_complete(&self) -> bool {
        self.state.is_some() && self.missing_fields().next().is_none()
    }
}

impl FromIterator<Self> for EnumTupleTestPartial {
    /// Folds partials together. Later partials win.
    fn from_iter<I: IntoIterator<Item = Self>>(iter: I) -> Self {
        iter.into_iter().fold(Self::new(), PartialOps::merge)
    }
}

// --------------------------------------------------------------- per variant

impl PartialOps for EnumTupleTestNoValuePartial {
    type Field = EnumTupleTestFields;
    type Target = EnumTupleTest;

    fn has(&self, _field: EnumTupleTestFields) -> bool {
        false
    }

    fn required_fields(&self) -> &'static [EnumTupleTestFields] {
        EnumTupleTestVariants::NoValue.fields()
    }

    fn merge(self, _other: Self) -> Self {
        self
    }
}

impl PartialOps for EnumTupleTestSingleValuePartial {
    type Field = EnumTupleTestFields;
    type Target = EnumTupleTest;

    fn has(&self, field: EnumTupleTestFields) -> bool {
        match field {
            EnumTupleTestFields::SingleValue0 => self.0.is_some(),
            _ => false,
        }
    }

    fn required_fields(&self) -> &'static [EnumTupleTestFields] {
        EnumTupleTestVariants::SingleValue.fields()
    }

    fn merge(self, other: Self) -> Self {
        Self(other.0.or(self.0))
    }
}

impl PartialOps for EnumTupleTestMultipleValuesPartial {
    type Field = EnumTupleTestFields;
    type Target = EnumTupleTest;

    fn has(&self, field: EnumTupleTestFields) -> bool {
        match field {
            EnumTupleTestFields::MultipleValues0 => self.0.is_some(),
            EnumTupleTestFields::MultipleValues1 => self.1.is_some(),
            EnumTupleTestFields::MultipleValues2 => self.2.is_some(),
            _ => false,
        }
    }

    fn required_fields(&self) -> &'static [EnumTupleTestFields] {
        EnumTupleTestVariants::MultipleValues.fields()
    }

    fn merge(self, other: Self) -> Self {
        Self(other.0.or(self.0), other.1.or(self.1), other.2.or(self.2))
    }
}

// ------------------------------------------------------------------ per enum

/// Decides which variant an [`EnumTupleTest`] gets, and the final value of
/// each field.
pub trait EnumTupleTestBuildStrategy {
    type Error: From<MissingVariant> + From<MissingField<EnumTupleTestFields>>;

    /// Returns `true` if the strategy expects to resolve every field.
    fn can_build(&self, partial: &EnumTupleTestPartial) -> bool {
        partial.is_complete()
    }

    /// Chooses a variant when the partial has none. Its fields are then
    /// resolved like any others.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_variant(
        &self,
    ) -> Result<EnumTupleTestPartialState, Self::Error> {
        Err(MissingVariant.into())
    }

    /// Produces the final field `0` of `SingleValue`.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_single_value_0(
        &self,
        value: Option<u32>,
    ) -> Result<u32, Self::Error> {
        value.ok_or_else(|| {
            MissingField(EnumTupleTestFields::SingleValue0).into()
        })
    }

    /// Produces the final field `0` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_multiple_values_0(
        &self,
        value: Option<u32>,
    ) -> Result<u32, Self::Error> {
        value.ok_or_else(|| {
            MissingField(EnumTupleTestFields::MultipleValues0).into()
        })
    }

    /// Produces the final field `1` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_multiple_values_1(
        &self,
        value: Option<u32>,
    ) -> Result<u32, Self::Error> {
        value.ok_or_else(|| {
            MissingField(EnumTupleTestFields::MultipleValues1).into()
        })
    }

    /// Produces the final field `2` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_multiple_values_2(
        &self,
        value: Option<String>,
    ) -> Result<String, Self::Error> {
        value.ok_or_else(|| {
            MissingField(EnumTupleTestFields::MultipleValues2).into()
        })
    }
}

impl EnumTupleTestBuildStrategy for DefaultBuildStrategy {
    type Error = MissingValue<EnumTupleTestFields>;
}

// ------------------------------------------------------------------ per enum

/// Decides whether values of [`EnumTupleTest`] are valid.
pub trait EnumTupleTestValidator {
    type Error;

    /// Checks a value for field `0` of `SingleValue`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_single_value_0(&self, _value: &u32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks a value for field `0` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_multiple_values_0(
        &self,
        _value: &u32,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks a value for field `1` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_multiple_values_1(
        &self,
        _value: &u32,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks a value for field `2` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_multiple_values_2(
        &self,
        _value: &String,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks the whole value, after every field has passed.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate(&self, _value: &EnumTupleTest) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl EnumTupleTestValidator for AcceptAll {
    type Error = Infallible;
}

/// Swaps the fields set on `input` into `target`, if both are the same variant.
/// Calling it a second time undoes it. Returns `false` if the variants differ.
fn __partly_swap_enum_tuple_test(
    target: &mut EnumTupleTest,
    input: &mut EnumTupleTestPartial,
) -> bool {
    match (target, &mut input.state) {
        (_, None)
        | (
            EnumTupleTest::NoValue,
            Some(EnumTupleTestPartialState::NoValue(_)),
        ) => true,
        (
            EnumTupleTest::SingleValue(field_0),
            Some(EnumTupleTestPartialState::SingleValue(partial)),
        ) => {
            if let Some(value) = &mut partial.0 {
                std::mem::swap(field_0, value);
            }
            true
        },
        (
            EnumTupleTest::MultipleValues(field_0, field_1, field_2),
            Some(EnumTupleTestPartialState::MultipleValues(partial)),
        ) => {
            if let Some(value) = &mut partial.0 {
                std::mem::swap(field_0, value);
            }
            if let Some(value) = &mut partial.1 {
                std::mem::swap(field_1, value);
            }
            if let Some(value) = &mut partial.2 {
                std::mem::swap(field_2, value);
            }
            true
        },
        _ => false,
    }
}

impl<V: EnumTupleTestValidator> ValidateWith<V> for EnumTupleTest {
    type ApplyError = BuildFailure<
        MissingField<EnumTupleTestFields>,
        ValidationErrors<EnumTupleTestFields, V::Error>,
    >;
    type Error = V::Error;

    fn validate_with(
        &self,
        validator: &V,
    ) -> Result<(), ValidationErrors<EnumTupleTestFields, V::Error>> {
        let mut errors = Vec::new();
        match self {
            Self::NoValue => {},
            Self::SingleValue(field_0) => {
                if let Err(error) = validator.validate_single_value_0(field_0) {
                    errors.push(FieldError {
                        field: EnumTupleTestFields::SingleValue0,
                        error,
                    });
                }
            },
            Self::MultipleValues(field_0, field_1, field_2) => {
                if let Err(error) =
                    validator.validate_multiple_values_0(field_0)
                {
                    errors.push(FieldError {
                        field: EnumTupleTestFields::MultipleValues0,
                        error,
                    });
                }
                if let Err(error) =
                    validator.validate_multiple_values_1(field_1)
                {
                    errors.push(FieldError {
                        field: EnumTupleTestFields::MultipleValues1,
                        error,
                    });
                }
                if let Err(error) =
                    validator.validate_multiple_values_2(field_2)
                {
                    errors.push(FieldError {
                        field: EnumTupleTestFields::MultipleValues2,
                        error,
                    });
                }
            },
        }
        if !errors.is_empty() {
            return Err(ValidationErrors::Fields(errors));
        }
        validator.validate(self).map_err(ValidationErrors::Struct)
    }

    fn validate_partial_with(
        partial: &EnumTupleTestPartial,
        validator: &V,
    ) -> Result<(), ValidationErrors<EnumTupleTestFields, V::Error>> {
        let mut errors = Vec::new();
        match &partial.state {
            None | Some(EnumTupleTestPartialState::NoValue(_)) => {},
            Some(EnumTupleTestPartialState::SingleValue(partial)) => {
                if let Some(Err(error)) = partial
                    .0
                    .as_ref()
                    .map(|value| validator.validate_single_value_0(value))
                {
                    errors.push(FieldError {
                        field: EnumTupleTestFields::SingleValue0,
                        error,
                    });
                }
            },
            Some(EnumTupleTestPartialState::MultipleValues(partial)) => {
                if let Some(Err(error)) = partial
                    .0
                    .as_ref()
                    .map(|value| validator.validate_multiple_values_0(value))
                {
                    errors.push(FieldError {
                        field: EnumTupleTestFields::MultipleValues0,
                        error,
                    });
                }
                if let Some(Err(error)) = partial
                    .1
                    .as_ref()
                    .map(|value| validator.validate_multiple_values_1(value))
                {
                    errors.push(FieldError {
                        field: EnumTupleTestFields::MultipleValues1,
                        error,
                    });
                }
                if let Some(Err(error)) = partial
                    .2
                    .as_ref()
                    .map(|value| validator.validate_multiple_values_2(value))
                {
                    errors.push(FieldError {
                        field: EnumTupleTestFields::MultipleValues2,
                        error,
                    });
                }
            },
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors::Fields(errors))
        }
    }

    /// A partial for the current variant writes the fields it sets. A partial
    /// for another variant must be complete, and replaces the value.
    fn apply_partial_with(
        &mut self,
        mut input: EnumTupleTestPartial,
        validator: &V,
    ) -> Result<(), Rejected<EnumTupleTestPartial, Self::ApplyError>> {
        if let Err(error) = Self::validate_partial_with(&input, validator) {
            return Err(Rejected {
                value: input,
                error: BuildFailure::Validation(error),
            });
        }

        if __partly_swap_enum_tuple_test(self, &mut input) {
            if let Err(error) = validator.validate(self) {
                __partly_swap_enum_tuple_test(self, &mut input);
                return Err(Rejected {
                    value: input,
                    error: BuildFailure::Validation(ValidationErrors::Struct(
                        error,
                    )),
                });
            }
            return Ok(());
        }

        let missing = input.missing_fields().next();
        if let Some(field) = missing {
            return Err(Rejected {
                value: input,
                error: BuildFailure::Strategy(MissingField(field)),
            });
        }
        let new = match input.state {
            Some(EnumTupleTestPartialState::NoValue(_)) => Self::NoValue,
            Some(EnumTupleTestPartialState::SingleValue(
                EnumTupleTestSingleValuePartial(Some(field_0)),
            )) => Self::SingleValue(field_0),
            Some(EnumTupleTestPartialState::MultipleValues(
                EnumTupleTestMultipleValuesPartial(
                    Some(field_0),
                    Some(field_1),
                    Some(field_2),
                ),
            )) => Self::MultipleValues(field_0, field_1, field_2),
            _ => unreachable!("the partial is complete"),
        };
        let old = std::mem::replace(self, new);
        if let Err(error) = validator.validate(self) {
            let new = std::mem::replace(self, old);
            return Err(Rejected {
                value: new.into(),
                error: BuildFailure::Validation(ValidationErrors::Struct(
                    error,
                )),
            });
        }
        Ok(())
    }
}

// --------------------------------------------------------------- per variant

impl<S: EnumTupleTestBuildStrategy, V: EnumTupleTestValidator> BuildWith<S, V>
    for EnumTupleTestNoValuePartial
{
    type Error =
        BuildFailure<S::Error, ValidationErrors<EnumTupleTestFields, V::Error>>;
    type Output = EnumTupleTest;

    fn build_with(
        self,
        _strategy: &S,
        validator: &V,
    ) -> Result<EnumTupleTest, Rejected<Self, Self::Error>> {
        let built = EnumTupleTest::NoValue;
        match built.validate_with(validator) {
            Ok(()) => Ok(built),
            Err(error) => {
                Err(Rejected {
                    value: self,
                    error: BuildFailure::Validation(error),
                })
            },
        }
    }
}

impl<S: EnumTupleTestBuildStrategy, V: EnumTupleTestValidator> BuildWith<S, V>
    for EnumTupleTestSingleValuePartial
{
    type Error =
        BuildFailure<S::Error, ValidationErrors<EnumTupleTestFields, V::Error>>;
    type Output = EnumTupleTest;

    fn build_with(
        mut self,
        strategy: &S,
        validator: &V,
    ) -> Result<EnumTupleTest, Rejected<Self, Self::Error>> {
        let field_0 = match strategy.resolve_single_value_0(self.0.take()) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let built = EnumTupleTest::SingleValue(field_0);
        match built.validate_with(validator) {
            Ok(()) => Ok(built),
            Err(error) => {
                let EnumTupleTest::SingleValue(field_0) = built else {
                    unreachable!("built as this variant")
                };
                Err(Rejected {
                    value: Self(Some(field_0)),
                    error: BuildFailure::Validation(error),
                })
            },
        }
    }
}

impl<S: EnumTupleTestBuildStrategy, V: EnumTupleTestValidator> BuildWith<S, V>
    for EnumTupleTestMultipleValuesPartial
{
    type Error =
        BuildFailure<S::Error, ValidationErrors<EnumTupleTestFields, V::Error>>;
    type Output = EnumTupleTest;

    fn build_with(
        mut self,
        strategy: &S,
        validator: &V,
    ) -> Result<EnumTupleTest, Rejected<Self, Self::Error>> {
        let field_0 = match strategy.resolve_multiple_values_0(self.0.take()) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let field_1 = match strategy.resolve_multiple_values_1(self.1.take()) {
            Ok(value) => value,
            Err(error) => {
                self.0 = Some(field_0);
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let field_2 = match strategy.resolve_multiple_values_2(self.2.take()) {
            Ok(value) => value,
            Err(error) => {
                self.0 = Some(field_0);
                self.1 = Some(field_1);
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let built = EnumTupleTest::MultipleValues(field_0, field_1, field_2);
        match built.validate_with(validator) {
            Ok(()) => Ok(built),
            Err(error) => {
                let EnumTupleTest::MultipleValues(field_0, field_1, field_2) =
                    built
                else {
                    unreachable!("built as this variant")
                };
                Err(Rejected {
                    value: Self(Some(field_0), Some(field_1), Some(field_2)),
                    error: BuildFailure::Validation(error),
                })
            },
        }
    }
}

// ------------------------------------------------------------------ per enum

impl<S: EnumTupleTestBuildStrategy, V: EnumTupleTestValidator> BuildWith<S, V>
    for EnumTupleTestPartial
{
    type Error =
        BuildFailure<S::Error, ValidationErrors<EnumTupleTestFields, V::Error>>;
    type Output = EnumTupleTest;

    fn build_with(
        self,
        strategy: &S,
        validator: &V,
    ) -> Result<EnumTupleTest, Rejected<Self, Self::Error>> {
        let state = match self.state {
            Some(state) => state,
            None => {
                match strategy.resolve_variant() {
                    Ok(state) => state,
                    Err(error) => {
                        return Err(Rejected {
                            value: Self::new(),
                            error: BuildFailure::Strategy(error),
                        });
                    },
                }
            },
        };
        match state {
            EnumTupleTestPartialState::NoValue(partial) => {
                partial
                    .build_with(strategy, validator)
                    .map_err(|rejected| rejected.map_value(Self::from))
            },
            EnumTupleTestPartialState::SingleValue(partial) => {
                partial
                    .build_with(strategy, validator)
                    .map_err(|rejected| rejected.map_value(Self::from))
            },
            EnumTupleTestPartialState::MultipleValues(partial) => {
                partial
                    .build_with(strategy, validator)
                    .map_err(|rejected| rejected.map_value(Self::from))
            },
        }
    }
}

impl Build for EnumTupleTestPartial {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = EnumTupleTest;

    fn can_build(&self) -> bool {
        // Fully qualified: other derives in this module implement their own
        // strategy trait for `DefaultBuildStrategy`.
        EnumTupleTestBuildStrategy::can_build(&DefaultBuildStrategy, self)
    }

    fn build(self) -> Result<EnumTupleTest, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

// --------------------------------------------------------------- per variant

impl Build for EnumTupleTestNoValuePartial {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = EnumTupleTest;

    fn can_build(&self) -> bool {
        self.is_complete()
    }

    fn build(self) -> Result<EnumTupleTest, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

impl Build for EnumTupleTestSingleValuePartial {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = EnumTupleTest;

    fn can_build(&self) -> bool {
        self.is_complete()
    }

    fn build(self) -> Result<EnumTupleTest, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

impl Build for EnumTupleTestMultipleValuesPartial {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = EnumTupleTest;

    fn can_build(&self) -> bool {
        self.is_complete()
    }

    fn build(self) -> Result<EnumTupleTest, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

// ------------------------------------------------------------------ per enum
// Reflection.

/// An owned field of [`EnumTupleTest`] and its value.
pub enum EnumTupleTestFieldValue {
    SingleValue0(u32),
    MultipleValues0(u32),
    MultipleValues1(u32),
    MultipleValues2(String),
}

impl std::fmt::Debug for EnumTupleTestFieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SingleValue0(value) => {
                f.debug_tuple("SingleValue0")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
            Self::MultipleValues0(value) => {
                f.debug_tuple("MultipleValues0")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
            Self::MultipleValues1(value) => {
                f.debug_tuple("MultipleValues1")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
            Self::MultipleValues2(value) => {
                f.debug_tuple("MultipleValues2")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
        }
    }
}

impl Clone for EnumTupleTestFieldValue
where
    for<'x> u32: Clone,
    for<'x> String: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::SingleValue0(value) => Self::SingleValue0(value.clone()),
            Self::MultipleValues0(value) => {
                Self::MultipleValues0(value.clone())
            },
            Self::MultipleValues1(value) => {
                Self::MultipleValues1(value.clone())
            },
            Self::MultipleValues2(value) => {
                Self::MultipleValues2(value.clone())
            },
        }
    }
}

impl PartialEq for EnumTupleTestFieldValue
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SingleValue0(left), Self::SingleValue0(right))
            | (Self::MultipleValues0(left), Self::MultipleValues0(right))
            | (Self::MultipleValues1(left), Self::MultipleValues1(right)) => {
                left == right
            },
            (Self::MultipleValues2(left), Self::MultipleValues2(right)) => {
                left == right
            },
            _ => false,
        }
    }
}

impl EnumTupleTestFieldValue {
    /// The field this value belongs to.
    #[must_use]
    pub const fn field(&self) -> EnumTupleTestFields {
        match self {
            Self::SingleValue0(_) => EnumTupleTestFields::SingleValue0,
            Self::MultipleValues0(_) => EnumTupleTestFields::MultipleValues0,
            Self::MultipleValues1(_) => EnumTupleTestFields::MultipleValues1,
            Self::MultipleValues2(_) => EnumTupleTestFields::MultipleValues2,
        }
    }
}

/// A borrowed field of [`EnumTupleTest`] and its value.
#[derive(Clone, Copy)]
pub enum EnumTupleTestFieldRef<'a> {
    SingleValue0(&'a u32),
    MultipleValues0(&'a u32),
    MultipleValues1(&'a u32),
    MultipleValues2(&'a String),
}

impl std::fmt::Debug for EnumTupleTestFieldRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SingleValue0(value) => {
                f.debug_tuple("SingleValue0")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
            Self::MultipleValues0(value) => {
                f.debug_tuple("MultipleValues0")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
            Self::MultipleValues1(value) => {
                f.debug_tuple("MultipleValues1")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
            Self::MultipleValues2(value) => {
                f.debug_tuple("MultipleValues2")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
        }
    }
}

impl PartialEq for EnumTupleTestFieldRef<'_>
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SingleValue0(left), Self::SingleValue0(right))
            | (Self::MultipleValues0(left), Self::MultipleValues0(right))
            | (Self::MultipleValues1(left), Self::MultipleValues1(right)) => {
                left == right
            },
            (Self::MultipleValues2(left), Self::MultipleValues2(right)) => {
                left == right
            },
            _ => false,
        }
    }
}

impl Reflect for EnumTupleTestPartial {
    type Value = EnumTupleTestFieldValue;
    type ValueRef<'a> = EnumTupleTestFieldRef<'a>;

    fn iter(&self) -> impl Iterator<Item = EnumTupleTestFieldRef<'_>> {
        let values: [Option<EnumTupleTestFieldRef<'_>>;
            ENUM_TUPLE_TEST_MAX_FIELDS] = match &self.state {
            None | Some(EnumTupleTestPartialState::NoValue(_)) => {
                [None, None, None]
            },
            Some(EnumTupleTestPartialState::SingleValue(partial)) => {
                [
                    partial.0.as_ref().map(EnumTupleTestFieldRef::SingleValue0),
                    None,
                    None,
                ]
            },
            Some(EnumTupleTestPartialState::MultipleValues(partial)) => {
                [
                    partial
                        .0
                        .as_ref()
                        .map(EnumTupleTestFieldRef::MultipleValues0),
                    partial
                        .1
                        .as_ref()
                        .map(EnumTupleTestFieldRef::MultipleValues1),
                    partial
                        .2
                        .as_ref()
                        .map(EnumTupleTestFieldRef::MultipleValues2),
                ]
            },
        };
        values.into_iter().flatten()
    }

    /// Switches to the value's variant first, if another one is chosen.
    fn set_field(
        &mut self,
        value: EnumTupleTestFieldValue,
    ) -> Option<EnumTupleTestFieldValue> {
        match value {
            EnumTupleTestFieldValue::SingleValue0(value) => {
                self.single_value()
                    .0
                    .replace(value)
                    .map(EnumTupleTestFieldValue::SingleValue0)
            },
            EnumTupleTestFieldValue::MultipleValues0(value) => {
                self.multiple_values()
                    .0
                    .replace(value)
                    .map(EnumTupleTestFieldValue::MultipleValues0)
            },
            EnumTupleTestFieldValue::MultipleValues1(value) => {
                self.multiple_values()
                    .1
                    .replace(value)
                    .map(EnumTupleTestFieldValue::MultipleValues1)
            },
            EnumTupleTestFieldValue::MultipleValues2(value) => {
                self.multiple_values()
                    .2
                    .replace(value)
                    .map(EnumTupleTestFieldValue::MultipleValues2)
            },
        }
    }

    /// Returns `None` for a field of a variant that isn't chosen.
    fn take_field(
        &mut self,
        field: EnumTupleTestFields,
    ) -> Option<EnumTupleTestFieldValue> {
        match (&mut self.state, field) {
            (
                Some(EnumTupleTestPartialState::SingleValue(partial)),
                EnumTupleTestFields::SingleValue0,
            ) => partial.0.take().map(EnumTupleTestFieldValue::SingleValue0),
            (
                Some(EnumTupleTestPartialState::MultipleValues(partial)),
                EnumTupleTestFields::MultipleValues0,
            ) => {
                partial
                    .0
                    .take()
                    .map(EnumTupleTestFieldValue::MultipleValues0)
            },
            (
                Some(EnumTupleTestPartialState::MultipleValues(partial)),
                EnumTupleTestFields::MultipleValues1,
            ) => {
                partial
                    .1
                    .take()
                    .map(EnumTupleTestFieldValue::MultipleValues1)
            },
            (
                Some(EnumTupleTestPartialState::MultipleValues(partial)),
                EnumTupleTestFields::MultipleValues2,
            ) => {
                partial
                    .2
                    .take()
                    .map(EnumTupleTestFieldValue::MultipleValues2)
            },
            _ => None,
        }
    }
}

impl IntoIterator for EnumTupleTestPartial {
    type IntoIter = std::iter::Flatten<
        std::array::IntoIter<
            Option<EnumTupleTestFieldValue>,
            ENUM_TUPLE_TEST_MAX_FIELDS,
        >,
    >;
    type Item = EnumTupleTestFieldValue;

    /// Moves out the fields. The chosen variant is lost, so a unit variant
    /// yields nothing.
    fn into_iter(self) -> Self::IntoIter {
        let values = match self.state {
            None | Some(EnumTupleTestPartialState::NoValue(_)) => {
                [None, None, None]
            },
            Some(EnumTupleTestPartialState::SingleValue(partial)) => {
                [
                    partial.0.map(EnumTupleTestFieldValue::SingleValue0),
                    None,
                    None,
                ]
            },
            Some(EnumTupleTestPartialState::MultipleValues(partial)) => {
                [
                    partial.0.map(EnumTupleTestFieldValue::MultipleValues0),
                    partial.1.map(EnumTupleTestFieldValue::MultipleValues1),
                    partial.2.map(EnumTupleTestFieldValue::MultipleValues2),
                ]
            },
        };
        values.into_iter().flatten()
    }
}

impl Extend<EnumTupleTestFieldValue> for EnumTupleTestPartial {
    fn extend<I: IntoIterator<Item = EnumTupleTestFieldValue>>(
        &mut self,
        iter: I,
    ) {
        for value in iter {
            self.set_field(value);
        }
    }
}

impl FromIterator<EnumTupleTestFieldValue> for EnumTupleTestPartial {
    fn from_iter<I: IntoIterator<Item = EnumTupleTestFieldValue>>(
        iter: I,
    ) -> Self {
        let mut partial = Self::new();
        partial.extend(iter);
        partial
    }
}

// ------------------------------------------------------------------ per enum
// The patch.

/// Copies an [`EnumTupleTest`] field by field.
fn __partly_clone_enum_tuple_test(value: &EnumTupleTest) -> EnumTupleTest
where
    for<'x> u32: Clone,
    for<'x> String: Clone,
{
    match value {
        EnumTupleTest::NoValue => EnumTupleTest::NoValue,
        EnumTupleTest::SingleValue(field_0) => {
            EnumTupleTest::SingleValue(field_0.clone())
        },
        EnumTupleTest::MultipleValues(field_0, field_1, field_2) => {
            EnumTupleTest::MultipleValues(
                field_0.clone(),
                field_1.clone(),
                field_2.clone(),
            )
        },
    }
}

/// Lists how `target` differs from `old`: the variant, or else each field.
fn __partly_compare_enum_tuple_test(
    old: &EnumTupleTest,
    target: &EnumTupleTest,
) -> Vec<EnumConflict<EnumTupleTestFields>>
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    let mut conflicts = Vec::new();
    match (old, target) {
        (EnumTupleTest::NoValue, EnumTupleTest::NoValue) => {},
        (
            EnumTupleTest::SingleValue(old_0),
            EnumTupleTest::SingleValue(target_0),
        ) => {
            if old_0 != target_0 {
                conflicts.push(EnumConflict::Field(
                    EnumTupleTestFields::SingleValue0,
                ));
            }
        },
        (
            EnumTupleTest::MultipleValues(old_0, old_1, old_2),
            EnumTupleTest::MultipleValues(target_0, target_1, target_2),
        ) => {
            if old_0 != target_0 {
                conflicts.push(EnumConflict::Field(
                    EnumTupleTestFields::MultipleValues0,
                ));
            }
            if old_1 != target_1 {
                conflicts.push(EnumConflict::Field(
                    EnumTupleTestFields::MultipleValues1,
                ));
            }
            if old_2 != target_2 {
                conflicts.push(EnumConflict::Field(
                    EnumTupleTestFields::MultipleValues2,
                ));
            }
        },
        _ => conflicts.push(EnumConflict::Variant),
    }
    conflicts
}

/// Finds where two partials write something different: another variant, or
/// a field both write.
fn __partly_compare_writes_enum_tuple_test(
    left: &EnumTupleTestPartial,
    right: &EnumTupleTestPartial,
) -> Option<EnumConflict<EnumTupleTestFields>>
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    match (&left.state, &right.state) {
        (None, _)
        | (_, None)
        | (
            Some(EnumTupleTestPartialState::NoValue(_)),
            Some(EnumTupleTestPartialState::NoValue(_)),
        ) => None,
        (
            Some(EnumTupleTestPartialState::SingleValue(left)),
            Some(EnumTupleTestPartialState::SingleValue(right)),
        ) => {
            matches!((&left.0, &right.0), (Some(a), Some(b)) if a != b)
                .then_some(EnumConflict::Field(
                    EnumTupleTestFields::SingleValue0,
                ))
        },
        (
            Some(EnumTupleTestPartialState::MultipleValues(left)),
            Some(EnumTupleTestPartialState::MultipleValues(right)),
        ) => {
            if matches!((&left.0, &right.0), (Some(a), Some(b)) if a != b) {
                Some(EnumConflict::Field(EnumTupleTestFields::MultipleValues0))
            } else if matches!((&left.1, &right.1), (Some(a), Some(b)) if a != b)
            {
                Some(EnumConflict::Field(EnumTupleTestFields::MultipleValues1))
            } else if matches!((&left.2, &right.2), (Some(a), Some(b)) if a != b)
            {
                Some(EnumConflict::Field(EnumTupleTestFields::MultipleValues2))
            } else {
                None
            }
        },
        _ => Some(EnumConflict::Variant),
    }
}

/// The value `inner` leaves: `old` with the fields written or, for another
/// variant, the value built from `new` with the default strategy.
fn __partly_patched_enum_tuple_test(
    inner: &EnumTupleTestPatchInnerData,
) -> Result<EnumTupleTest, MissingValue<EnumTupleTestFields>>
where
    for<'x> u32: Clone,
    for<'x> String: Clone,
{
    let mut value = __partly_clone_enum_tuple_test(&inner.old);
    let mut new = inner.new.clone();
    if __partly_swap_enum_tuple_test(&mut value, &mut new) {
        return Ok(value);
    }
    match new.build() {
        Ok(value) => Ok(value),
        Err(rejected) => {
            match rejected.error {
                BuildFailure::Strategy(missing) => Err(missing),
                BuildFailure::Validation(_) => {
                    unreachable!("`AcceptAll` accepts every value")
                },
            }
        },
    }
}

/// The value an [`EnumTupleTestPatch`] was made against, and what it writes.
pub struct EnumTupleTestPatchInnerData {
    old: EnumTupleTest,
    new: EnumTupleTestPartial,
}

impl std::fmt::Debug for EnumTupleTestPatchInnerData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnumTupleTestPatchInnerData")
            .field("old", &partly::__debug_value!(&self.old))
            .field("new", &self.new)
            .finish()
    }
}

impl Clone for EnumTupleTestPatchInnerData
where
    for<'x> u32: Clone,
    for<'x> String: Clone,
{
    fn clone(&self) -> Self {
        Self {
            old: __partly_clone_enum_tuple_test(&self.old),
            new: self.new.clone(),
        }
    }
}

impl PartialEq for EnumTupleTestPatchInnerData
where
    for<'x> u32: PartialEq,
    for<'x> String: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        __partly_compare_enum_tuple_test(&self.old, &other.old).is_empty()
            && self.new == other.new
    }
}

impl Eq for EnumTupleTestPatchInnerData
where
    for<'x> u32: Eq,
    for<'x> String: Eq,
{
}

/// Changes to an [`EnumTupleTest`], made against a complete value of it.
#[derive(Debug)]
pub struct EnumTupleTestPatch {
    inner: Option<EnumTupleTestPatchInnerData>,
}

impl Clone for EnumTupleTestPatch
where
    for<'x> EnumTupleTestPatchInnerData: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl PartialEq for EnumTupleTestPatch
where
    for<'x> EnumTupleTestPatchInnerData: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for EnumTupleTestPatch where for<'x> EnumTupleTestPatchInnerData: Eq {}

impl EnumTupleTestPatch {
    /// Creates an empty patch, which applies to any value.
    #[must_use]
    pub const fn new() -> Self {
        Self { inner: None }
    }

    /// Creates a patch made against `old`, which writes `new`.
    #[must_use]
    pub const fn from_parts(
        old: EnumTupleTest,
        new: EnumTupleTestPartial,
    ) -> Self {
        Self {
            inner: Some(EnumTupleTestPatchInnerData { old, new }),
        }
    }

    /// The value the patch was made against and what it writes, unless it is
    /// empty.
    #[must_use]
    pub fn into_parts(self) -> Option<(EnumTupleTest, EnumTupleTestPartial)> {
        self.inner.map(|inner| (inner.old, inner.new))
    }

    /// The value the patch was made against, unless it is empty.
    #[must_use]
    pub const fn old_value(&self) -> Option<&EnumTupleTest> {
        match &self.inner {
            Some(inner) => Some(&inner.old),
            None => None,
        }
    }

    /// What the patch writes, unless it is empty.
    #[must_use]
    pub const fn new_value(&self) -> Option<&EnumTupleTestPartial> {
        match &self.inner {
            Some(inner) => Some(&inner.new),
            None => None,
        }
    }

    #[must_use]
    pub const fn new_value_mut(&mut self) -> Option<&mut EnumTupleTestPartial> {
        match &mut self.inner {
            Some(inner) => Some(&mut inner.new),
            None => None,
        }
    }

    /// Returns `true` if the patch writes a variant.
    #[must_use]
    pub fn has_variant(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|inner| inner.new.variant().is_some())
    }

    /// Returns `true` if the patch writes another variant than the one it was
    /// made against.
    #[must_use]
    pub fn changes_variant(&self) -> bool {
        self.inner.as_ref().is_some_and(|inner| {
            inner.new.variant().is_some_and(|variant| {
                variant != EnumTupleTestVariants::from(&inner.old)
            })
        })
    }
}

impl Default for EnumTupleTestPatch {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchOps for EnumTupleTestPatch
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type BuildError = MissingValue<EnumTupleTestFields>;
    type Conflict = EnumConflict<EnumTupleTestFields>;
    type Target = EnumTupleTest;

    fn fields(&self) -> &'static [EnumTupleTestFields] {
        match self.inner.as_ref().and_then(|inner| inner.new.variant()) {
            Some(variant) => variant.fields(),
            None => &[],
        }
    }

    fn has(&self, field: EnumTupleTestFields) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|inner| inner.new.has(field))
    }

    fn is_empty(&self) -> bool {
        self.inner.as_ref().is_none_or(|inner| inner.new.is_empty())
    }

    fn is_complete(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|inner| inner.new.is_complete())
    }

    fn is_noop(&self) -> bool {
        self.inner.as_ref().is_none_or(|inner| {
            match (&inner.old, &inner.new.state) {
                (_, None)
                | (
                    EnumTupleTest::NoValue,
                    Some(EnumTupleTestPartialState::NoValue(_)),
                ) => true,
                (
                    EnumTupleTest::SingleValue(field_0),
                    Some(EnumTupleTestPartialState::SingleValue(partial)),
                ) => partial.0.as_ref().is_none_or(|new| new == field_0),
                (
                    EnumTupleTest::MultipleValues(field_0, field_1, field_2),
                    Some(EnumTupleTestPartialState::MultipleValues(partial)),
                ) => {
                    partial.0.as_ref().is_none_or(|new| new == field_0)
                        && partial.1.as_ref().is_none_or(|new| new == field_1)
                        && partial.2.as_ref().is_none_or(|new| new == field_2)
                },
                _ => false,
            }
        })
    }

    fn check(
        &self,
        target: &EnumTupleTest,
    ) -> Result<(), Vec<EnumConflict<EnumTupleTestFields>>> {
        let Some(inner) = &self.inner else {
            return Ok(());
        };
        let conflicts = __partly_compare_enum_tuple_test(&inner.old, target);
        if conflicts.is_empty() {
            Ok(())
        } else {
            Err(conflicts)
        }
    }

    fn invert(
        self,
    ) -> Result<Self, Rejected<Self, MissingValue<EnumTupleTestFields>>> {
        let Some(mut inner) = self.inner else {
            return Ok(Self::new());
        };
        if __partly_swap_enum_tuple_test(&mut inner.old, &mut inner.new) {
            return Ok(Self { inner: Some(inner) });
        }
        match __partly_patched_enum_tuple_test(&inner) {
            Ok(left) => Ok(Self::from_parts(left, inner.old.into())),
            Err(error) => {
                Err(Rejected {
                    value: Self { inner: Some(inner) },
                    error,
                })
            },
        }
    }

    fn into_partial(self) -> EnumTupleTestPartial {
        self.inner
            .map_or_else(EnumTupleTestPartial::new, |inner| inner.new)
    }

    fn then(
        self,
        next: Self,
    ) -> Result<
        Self,
        Rejected<
            (Self, Self),
            ChainFailure<
                EnumConflict<EnumTupleTestFields>,
                MissingValue<EnumTupleTestFields>,
            >,
        >,
    > {
        let failure = match (&self.inner, &next.inner) {
            (Some(first), Some(second)) => {
                match __partly_patched_enum_tuple_test(first) {
                    Ok(left) => {
                        __partly_compare_enum_tuple_test(&second.old, &left)
                            .into_iter()
                            .next()
                            .map(ChainFailure::Conflict)
                    },
                    Err(error) => Some(ChainFailure::Build(error)),
                }
            },
            _ => None,
        };
        if let Some(error) = failure {
            return Err(Rejected {
                value: (self, next),
                error,
            });
        }
        Ok(match (self.inner, next.inner) {
            (Some(first), Some(second)) => {
                Self::from_parts(first.old, first.new.merge(second.new))
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
    ) -> Result<Self, Rejected<(Self, Self), EnumConflict<EnumTupleTestFields>>>
    {
        let conflict = match (&self.inner, &other.inner) {
            (Some(left), Some(right)) => {
                __partly_compare_enum_tuple_test(&left.old, &right.old)
                    .into_iter()
                    .next()
                    .or_else(|| {
                        __partly_compare_writes_enum_tuple_test(
                            &left.new, &right.new,
                        )
                    })
            },
            _ => None,
        };
        if let Some(conflict) = conflict {
            return Err(Rejected {
                value: (self, other),
                error: conflict,
            });
        }
        Ok(match (self.inner, other.inner) {
            (Some(left), Some(right)) => {
                Self::from_parts(left.old, left.new.merge(right.new))
            },
            (left, right) => {
                Self {
                    inner: left.or(right),
                }
            },
        })
    }
}

// ------------------------------------------------------------------ per enum

/// An [`EnumTupleTestPatch`] being built against a borrowed [`EnumTupleTest`].
pub struct BoundEnumTupleTestPatch<'a> {
    target: &'a EnumTupleTest,
    new:    EnumTupleTestPartial,
}

impl std::fmt::Debug for BoundEnumTupleTestPatch<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundEnumTupleTestPatch")
            .field("target", &partly::__debug_value!(self.target))
            .field("new", &self.new)
            .finish()
    }
}

impl Clone for BoundEnumTupleTestPatch<'_>
where
    for<'x> EnumTupleTestPartial: Clone,
{
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            new:    self.new.clone(),
        }
    }
}

impl<'a> BoundEnumTupleTestPatch<'a> {
    /// Starts a patch against `target`.
    #[must_use]
    pub const fn new(target: &'a EnumTupleTest) -> Self {
        Self {
            target,
            new: EnumTupleTestPartial::new(),
        }
    }

    /// What to write so far.
    #[must_use]
    pub const fn new_value(&self) -> &EnumTupleTestPartial {
        &self.new
    }

    #[must_use]
    pub const fn new_value_mut(&mut self) -> &mut EnumTupleTestPartial {
        &mut self.new
    }

    // ----------------------------------------------------------- per variant

    /// Writes [`EnumTupleTest::NoValue`].
    pub fn no_value(&mut self) -> &mut EnumTupleTestNoValuePartial {
        self.new.no_value()
    }

    /// Writes [`EnumTupleTest::SingleValue`], with the fields set on the
    /// returned partial.
    pub fn single_value(&mut self) -> &mut EnumTupleTestSingleValuePartial {
        self.new.single_value()
    }

    /// Writes [`EnumTupleTest::MultipleValues`], with the fields set on the
    /// returned partial.
    pub fn multiple_values(
        &mut self,
    ) -> &mut EnumTupleTestMultipleValuesPartial {
        self.new.multiple_values()
    }
}

impl<'a> BoundPatch<'a> for BoundEnumTupleTestPatch<'a>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type Target = EnumTupleTest;

    fn finish(self) -> EnumTupleTestPatch {
        EnumTupleTestPatch::from_parts(
            __partly_clone_enum_tuple_test(self.target),
            self.new,
        )
    }
}

impl Patchable for EnumTupleTest
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type Bound<'a> = BoundEnumTupleTestPatch<'a>;
    type DefaultMergeStrategy = StrictMerge;
    type Patch = EnumTupleTestPatch;

    fn patch(&self) -> BoundEnumTupleTestPatch<'_> {
        BoundEnumTupleTestPatch::new(self)
    }

    fn diff(&self, new: &Self) -> EnumTupleTestPatch {
        let changes = match (self, new) {
            (Self::NoValue, Self::NoValue) => EnumTupleTestPartial::new(),
            (Self::SingleValue(before_0), Self::SingleValue(after_0)) => {
                let partial = EnumTupleTestSingleValuePartial(
                    (before_0 != after_0).then(|| after_0.clone()),
                );
                if partial.is_empty() {
                    EnumTupleTestPartial::new()
                } else {
                    partial.into()
                }
            },
            (
                Self::MultipleValues(before_0, before_1, before_2),
                Self::MultipleValues(after_0, after_1, after_2),
            ) => {
                let partial = EnumTupleTestMultipleValuesPartial(
                    (before_0 != after_0).then(|| after_0.clone()),
                    (before_1 != after_1).then(|| after_1.clone()),
                    (before_2 != after_2).then(|| after_2.clone()),
                );
                if partial.is_empty() {
                    EnumTupleTestPartial::new()
                } else {
                    partial.into()
                }
            },
            _ => __partly_clone_enum_tuple_test(new).into(),
        };
        EnumTupleTestPatch::from_parts(
            __partly_clone_enum_tuple_test(self),
            changes,
        )
    }
}

// ------------------------------------------------------------------ per enum

/// Settles conflicts when an [`EnumTupleTestPatch`] is applied. See
/// [`EnumStructTestMergeStrategy`] for when each method runs.
pub trait EnumTupleTestMergeStrategy {
    type Error: From<EnumConflict<EnumTupleTestFields>>
        + From<MissingValue<EnumTupleTestFields>>;

    /// Settles a conflict on field `0` of `SingleValue`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_single_value_0(
        &self,
        _current: &u32,
        _old: &u32,
        _new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        Err(EnumConflict::Field(EnumTupleTestFields::SingleValue0).into())
    }

    /// Settles a conflict on field `0` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_multiple_values_0(
        &self,
        _current: &u32,
        _old: &u32,
        _new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        Err(EnumConflict::Field(EnumTupleTestFields::MultipleValues0).into())
    }

    /// Settles a conflict on field `1` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_multiple_values_1(
        &self,
        _current: &u32,
        _old: &u32,
        _new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        Err(EnumConflict::Field(EnumTupleTestFields::MultipleValues1).into())
    }

    /// Settles a conflict on field `2` of `MultipleValues`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_multiple_values_2(
        &self,
        _current: &String,
        _old: &String,
        _new: Option<&String>,
    ) -> Result<Resolution<String>, Self::Error> {
        Err(EnumConflict::Field(EnumTupleTestFields::MultipleValues2).into())
    }

    /// Settles a conflict on the whole value, for the reason in `conflict`.
    /// `UseNew` writes what [`build_new`](Self::build_new) builds from `new`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_variant(
        &self,
        _current: &EnumTupleTest,
        _old: &EnumTupleTest,
        _new: &EnumTupleTestPartial,
        conflict: &VariantConflict<EnumTupleTestFields>,
    ) -> Result<Resolution<EnumTupleTest>, Self::Error> {
        Err(match conflict {
            VariantConflict::OtherVariant => EnumConflict::Variant,
            VariantConflict::Fields(fields) => {
                fields.first().map_or(EnumConflict::Variant, |field| {
                    EnumConflict::Field(*field)
                })
            },
        }
        .into())
    }

    /// Builds the value that replaces the target when the patch writes
    /// another variant.
    ///
    /// # Errors
    ///
    /// By default, builds with the default build strategy, which fails on a
    /// missing field.
    fn build_new(
        &self,
        new: &EnumTupleTestPartial,
    ) -> Result<EnumTupleTest, Self::Error>
    where
        for<'x> EnumTupleTestPartial: Clone,
    {
        match new.clone().build() {
            Ok(value) => Ok(value),
            Err(rejected) => {
                match rejected.error {
                    BuildFailure::Strategy(missing) => Err(missing.into()),
                    BuildFailure::Validation(_) => {
                        unreachable!("`AcceptAll` accepts every value")
                    },
                }
            },
        }
    }
}

impl EnumTupleTestMergeStrategy for StrictMerge {
    type Error = EnumMergeError<EnumTupleTestFields>;
}

impl EnumTupleTestMergeStrategy for FieldMerge {
    type Error = EnumMergeError<EnumTupleTestFields>;

    fn resolve_single_value_0(
        &self,
        _current: &u32,
        _old: &u32,
        new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => {
                Err(EnumConflict::Field(EnumTupleTestFields::SingleValue0)
                    .into())
            },
        }
    }

    fn resolve_multiple_values_0(
        &self,
        _current: &u32,
        _old: &u32,
        new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => {
                Err(EnumConflict::Field(EnumTupleTestFields::MultipleValues0)
                    .into())
            },
        }
    }

    fn resolve_multiple_values_1(
        &self,
        _current: &u32,
        _old: &u32,
        new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => {
                Err(EnumConflict::Field(EnumTupleTestFields::MultipleValues1)
                    .into())
            },
        }
    }

    fn resolve_multiple_values_2(
        &self,
        _current: &String,
        _old: &String,
        new: Option<&String>,
    ) -> Result<Resolution<String>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => {
                Err(EnumConflict::Field(EnumTupleTestFields::MultipleValues2)
                    .into())
            },
        }
    }
}

/// Swaps the values chosen for each field with the target's. Calling it a
/// second time undoes it.
fn __partly_swap_resolved_enum_tuple_test(
    target: &mut EnumTupleTest,
    new: &mut Option<EnumTupleTestPartialState>,
    single_0: &mut Option<Resolution<u32>>,
    multiple: &mut (
        Option<Resolution<u32>>,
        Option<Resolution<u32>>,
        Option<Resolution<String>>,
    ),
) {
    let (new_single_0, new_multiple_0, new_multiple_1, new_multiple_2) =
        match new {
            Some(EnumTupleTestPartialState::SingleValue(partial)) => {
                (partial.0.as_mut(), None, None, None)
            },
            Some(EnumTupleTestPartialState::MultipleValues(partial)) => {
                (
                    None,
                    partial.0.as_mut(),
                    partial.1.as_mut(),
                    partial.2.as_mut(),
                )
            },
            _ => (None, None, None, None),
        };
    match target {
        EnumTupleTest::NoValue => {},
        EnumTupleTest::SingleValue(current_0) => {
            if let Some(resolution) = single_0 {
                resolution.__swap(current_0, new_single_0);
            }
        },
        EnumTupleTest::MultipleValues(current_0, current_1, current_2) => {
            if let Some(resolution) = &mut multiple.0 {
                resolution.__swap(current_0, new_multiple_0);
            }
            if let Some(resolution) = &mut multiple.1 {
                resolution.__swap(current_1, new_multiple_1);
            }
            if let Some(resolution) = &mut multiple.2 {
                resolution.__swap(current_2, new_multiple_2);
            }
        },
    }
}

/// Applies a patch whose variant is the target's and the one it was made
/// against, field by field like a struct patch.
#[expect(clippy::type_complexity, reason = "the bound is the signature")]
fn __partly_apply_fields_enum_tuple_test<
    M: EnumTupleTestMergeStrategy,
    V: EnumTupleTestValidator,
>(
    target: &mut EnumTupleTest,
    mut inner: EnumTupleTestPatchInnerData,
    strategy: &M,
    validator: &V,
) -> Result<
    EnumTupleTestPartial,
    Rejected<
        EnumTupleTestPatch,
        ApplyFailure<M::Error, ValidationErrors<EnumTupleTestFields, V::Error>>,
    >,
>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    // Phase 1: settle every field of the variant. Nothing has changed yet.
    let mut single_0: Option<Resolution<u32>> = None;
    let mut multiple: (
        Option<Resolution<u32>>,
        Option<Resolution<u32>>,
        Option<Resolution<String>>,
    ) = (None, None, None);
    let settled = 'settle: {
        match (&*target, &inner.old, &inner.new.state) {
            (
                EnumTupleTest::SingleValue(current_0),
                EnumTupleTest::SingleValue(old_0),
                new,
            ) => {
                let new = match new {
                    Some(EnumTupleTestPartialState::SingleValue(partial)) => {
                        partial.0.as_ref()
                    },
                    _ => None,
                };
                single_0 = match partly::__private::settle(
                    current_0,
                    old_0,
                    new,
                    |current, old, new| {
                        strategy.resolve_single_value_0(current, old, new)
                    },
                ) {
                    Ok(resolution) => resolution,
                    Err(error) => break 'settle Err(error),
                };
            },
            (
                EnumTupleTest::MultipleValues(current_0, current_1, current_2),
                EnumTupleTest::MultipleValues(old_0, old_1, old_2),
                new,
            ) => {
                let new = match new {
                    Some(EnumTupleTestPartialState::MultipleValues(
                        partial,
                    )) => Some(partial),
                    _ => None,
                };
                multiple.0 = match partly::__private::settle(
                    current_0,
                    old_0,
                    new.and_then(|partial| partial.0.as_ref()),
                    |current, old, new| {
                        strategy.resolve_multiple_values_0(current, old, new)
                    },
                ) {
                    Ok(resolution) => resolution,
                    Err(error) => break 'settle Err(error),
                };
                multiple.1 = match partly::__private::settle(
                    current_1,
                    old_1,
                    new.and_then(|partial| partial.1.as_ref()),
                    |current, old, new| {
                        strategy.resolve_multiple_values_1(current, old, new)
                    },
                ) {
                    Ok(resolution) => resolution,
                    Err(error) => break 'settle Err(error),
                };
                multiple.2 = match partly::__private::settle(
                    current_2,
                    old_2,
                    new.and_then(|partial| partial.2.as_ref()),
                    |current, old, new| {
                        strategy.resolve_multiple_values_2(current, old, new)
                    },
                ) {
                    Ok(resolution) => resolution,
                    Err(error) => break 'settle Err(error),
                };
            },
            _ => {},
        }
        Ok(())
    };
    if let Err(error) = settled {
        return Err(Rejected {
            value: EnumTupleTestPatch { inner: Some(inner) },
            error: ApplyFailure::Merge(error),
        });
    }

    // Phase 2: validate the values that would be written.
    let (new_single_0, new_multiple_0, new_multiple_1, new_multiple_2) =
        match &inner.new.state {
            Some(EnumTupleTestPartialState::SingleValue(partial)) => {
                (partial.0.as_ref(), None, None, None)
            },
            Some(EnumTupleTestPartialState::MultipleValues(partial)) => {
                (
                    None,
                    partial.0.as_ref(),
                    partial.1.as_ref(),
                    partial.2.as_ref(),
                )
            },
            _ => (None, None, None, None),
        };
    let mut errors = Vec::new();
    if let Some(Err(error)) = single_0
        .as_ref()
        .and_then(|resolution| resolution.chosen(new_single_0))
        .map(|value| validator.validate_single_value_0(value))
    {
        errors.push(FieldError {
            field: EnumTupleTestFields::SingleValue0,
            error,
        });
    }
    if let Some(Err(error)) = multiple
        .0
        .as_ref()
        .and_then(|resolution| resolution.chosen(new_multiple_0))
        .map(|value| validator.validate_multiple_values_0(value))
    {
        errors.push(FieldError {
            field: EnumTupleTestFields::MultipleValues0,
            error,
        });
    }
    if let Some(Err(error)) = multiple
        .1
        .as_ref()
        .and_then(|resolution| resolution.chosen(new_multiple_1))
        .map(|value| validator.validate_multiple_values_1(value))
    {
        errors.push(FieldError {
            field: EnumTupleTestFields::MultipleValues1,
            error,
        });
    }
    if let Some(Err(error)) = multiple
        .2
        .as_ref()
        .and_then(|resolution| resolution.chosen(new_multiple_2))
        .map(|value| validator.validate_multiple_values_2(value))
    {
        errors.push(FieldError {
            field: EnumTupleTestFields::MultipleValues2,
            error,
        });
    }
    if !errors.is_empty() {
        return Err(Rejected {
            value: EnumTupleTestPatch { inner: Some(inner) },
            error: ApplyFailure::Validation(ValidationErrors::Fields(errors)),
        });
    }

    // Phase 3: write, and undo if the whole-value check fails.
    __partly_swap_resolved_enum_tuple_test(
        target,
        &mut inner.new.state,
        &mut single_0,
        &mut multiple,
    );
    if let Err(error) = validator.validate(target) {
        __partly_swap_resolved_enum_tuple_test(
            target,
            &mut inner.new.state,
            &mut single_0,
            &mut multiple,
        );
        return Err(Rejected {
            value: EnumTupleTestPatch { inner: Some(inner) },
            error: ApplyFailure::Validation(ValidationErrors::Struct(error)),
        });
    }

    // After the swaps, the patch holds the values that were replaced.
    let (new_single_0, new_multiple_0, new_multiple_1, new_multiple_2) =
        match inner.new.state {
            Some(EnumTupleTestPartialState::SingleValue(partial)) => {
                (partial.0, None, None, None)
            },
            Some(EnumTupleTestPartialState::MultipleValues(partial)) => {
                (None, partial.0, partial.1, partial.2)
            },
            _ => (None, None, None, None),
        };
    Ok(match &*target {
        EnumTupleTest::NoValue => EnumTupleTestPartial::new(),
        EnumTupleTest::SingleValue(..) => {
            let partial = EnumTupleTestSingleValuePartial(
                single_0
                    .and_then(|resolution| resolution.__previous(new_single_0)),
            );
            if partial.is_empty() {
                EnumTupleTestPartial::new()
            } else {
                partial.into()
            }
        },
        EnumTupleTest::MultipleValues(..) => {
            let (resolution_0, resolution_1, resolution_2) = multiple;
            let partial = EnumTupleTestMultipleValuesPartial(
                resolution_0.and_then(|resolution| {
                    resolution.__previous(new_multiple_0)
                }),
                resolution_1.and_then(|resolution| {
                    resolution.__previous(new_multiple_1)
                }),
                resolution_2.and_then(|resolution| {
                    resolution.__previous(new_multiple_2)
                }),
            );
            if partial.is_empty() {
                EnumTupleTestPartial::new()
            } else {
                partial.into()
            }
        },
    })
}

/// Applies a patch that replaces the whole value: it writes another variant,
/// or the target is another variant than it was made against.
#[expect(clippy::type_complexity, reason = "the bound is the signature")]
fn __partly_apply_value_enum_tuple_test<
    M: EnumTupleTestMergeStrategy,
    V: EnumTupleTestValidator,
>(
    target: &mut EnumTupleTest,
    inner: EnumTupleTestPatchInnerData,
    strategy: &M,
    validator: &V,
) -> Result<
    EnumTupleTestPartial,
    Rejected<
        EnumTupleTestPatch,
        ApplyFailure<M::Error, ValidationErrors<EnumTupleTestFields, V::Error>>,
    >,
>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    let conflict = if EnumTupleTestVariants::from(&*target)
        == EnumTupleTestVariants::from(&inner.old)
    {
        let fields: Vec<_> =
            __partly_compare_enum_tuple_test(&inner.old, target)
                .into_iter()
                .filter_map(|conflict| {
                    match conflict {
                        EnumConflict::Field(field) => Some(field),
                        EnumConflict::Variant => None,
                    }
                })
                .collect();
        (!fields.is_empty()).then_some(VariantConflict::Fields(fields))
    } else {
        Some(VariantConflict::OtherVariant)
    };
    let resolution = match &conflict {
        None => Resolution::UseNew,
        Some(conflict) => {
            match strategy
                .resolve_variant(target, &inner.old, &inner.new, conflict)
            {
                Ok(resolution) => resolution,
                Err(error) => {
                    return Err(Rejected {
                        value: EnumTupleTestPatch { inner: Some(inner) },
                        error: ApplyFailure::Merge(error),
                    });
                },
            }
        },
    };
    let mut value = match resolution {
        Resolution::KeepCurrent => return Ok(EnumTupleTestPartial::new()),
        Resolution::UseNew => {
            match strategy.build_new(&inner.new) {
                Ok(value) => value,
                Err(error) => {
                    return Err(Rejected {
                        value: EnumTupleTestPatch { inner: Some(inner) },
                        error: ApplyFailure::Merge(error),
                    });
                },
            }
        },
        Resolution::Use(value) => value,
    };
    if let Err(error) = value.validate_with(validator) {
        return Err(Rejected {
            value: EnumTupleTestPatch { inner: Some(inner) },
            error: ApplyFailure::Validation(error),
        });
    }
    std::mem::swap(target, &mut value);
    Ok(value.into())
}

/// Applies `patch` to `target`, and returns the values it replaced.
#[expect(clippy::type_complexity, reason = "the bound is the signature")]
fn __partly_apply_enum_tuple_test_patch<
    M: EnumTupleTestMergeStrategy,
    V: EnumTupleTestValidator,
>(
    target: &mut EnumTupleTest,
    patch: EnumTupleTestPatch,
    strategy: &M,
    validator: &V,
) -> Result<
    EnumTupleTestPartial,
    Rejected<
        EnumTupleTestPatch,
        ApplyFailure<M::Error, ValidationErrors<EnumTupleTestFields, V::Error>>,
    >,
>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    let Some(inner) = patch.inner else {
        return Ok(EnumTupleTestPartial::new());
    };
    let old_variant = EnumTupleTestVariants::from(&inner.old);
    let same_target = EnumTupleTestVariants::from(&*target) == old_variant;
    let same_write = inner
        .new
        .variant()
        .is_none_or(|variant| variant == old_variant);
    if same_target && same_write {
        __partly_apply_fields_enum_tuple_test(
            target, inner, strategy, validator,
        )
    } else {
        __partly_apply_value_enum_tuple_test(target, inner, strategy, validator)
    }
}

impl<M: EnumTupleTestMergeStrategy, V: EnumTupleTestValidator>
    ApplyPatchWith<M, V> for EnumTupleTest
where
    for<'x> u32: Clone + PartialEq,
    for<'x> String: Clone + PartialEq,
{
    type MergeError = M::Error;

    fn apply_patch_with(
        &mut self,
        patch: EnumTupleTestPatch,
        strategy: &M,
        validator: &V,
    ) -> Result<
        (),
        Rejected<
            EnumTupleTestPatch,
            ApplyFailure<
                M::Error,
                ValidationErrors<EnumTupleTestFields, V::Error>,
            >,
        >,
    > {
        __partly_apply_enum_tuple_test_patch(self, patch, strategy, validator)
            .map(|_replaced| ())
    }

    fn apply_patch_with_get_undo(
        &mut self,
        patch: EnumTupleTestPatch,
        strategy: &M,
        validator: &V,
    ) -> Result<
        EnumTupleTestPatch,
        Rejected<
            EnumTupleTestPatch,
            ApplyFailure<
                M::Error,
                ValidationErrors<EnumTupleTestFields, V::Error>,
            >,
        >,
    > {
        let replaced = __partly_apply_enum_tuple_test_patch(
            self, patch, strategy, validator,
        )?;
        Ok(if replaced.is_empty() {
            EnumTupleTestPatch::new()
        } else {
            EnumTupleTestPatch::from_parts(
                __partly_clone_enum_tuple_test(self),
                replaced,
            )
        })
    }
}
