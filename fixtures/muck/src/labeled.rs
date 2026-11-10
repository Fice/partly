//! The base expansion for a struct with a lifetime and a type parameter. Only
//! the blocks the tests use are written out; the rest follows `foo.rs`.
//!
//! ```ignore
//! #[derive(Muck)]
//! pub struct Labeled<'a, T> {
//!     pub label: &'a str,
//!     pub value: T,
//! }
//! ```

use fieldmuck::{
    Field, FieldOf, Make, MakeField, SetField, Shape, ShapeWith, SlotFor,
    SlotOf, Visit, VisitField, VisitFieldWithValue, VisitWithValue,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labeled<'a, T> {
    pub label: &'a str,
    pub value: T,
}

/// Names a field of [`Labeled`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabeledFields {
    Label,
    Value,
}

impl Field for LabeledFields {
    const ALL: &'static [Self] = &[Self::Label, Self::Value];

    fn name(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::Value => "value",
        }
    }
}

// ---------------------------------------------------------------- per struct
// The field types and the companion carry no generics: the slots do.

/// The fields of [`Labeled`], as types.
pub mod labeled_field {
    /// The field `label`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Label;
    /// The field `value`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Value;
}

/// [`Labeled`] with a slot of any type in place of each field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LabeledShape<L, V> {
    pub label: L,
    pub value: V,
}

impl<T> Shape for Labeled<'_, T> {
    type Fields = LabeledFields;

    const NAME: &'static str = "Labeled";
}

impl<'a, T, K> ShapeWith<K> for Labeled<'a, T>
where
    K: SlotFor<Labeled<'a, T>, labeled_field::Label>
        + SlotFor<Labeled<'a, T>, labeled_field::Value>,
{
    type Out = LabeledShape<
        <K as SlotFor<Labeled<'a, T>, labeled_field::Label>>::Slot,
        <K as SlotFor<Labeled<'a, T>, labeled_field::Value>>::Slot,
    >;
}

impl<'a, T> FieldOf<Labeled<'a, T>> for labeled_field::Label {
    type Type = &'a str;

    const FIELD: LabeledFields = LabeledFields::Label;

    fn get<'v>(value: &'v Labeled<'a, T>) -> &'v &'a str {
        &value.label
    }

    fn get_mut<'v>(value: &'v mut Labeled<'a, T>) -> &'v mut &'a str {
        &mut value.label
    }
}

impl<T> FieldOf<Labeled<'_, T>> for labeled_field::Value {
    type Type = T;

    const FIELD: LabeledFields = LabeledFields::Value;

    fn get<'v>(value: &'v Labeled<'_, T>) -> &'v T {
        &value.value
    }

    fn get_mut<'v>(value: &'v mut Labeled<'_, T>) -> &'v mut T {
        &mut value.value
    }
}

impl<'a, T, K> SlotOf<Labeled<'a, T>, K> for labeled_field::Label
where
    K: SlotFor<Labeled<'a, T>, labeled_field::Label>
        + SlotFor<Labeled<'a, T>, labeled_field::Value>,
{
    // Explicit lifetimes: the struct's own lifetime makes elision ambiguous.
    fn slot<'w>(
        with: &'w <Labeled<'a, T> as ShapeWith<K>>::Out,
    ) -> &'w <K as SlotFor<Labeled<'a, T>, Self>>::Slot {
        &with.label
    }

    fn slot_mut<'w>(
        with: &'w mut <Labeled<'a, T> as ShapeWith<K>>::Out,
    ) -> &'w mut <K as SlotFor<Labeled<'a, T>, Self>>::Slot {
        &mut with.label
    }
}

impl<'a, T, K> SlotOf<Labeled<'a, T>, K> for labeled_field::Value
where
    K: SlotFor<Labeled<'a, T>, labeled_field::Label>
        + SlotFor<Labeled<'a, T>, labeled_field::Value>,
{
    // Explicit lifetimes: the struct's own lifetime makes elision ambiguous.
    fn slot<'w>(
        with: &'w <Labeled<'a, T> as ShapeWith<K>>::Out,
    ) -> &'w <K as SlotFor<Labeled<'a, T>, Self>>::Slot {
        &with.value
    }

    fn slot_mut<'w>(
        with: &'w mut <Labeled<'a, T> as ShapeWith<K>>::Out,
    ) -> &'w mut <K as SlotFor<Labeled<'a, T>, Self>>::Slot {
        &mut with.value
    }
}

impl<'a, T, K, V> Visit<K, V> for Labeled<'a, T>
where
    K: SlotFor<Labeled<'a, T>, labeled_field::Label>
        + SlotFor<Labeled<'a, T>, labeled_field::Value>,
    V: VisitField<Labeled<'a, T>, labeled_field::Label, K>
        + VisitField<Labeled<'a, T>, labeled_field::Value, K>,
{
    fn visit(with: &Self::Out, visitor: &mut V) {
        VisitField::<Labeled<'a, T>, labeled_field::Label, K>::visit(
            visitor,
            &with.label,
        );
        VisitField::<Labeled<'a, T>, labeled_field::Value, K>::visit(
            visitor,
            &with.value,
        );
    }
}

impl<'a, T, K, V> VisitWithValue<K, V> for Labeled<'a, T>
where
    K: SlotFor<Labeled<'a, T>, labeled_field::Label>
        + SlotFor<Labeled<'a, T>, labeled_field::Value>,
    V: VisitFieldWithValue<Labeled<'a, T>, labeled_field::Label, K>
        + VisitFieldWithValue<Labeled<'a, T>, labeled_field::Value, K>,
{
    fn visit_with_value(with: &Self::Out, value: &Self, visitor: &mut V) {
        VisitFieldWithValue::<Labeled<'a, T>, labeled_field::Label, K>::visit(
            visitor,
            &with.label,
            &value.label,
        );
        VisitFieldWithValue::<Labeled<'a, T>, labeled_field::Value, K>::visit(
            visitor,
            &with.value,
            &value.value,
        );
    }
}

impl<'a, T, K, V> Make<K, V> for Labeled<'a, T>
where
    K: SlotFor<Labeled<'a, T>, labeled_field::Label>
        + SlotFor<Labeled<'a, T>, labeled_field::Value>,
    V: MakeField<Labeled<'a, T>, labeled_field::Label, K>
        + MakeField<Labeled<'a, T>, labeled_field::Value, K>,
{
    fn make(maker: &mut V) -> Self::Out {
        LabeledShape {
            label: MakeField::<Labeled<'a, T>, labeled_field::Label, K>::make(
                maker,
            ),
            value: MakeField::<Labeled<'a, T>, labeled_field::Value, K>::make(
                maker,
            ),
        }
    }
}

impl<'a, T> SetField<Labeled<'a, T>, labeled_field::Label> for Labeled<'a, T> {
    fn set_field(&mut self, value: &'a str) {
        self.label = value;
    }
}

impl<'a, T> SetField<Labeled<'a, T>, labeled_field::Value> for Labeled<'a, T> {
    fn set_field(&mut self, value: T) {
        self.value = value;
    }
}

/// Writes the fields of [`Labeled`] by name, to anything that accepts it.
pub trait LabeledSetters<'a, T> {
    fn set_label(&mut self, value: &'a str)
    where
        Self: SetField<Labeled<'a, T>, labeled_field::Label>,
    {
        SetField::<Labeled<'a, T>, labeled_field::Label>::set_field(
            self, value,
        );
    }

    fn set_value(&mut self, value: T)
    where
        Self: SetField<Labeled<'a, T>, labeled_field::Value>,
    {
        SetField::<Labeled<'a, T>, labeled_field::Value>::set_field(
            self, value,
        );
    }
}

impl<'a, T, R: ?Sized> LabeledSetters<'a, T> for R {}
