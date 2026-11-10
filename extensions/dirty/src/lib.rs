//! Change tracking for any [`Shape`], written without a macro.
//!
//! [`Dirty`] remembers which fields were written, and holds no values. It
//! accepts writes directly, through [`SetField`], so code that writes fields
//! doesn't need the value itself. It is also an observer, so it can watch a
//! [`Tracked`](fieldmuck::Tracked) value, alone or chained with a
//! [`Recorder`](fieldmuck::Recorder) as a pair. And it can be the buffer of a
//! [`Staged`](fieldmuck::Staged) value.
#![warn(missing_docs)]

use std::fmt::{self, Debug};

use fieldmuck::{
    Accepts, FieldOf, Kind, Make, MakeField, Observe, SetField, Shape,
    ShapeWith, SlotFor, SlotOf, Visit, VisitField, With,
};

// -------------------------------------------------------------------- dirty

/// The kind whose slot says whether the field was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DirtyKind;

impl<S: Shape, F: FieldOf<S>> SlotFor<S, F> for DirtyKind {
    type Slot = bool;
}

impl Kind for DirtyKind {
    const NAME: &'static str = "Dirty";
}

impl<T> Accepts<T, bool> for DirtyKind {}

/// Makes every slot clean.
#[derive(Debug, Clone, Copy, Default)]
pub struct Clean;

impl<S: Shape, F: FieldOf<S>> MakeField<S, F, DirtyKind> for Clean {
    fn make(&mut self) -> bool {
        false
    }
}

/// Collects the fields that were written.
#[derive(Debug)]
pub struct CollectDirty<S: Shape>(pub Vec<S::Fields>);

impl<S: Shape, F: FieldOf<S>> VisitField<S, F, DirtyKind> for CollectDirty<S> {
    fn visit(&mut self, slot: &bool) {
        if *slot {
            self.0.push(F::FIELD);
        }
    }
}

/// Which fields of an `S` were written. Holds no values.
pub struct Dirty<S: ShapeWith<DirtyKind>> {
    flags: With<S, DirtyKind>,
}

impl<S: ShapeWith<DirtyKind>> Debug for Dirty<S>
where
    With<S, DirtyKind>: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Dirty").field(&self.flags).finish()
    }
}

impl<S: ShapeWith<DirtyKind>> Clone for Dirty<S>
where
    With<S, DirtyKind>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            flags: self.flags.clone(),
        }
    }
}

impl<S: Make<DirtyKind, Clean>> Default for Dirty<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: ShapeWith<DirtyKind>> Dirty<S> {
    /// No field written yet.
    #[must_use]
    pub fn new() -> Self
    where
        S: Make<DirtyKind, Clean>,
    {
        Self {
            flags: S::make(&mut Clean),
        }
    }

    /// The flag of each field, as the companion struct.
    #[must_use]
    pub const fn flags(&self) -> &With<S, DirtyKind> {
        &self.flags
    }

    /// Returns `true` if field `F` was written.
    #[must_use]
    pub fn is_dirty<F>(&self) -> bool
    where
        F: SlotOf<S, DirtyKind>,
        DirtyKind: SlotFor<S, F, Slot = bool>,
    {
        *F::slot(&self.flags)
    }

    /// Marks field `F` as written.
    pub fn mark<F>(&mut self)
    where
        F: SlotOf<S, DirtyKind>,
        DirtyKind: SlotFor<S, F, Slot = bool>,
    {
        *F::slot_mut(&mut self.flags) = true;
    }

    /// The fields that were written, in declaration order.
    #[must_use]
    pub fn fields(&self) -> Vec<S::Fields>
    where
        S: Visit<DirtyKind, CollectDirty<S>>,
    {
        let mut collect = CollectDirty(Vec::new());
        S::visit(&self.flags, &mut collect);
        collect.0
    }

    /// Returns `true` if no field was written.
    #[must_use]
    pub fn is_clean(&self) -> bool
    where
        S: Visit<DirtyKind, CollectDirty<S>>,
    {
        self.fields().is_empty()
    }
}

/// Marks a field of a tracked value when it is written.
impl<S, F> Observe<S, F> for Dirty<S>
where
    S: ShapeWith<DirtyKind>,
    F: SlotOf<S, DirtyKind>,
    DirtyKind: SlotFor<S, F, Slot = bool>,
{
    fn observe(&mut self, _old: &F::Type, _new: &F::Type) {
        self.mark::<F>();
    }
}

/// Without a value to write to, a write only marks the field.
impl<S, F> SetField<S, F> for Dirty<S>
where
    S: ShapeWith<DirtyKind>,
    F: SlotOf<S, DirtyKind>,
    DirtyKind: SlotFor<S, F, Slot = bool>,
{
    fn set_field(&mut self, _value: F::Type) {
        self.mark::<F>();
    }
}
