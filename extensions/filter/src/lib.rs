//! Filters for any [`Shape`], written without a macro.
//!
//! [`Filter<S>`] is the companion of `S` where each field's slot holds an
//! optional filter. The filter's type depends on the field's type, through
//! [`Filterable`]: a number gets a [`Comparison`], a string a [`TextFilter`],
//! and an `Option` an [`OptionFilter`] of its inner type.
//!
//! ```ignore
//! let mut filter = fieldmuck_filter::empty::<Foo>();
//! filter.bar = Some(Comparison::Greater(3));
//! filter.nick = Some(OptionFilter::IsNotSet);
//! assert!(fieldmuck_filter::matches(&filter, &foo));
//! ```
#![warn(missing_docs)]

use fieldmuck::{
    Accepts, FieldOf, Kind, Make, MakeField, Shape, SlotFor,
    VisitFieldWithValue, VisitWithValue, With,
};

/// How values of a type are filtered.
///
/// This crate implements it for primitive types, strings and `Option`. A
/// user's type implements it itself. A third-party type needs an impl in this
/// crate, or in the type's crate.
pub trait Filterable {
    /// The filter for values of this type.
    type Filter;

    /// Returns `true` if `value` passes `filter`.
    fn matches(filter: &Self::Filter, value: &Self) -> bool;
}

/// Compares a value with a fixed one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison<T> {
    /// The value equals this one.
    Equal(T),
    /// The value doesn't equal this one.
    NotEqual(T),
    /// The value is less than this one.
    Less(T),
    /// The value is less than or equal to this one.
    LessOrEqual(T),
    /// The value is greater than this one.
    Greater(T),
    /// The value is greater than or equal to this one.
    GreaterOrEqual(T),
}

impl<T: PartialOrd> Comparison<T> {
    /// Returns `true` if `value` passes the comparison.
    #[must_use]
    pub fn matches(&self, value: &T) -> bool {
        match self {
            Self::Equal(other) => value == other,
            Self::NotEqual(other) => value != other,
            Self::Less(other) => value < other,
            Self::LessOrEqual(other) => value <= other,
            Self::Greater(other) => value > other,
            Self::GreaterOrEqual(other) => value >= other,
        }
    }
}

macro_rules! filter_by_comparison {
    ($($type:ty),* $(,)?) => {
        $(
            impl Filterable for $type {
                type Filter = Comparison<$type>;

                fn matches(filter: &Comparison<$type>, value: &$type) -> bool {
                    filter.matches(value)
                }
            }
        )*
    };
}

filter_by_comparison!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64,
    char,
);

/// A `bool` is filtered by the value it must have.
impl Filterable for bool {
    type Filter = Self;

    fn matches(filter: &Self, value: &Self) -> bool {
        filter == value
    }
}

/// Filters text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFilter {
    /// The text equals this one.
    Equal(String),
    /// The text contains this one.
    Contains(String),
    /// The text starts with this one.
    StartsWith(String),
    /// The text ends with this one.
    EndsWith(String),
    /// The text's length in bytes passes the comparison.
    Length(Comparison<usize>),
}

impl TextFilter {
    /// Returns `true` if `value` passes the filter.
    #[must_use]
    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Equal(other) => value == other,
            Self::Contains(other) => value.contains(other.as_str()),
            Self::StartsWith(other) => value.starts_with(other.as_str()),
            Self::EndsWith(other) => value.ends_with(other.as_str()),
            Self::Length(comparison) => comparison.matches(&value.len()),
        }
    }
}

impl Filterable for String {
    type Filter = TextFilter;

    fn matches(filter: &TextFilter, value: &Self) -> bool {
        filter.matches(value)
    }
}

impl Filterable for &str {
    type Filter = TextFilter;

    fn matches(filter: &TextFilter, value: &Self) -> bool {
        filter.matches(value)
    }
}

/// Filters an optional value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionFilter<F> {
    /// There is a value.
    IsSet,
    /// There is no value.
    IsNotSet,
    /// There is a value, and it passes this filter.
    Value(F),
}

impl<T: Filterable> Filterable for Option<T> {
    type Filter = OptionFilter<T::Filter>;

    fn matches(filter: &Self::Filter, value: &Self) -> bool {
        match (filter, value) {
            (OptionFilter::IsSet, value) => value.is_some(),
            (OptionFilter::IsNotSet, value) => value.is_none(),
            (OptionFilter::Value(filter), Some(value)) => {
                T::matches(filter, value)
            },
            (OptionFilter::Value(_), None) => false,
        }
    }
}

/// The filter kind: each field's slot is an optional filter for its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FilterKind;

impl<S: Shape, F: FieldOf<S>> SlotFor<S, F> for FilterKind
where
    F::Type: Filterable,
{
    type Slot = Option<<F::Type as Filterable>::Filter>;
}

impl Kind for FilterKind {
    const NAME: &'static str = "Filter";
}

impl<T: Filterable> Accepts<T, Option<T::Filter>> for FilterKind {}

/// The filter of `S`. A field with no filter matches every value.
pub type Filter<S> = With<S, FilterKind>;

/// Makes a filter with no field filtered.
#[derive(Debug, Clone, Copy, Default)]
pub struct Empty;

impl<S: Shape, F: FieldOf<S>> MakeField<S, F, FilterKind> for Empty
where
    F::Type: Filterable,
{
    fn make(&mut self) -> Option<<F::Type as Filterable>::Filter> {
        None
    }
}

/// Collects whether every field passes its filter.
#[derive(Debug, Clone, Copy)]
pub struct Matches(pub bool);

impl<S: Shape, F: FieldOf<S>> VisitFieldWithValue<S, F, FilterKind> for Matches
where
    F::Type: Filterable,
{
    fn visit(
        &mut self,
        slot: &Option<<F::Type as Filterable>::Filter>,
        value: &F::Type,
    ) {
        if let Some(filter) = slot {
            self.0 &= <F::Type as Filterable>::matches(filter, value);
        }
    }
}

/// Creates a filter that matches every value.
#[must_use]
pub fn empty<S: Make<FilterKind, Empty>>() -> Filter<S> {
    S::make(&mut Empty)
}

/// Returns `true` if `value` passes every field's filter.
#[must_use]
pub fn matches<S: VisitWithValue<FilterKind, Matches>>(
    filter: &Filter<S>,
    value: &S,
) -> bool {
    let mut matches = Matches(true);
    S::visit_with_value(filter, value, &mut matches);
    matches.0
}
