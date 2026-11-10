//! Turns a [`Filter`] into an SQL condition with parameters.
//!
//! This crate only knows the filter crate and the fieldmuck runtime, not any
//! user type: it shows that a filter can be extended by a third crate. The
//! column of each field is its name.
//!
//! ```ignore
//! let condition = fieldmuck_filter_sql::condition(&filter);
//! assert_eq!(condition.sql(), "bar > ? AND nick IS NULL");
//! ```
#![warn(missing_docs)]

use fieldmuck::{Field, FieldOf, Shape, Visit, VisitField};
use fieldmuck_filter::{
    Comparison, Filter, FilterKind, Filterable, OptionFilter, TextFilter,
};

/// A parameter value.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    /// An integer.
    Integer(i64),
    /// A floating-point number.
    Real(f64),
    /// Text.
    Text(String),
    /// A boolean.
    Bool(bool),
}

/// A value that can be a parameter.
pub trait ToSql {
    /// The parameter value.
    fn to_sql(&self) -> SqlValue;
}

macro_rules! integer_to_sql {
    ($($type:ty),* $(,)?) => {
        $(
            impl ToSql for $type {
                fn to_sql(&self) -> SqlValue {
                    SqlValue::Integer(i64::from(*self))
                }
            }
        )*
    };
}

integer_to_sql!(u8, u16, u32, i8, i16, i32, i64);

impl ToSql for f32 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Real(f64::from(*self))
    }
}

impl ToSql for f64 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Real(*self)
    }
}

impl ToSql for bool {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Bool(*self)
    }
}

impl ToSql for String {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Text(self.clone())
    }
}

impl ToSql for str {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Text(self.to_owned())
    }
}

impl ToSql for usize {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(i64::try_from(*self).unwrap_or(i64::MAX))
    }
}

/// A condition being built: SQL with `?` placeholders, and their values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Condition {
    parts:  Vec<String>,
    params: Vec<SqlValue>,
}

impl Condition {
    /// The SQL, with every part joined by `AND`. `TRUE` if there are none.
    #[must_use]
    pub fn sql(&self) -> String {
        if self.parts.is_empty() {
            "TRUE".to_owned()
        } else {
            self.parts.join(" AND ")
        }
    }

    /// The values of the placeholders, in order.
    #[must_use]
    pub fn params(&self) -> &[SqlValue] {
        &self.params
    }

    /// Adds a part, with the values of its placeholders.
    pub fn push(
        &mut self,
        part: String,
        params: impl IntoIterator<Item = SqlValue>,
    ) {
        self.parts.push(part);
        self.params.extend(params);
    }
}

/// A filter that can be written as SQL on one column.
pub trait SqlFilter {
    /// Adds the condition for `column`.
    fn push_sql(&self, column: &str, condition: &mut Condition);
}

impl<T: ToSql> SqlFilter for Comparison<T> {
    fn push_sql(&self, column: &str, condition: &mut Condition) {
        let (operator, value) = match self {
            Self::Equal(value) => ("=", value),
            Self::NotEqual(value) => ("<>", value),
            Self::Less(value) => ("<", value),
            Self::LessOrEqual(value) => ("<=", value),
            Self::Greater(value) => (">", value),
            Self::GreaterOrEqual(value) => (">=", value),
        };
        condition.push(format!("{column} {operator} ?"), [value.to_sql()]);
    }
}

/// Escapes `%`, `_` and `\` for a `LIKE` pattern.
fn like_escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

impl SqlFilter for TextFilter {
    fn push_sql(&self, column: &str, condition: &mut Condition) {
        let like = |pattern: String| {
            (
                format!("{column} LIKE ? ESCAPE '\\'"),
                SqlValue::Text(pattern),
            )
        };
        let (part, value) = match self {
            Self::Equal(text) => {
                (format!("{column} = ?"), SqlValue::Text(text.clone()))
            },
            Self::Contains(text) => like(format!("%{}%", like_escape(text))),
            Self::StartsWith(text) => like(format!("{}%", like_escape(text))),
            Self::EndsWith(text) => like(format!("%{}", like_escape(text))),
            Self::Length(comparison) => {
                return comparison
                    .push_sql(&format!("LENGTH({column})"), condition);
            },
        };
        condition.push(part, [value]);
    }
}

impl<F: SqlFilter> SqlFilter for OptionFilter<F> {
    fn push_sql(&self, column: &str, condition: &mut Condition) {
        match self {
            Self::IsSet => condition.push(format!("{column} IS NOT NULL"), []),
            Self::IsNotSet => condition.push(format!("{column} IS NULL"), []),
            Self::Value(filter) => filter.push_sql(column, condition),
        }
    }
}

impl SqlFilter for bool {
    fn push_sql(&self, column: &str, condition: &mut Condition) {
        condition.push(format!("{column} = ?"), [SqlValue::Bool(*self)]);
    }
}

impl<S: Shape, F: FieldOf<S>> VisitField<S, F, FilterKind> for Condition
where
    F::Type: Filterable,
    <F::Type as Filterable>::Filter: SqlFilter,
{
    fn visit(&mut self, slot: &Option<<F::Type as Filterable>::Filter>) {
        if let Some(filter) = slot {
            filter.push_sql(F::FIELD.name(), self);
        }
    }
}

/// Writes `filter` as an SQL condition. Fields with no filter are left out.
#[must_use]
pub fn condition<S: Visit<FilterKind, Condition>>(
    filter: &Filter<S>,
) -> Condition {
    let mut condition = Condition::default();
    S::visit(filter, &mut condition);
    condition
}
