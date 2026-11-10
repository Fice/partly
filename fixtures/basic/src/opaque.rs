//! A field type with no common traits, to show that the expansion still
//! compiles and that the operations needing those traits simply don't exist.
//!
//! The `#[derive(Muck)]` half is the same as `foo.rs`, minus the traversals
//! and views `partly` doesn't use here. The `#[derive(Partly)]` half only has
//! the blocks that differ from `foo.rs`.
//!
//! ```ignore
//! #[derive(Muck, Partly)]
//! #[muck(debug = custom)]
//! pub struct Opaque {
//!     pub handle: Handle,
//!     pub count: u32,
//! }
//! ```
//!
//! The partial isn't `Clone`, so it has no `build_cloned`:
//!
//! ```compile_fail,E0277
//! use partly::prelude::*;
//! use partly_fixtures_basic::opaque::OpaquePartial;
//!
//! let partial = OpaquePartial::default();
//! let _ = partial.build_cloned();
//! ```
//!
//! `handle` is patched but isn't `Clone + PartialEq`, so `Opaque` isn't
//! `Patchable` and has no `diff`:
//!
//! ```compile_fail,E0599
//! use partly::prelude::*;
//! use partly_fixtures_basic::opaque::{Handle, Opaque};
//!
//! let opaque = Opaque { handle: Handle, count: 1 };
//! let _ = opaque.diff(&opaque);
//! ```

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use fieldmuck::{
    Accepts, DebugShape, EachField, FieldOf, ForEach, Kind, Make, MakeField,
    Optional, ReadField, ReadSlot, Representation, Shape, ShapeWith, SlotFor,
    SlotOf, SlotType, SlotView, Staged, name_of,
};
use partly::prelude::*;
use partly::{
    AcceptAll, ChangeRecorder, Field, MissingField, PatchField, Patched,
    Rejected, ShapePatch, StrictMerge,
};

/// Implements nothing, on purpose.
pub struct Handle;

pub struct Opaque {
    pub handle: Handle,
    pub count:  u32,
}

// ======================================================= #[derive(Muck)]

// ---------------------------------------------------------------- per struct

/// Names a field of [`Opaque`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpaqueFields {
    Handle,
    Count,
}

impl Field for OpaqueFields {
    const ALL: &'static [Self] = &[Self::Handle, Self::Count];

    fn name(self) -> &'static str {
        match self {
            Self::Handle => "handle",
            Self::Count => "count",
        }
    }
}

/// The fields of [`Opaque`], as types.
pub mod opaque_field {
    /// The field `handle`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Handle;
    /// The field `count`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Count;
}

/// [`Opaque`] with a slot of any type in place of each field, of kind `K`.
///
/// These derives must not cause an error, even though `Handle` has no traits:
/// they only apply to slots that have them.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpaqueShape<H, C, K = ()> {
    pub handle: H,
    pub count: C,
    __kind: PhantomData<K>,
}

impl<H, C, K> OpaqueShape<H, C, K> {
    /// Creates the companion from its slots.
    pub const fn from_slots(handle: H, count: C) -> Self
    where
        K: Accepts<Handle, H> + Accepts<u32, C>,
    {
        Self {
            handle,
            count,
            __kind: PhantomData,
        }
    }
}

impl<H: Default, C: Default, K> Default for OpaqueShape<H, C, K>
where
    K: Accepts<Handle, H> + Accepts<u32, C>,
{
    fn default() -> Self {
        Self::from_slots(H::default(), C::default())
    }
}

impl Shape for Opaque {
    type Fields = OpaqueFields;

    const NAME: &'static str = "Opaque";
}

impl Representation<Opaque> for Opaque {
    const KIND: &'static str = "";
}

impl<H, C, K: Kind> Representation<Opaque> for OpaqueShape<H, C, K> {
    const KIND: &'static str = K::NAME;
}

impl<K> ShapeWith<K> for Opaque
where
    K: SlotFor<Opaque, opaque_field::Handle>
        + SlotFor<Opaque, opaque_field::Count>,
{
    type Out = OpaqueShape<
        <K as SlotFor<Opaque, opaque_field::Handle>>::Slot,
        <K as SlotFor<Opaque, opaque_field::Count>>::Slot,
        K,
    >;
}

// ----------------------------------------------------------------- per field

impl FieldOf<Opaque> for opaque_field::Handle {
    type Type = Handle;

    const FIELD: OpaqueFields = OpaqueFields::Handle;

    fn get(value: &Opaque) -> &Handle {
        &value.handle
    }

    fn get_mut(value: &mut Opaque) -> &mut Handle {
        &mut value.handle
    }
}

impl FieldOf<Opaque> for opaque_field::Count {
    type Type = u32;

    const FIELD: OpaqueFields = OpaqueFields::Count;

    fn get(value: &Opaque) -> &u32 {
        &value.count
    }

    fn get_mut(value: &mut Opaque) -> &mut u32 {
        &mut value.count
    }
}

impl<K> SlotOf<Opaque, K> for opaque_field::Handle
where
    K: SlotFor<Opaque, opaque_field::Handle>
        + SlotFor<Opaque, opaque_field::Count>,
{
    fn slot(
        with: &<Opaque as ShapeWith<K>>::Out,
    ) -> &<K as SlotFor<Opaque, Self>>::Slot {
        &with.handle
    }

    fn slot_mut(
        with: &mut <Opaque as ShapeWith<K>>::Out,
    ) -> &mut <K as SlotFor<Opaque, Self>>::Slot {
        &mut with.handle
    }
}

impl<K> SlotOf<Opaque, K> for opaque_field::Count
where
    K: SlotFor<Opaque, opaque_field::Handle>
        + SlotFor<Opaque, opaque_field::Count>,
{
    fn slot(
        with: &<Opaque as ShapeWith<K>>::Out,
    ) -> &<K as SlotFor<Opaque, Self>>::Slot {
        &with.count
    }

    fn slot_mut(
        with: &mut <Opaque as ShapeWith<K>>::Out,
    ) -> &mut <K as SlotFor<Opaque, Self>>::Slot {
        &mut with.count
    }
}

// ---------------------------------------------------------------- per struct

impl<V> ForEach<V> for Opaque
where
    V: EachField<Opaque, opaque_field::Handle>
        + EachField<Opaque, opaque_field::Count>,
{
    fn for_each(visitor: &mut V) {
        EachField::<Opaque, opaque_field::Handle>::each(visitor);
        EachField::<Opaque, opaque_field::Count>::each(visitor);
    }
}

impl<K, V> Make<K, V> for Opaque
where
    K: SlotFor<Opaque, opaque_field::Handle>
        + SlotFor<Opaque, opaque_field::Count>,
    V: MakeField<Opaque, opaque_field::Handle, K>
        + MakeField<Opaque, opaque_field::Count, K>,
{
    fn make(maker: &mut V) -> Self::Out {
        OpaqueShape {
            handle: MakeField::<Opaque, opaque_field::Handle, K>::make(maker),
            count:  MakeField::<Opaque, opaque_field::Count, K>::make(maker),
            __kind: PhantomData,
        }
    }
}

// ----------------------------------------------------------------- per field

impl ReadField<Opaque, opaque_field::Handle> for Opaque {
    fn read_field(&self) -> Option<&Handle> {
        Some(&self.handle)
    }
}

impl ReadField<Opaque, opaque_field::Count> for Opaque {
    fn read_field(&self) -> Option<&u32> {
        Some(&self.count)
    }
}

impl<H: ReadSlot<Handle>, C, K> ReadField<Opaque, opaque_field::Handle>
    for OpaqueShape<H, C, K>
{
    fn read_field(&self) -> Option<&Handle> {
        self.handle.read_slot()
    }
}

impl<H, C: ReadSlot<u32>, K> ReadField<Opaque, opaque_field::Count>
    for OpaqueShape<H, C, K>
{
    fn read_field(&self) -> Option<&u32> {
        self.count.read_slot()
    }
}

impl<'a, H, C, K> SlotType<'a, Opaque, opaque_field::Handle>
    for OpaqueShape<H, C, K>
{
    type Slot = &'a H;
}

impl<H, C, K> SlotView<Opaque, opaque_field::Handle> for OpaqueShape<H, C, K> {
    fn slot_view(&self) -> &H {
        &self.handle
    }
}

impl<'a, H, C, K> SlotType<'a, Opaque, opaque_field::Count>
    for OpaqueShape<H, C, K>
{
    type Slot = &'a C;
}

impl<H, C, K> SlotView<Opaque, opaque_field::Count> for OpaqueShape<H, C, K> {
    fn slot_view(&self) -> &C {
        &self.count
    }
}

// ---------------------------------------------------------------- per struct

/// Has a slot for every field of [`Opaque`]: the slot view, by name.
pub trait OpaqueSlots:
    SlotView<Opaque, opaque_field::Handle> + SlotView<Opaque, opaque_field::Count>
{
    fn handle_slot(&self) -> opaque_slot::Handle<'_, Self> {
        SlotView::<Opaque, opaque_field::Handle>::slot_view(self)
    }

    fn count_slot(&self) -> opaque_slot::Count<'_, Self> {
        SlotView::<Opaque, opaque_field::Count>::slot_view(self)
    }
}

impl<T> OpaqueSlots for T where
    T: ?Sized
        + SlotView<Opaque, opaque_field::Handle>
        + SlotView<Opaque, opaque_field::Count>
{
}

/// The type of each field's slot in a representation `V`.
pub mod opaque_slot {
    use fieldmuck::SlotType;

    use super::{Opaque, opaque_field};

    /// The slot of `handle` in `V`.
    pub type Handle<'a, V> =
        <V as SlotType<'a, Opaque, opaque_field::Handle>>::Slot;
    /// The slot of `count` in `V`.
    pub type Count<'a, V> =
        <V as SlotType<'a, Opaque, opaque_field::Count>>::Slot;
}

// `#[muck(debug = custom)]`: no default hook. The default would need `Debug`
// on every slot, and so on `Handle`.

impl<H, C, K> Debug for OpaqueShape<H, C, K>
where
    Opaque: DebugShape<Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Opaque as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ============================================================ the user's

/// Prints every representation of an `Opaque` with `handle` as `<opaque>`.
/// Errors hold a partial, so a partial must print even when a field can't.
impl<V> DebugShape<V> for Opaque
where
    V: OpaqueSlots + Representation<Opaque> + ?Sized,
    for<'a> opaque_slot::Count<'a, V>: Debug,
{
    fn fmt_shape(view: &V, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&name_of::<Opaque, V>())
            .field("handle", &format_args!("<opaque>"))
            .field("count", &view.count_slot())
            .finish()
    }
}

// ===================================================== #[derive(Partly)]

// ---------------------------------------------------------------- per struct

/// An [`Opaque`] whose fields may or may not be set.
pub type OpaquePartial<H = Option<Handle>, C = Option<u32>> =
    OpaqueShape<H, C, Optional>;

// ----------------------------------------------------------------- per field

impl<C> OpaquePartial<Option<Handle>, C> {
    pub fn set_handle(&mut self, value: Handle) {
        self.handle = Some(value);
    }
}

impl<H> OpaquePartial<H, Option<u32>> {
    pub fn set_count(&mut self, value: u32) {
        self.count = Some(value);
    }

    #[must_use]
    pub const fn count(&self) -> Option<&u32> {
        self.count.as_ref()
    }
}

// ---------------------------------------------------------------- per struct

impl From<Opaque> for OpaquePartial {
    fn from(value: Opaque) -> Self {
        Self::from_slots(Some(value.handle), Some(value.count))
    }
}

impl Partly for Opaque {
    type DefaultValidator = AcceptAll;
    type Field = OpaqueFields;
    type Partial = OpaquePartial;
}

impl PartialOps for OpaquePartial {
    type Field = OpaqueFields;
    type Target = Opaque;

    fn has(&self, field: OpaqueFields) -> bool {
        match field {
            OpaqueFields::Handle => self.handle.is_some(),
            OpaqueFields::Count => self.count.is_some(),
        }
    }

    fn merge(self, other: Self) -> Self {
        Self::from_slots(
            other.handle.or(self.handle),
            other.count.or(self.count),
        )
    }
}

impl Build for OpaquePartial {
    type Error = MissingField<OpaqueFields>;
    type Output = Opaque;

    fn can_build(&self) -> bool {
        self.is_complete()
    }

    fn build(self) -> Result<Opaque, Rejected<Self, Self::Error>> {
        match (self.handle, self.count) {
            (Some(handle), Some(count)) => Ok(Opaque { handle, count }),
            (handle, count) => {
                let field = if handle.is_none() {
                    OpaqueFields::Handle
                } else {
                    OpaqueFields::Count
                };
                Err(Rejected {
                    value: Self::from_slots(handle, count),
                    error: MissingField(field),
                })
            },
        }
    }
}

// ---------------------------------------------------------------- per struct
// The patch. `handle` is patched, so its mode needs `Clone + PartialEq`. As
// in `foo.rs`, the higher-ranked bound makes the impl absent instead of an
// error, and with it
// everything that needs it: `ShapePatch<Opaque>` names a type that can't be
// used, and `Opaque` isn't `Patchable`. `#[partly(skip(patch))]` on `handle`
// would make it `Skipped` and give `Opaque` patches, as in `bar.rs`.

/// Changes to an [`Opaque`]. Unusable, since `Handle` can't be compared.
pub type OpaquePatch = ShapePatch<Opaque>;

/// An [`OpaquePatch`] being built.
pub type BoundOpaquePatch<'a> = Staged<'a, Opaque, ChangeRecorder<Opaque>>;

impl PatchField<Opaque> for opaque_field::Handle
where
    for<'x> Handle: Clone + PartialEq,
{
    type Mode = Patched;
}

impl PatchField<Opaque> for opaque_field::Count
where
    for<'x> u32: Clone + PartialEq,
{
    type Mode = Patched;
}

impl partly::shape_patch::PatchShape for Opaque {
    const FIELDS: &'static [OpaqueFields] = OpaqueFields::ALL;
}

impl Patchable for Opaque
where
    for<'x> Handle: Clone + PartialEq,
    for<'x> u32: Clone + PartialEq,
{
    type Bound<'a> = BoundOpaquePatch<'a>;
    type DefaultMergeStrategy = StrictMerge;
    type Patch = OpaquePatch;

    fn patch(&self) -> BoundOpaquePatch<'_> {
        Staged::new(self, ChangeRecorder::new())
    }

    fn diff(&self, new: &Self) -> OpaquePatch {
        partly::shape_patch::diff(self, new)
    }
}
