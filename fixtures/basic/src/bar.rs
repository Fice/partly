//! The complete dynamic expansion for a mix of field kinds, built on
//! `fieldmuck`: a plain `Copy` value, a reference, an `Option`, and a type with
//! no traits at all.
//!
//! ```ignore
//! #[derive(Muck, Partly)]
//! #[muck(debug = custom)]
//! pub struct Bar<'a> {
//!     pub count: u32,
//!     pub label: &'a str,
//!     pub nick: Option<String>,
//!     #[partly(skip(patch))]
//!     pub resource: Resource,
//! }
//! ```
//!
//! `skip(patch)` makes `resource` [`Skipped`] in patches: they neither check
//! nor write it, so `Bar` has patches even though `Resource` can't be copied
//! or compared. It is still part of the partial.
//!
//! `Resource` has no `Debug`, so `Bar`'s `Debug` hook is written by hand and
//! prints it as `<opaque>`.
//!
//! The partial isn't `Clone`, so it has no `build_cloned`:
//!
//! ```compile_fail,E0277
//! use partly::prelude::*;
//! use partly_fixtures_basic::bar::BarPartial;
//!
//! let partial = BarPartial::new();
//! let _ = partial.build_cloned();
//! ```
//!
//! `Bar` isn't `Clone`, so it has no `to_partial`:
//!
//! ```compile_fail,E0277
//! use partly::prelude::*;
//! use partly_fixtures_basic::bar::{Bar, Resource};
//!
//! let bar = Bar {
//!     count:    1,
//!     label:    "a",
//!     nick:     None,
//!     resource: Resource(1),
//! };
//! let _ = bar.to_partial();
//! ```
//!
//! A field value holding a `Resource` can't be cloned:
//!
//! ```compile_fail,E0599
//! use partly_fixtures_basic::bar::{BarFieldValue, Resource};
//!
//! let _ = BarFieldValue::Resource(Resource(1)).clone();
//! ```
//!
//! A bound patch can't write the skipped field:
//!
//! ```compile_fail,E0277
//! use partly::prelude::*;
//! use partly_fixtures_basic::bar::bar_prelude::*;
//! use partly_fixtures_basic::bar::{Bar, Resource};
//!
//! let bar = Bar { count: 1, label: "a", nick: None, resource: Resource(1) };
//! let mut bound = bar.patch();
//! bound.set_resource(Resource(2));
//! ```

use std::convert::Infallible;
use std::fmt::{self, Debug};
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
    ShapePatch, Skipped, StrictMerge, ValidateField, ValidateWhole,
    ValidationErrors,
};

/// Has no `Debug`, `Copy`, `Clone` or `PartialEq`, on purpose.
pub struct Resource(pub u32);

pub struct Bar<'a> {
    pub count: u32,
    pub label: &'a str,
    pub nick: Option<String>,
    pub resource: Resource,
}

// ======================================================= #[derive(Muck)]

// ---------------------------------------------------------------- per struct

/// Names a field of [`Bar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarFields {
    Count,
    Label,
    Nick,
    Resource,
}

impl Field for BarFields {
    const ALL: &'static [Self] =
        &[Self::Count, Self::Label, Self::Nick, Self::Resource];

    fn name(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Label => "label",
            Self::Nick => "nick",
            Self::Resource => "resource",
        }
    }
}

/// The fields of [`Bar`], as types. They carry no lifetime: the field of
/// `Bar<'a>` for every `'a`.
pub mod bar_field {
    /// The field `count`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Count;
    /// The field `label`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Label;
    /// The field `nick`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Nick;
    /// The field `resource`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Resource;
}

/// [`Bar`] with a slot of any type in place of each field, of kind `K`. The
/// slots carry the lifetime, so the companion needs none. The kind is private
/// and checked by every constructor.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BarShape<C, L, N, R, K = ()> {
    pub count: C,
    pub label: L,
    pub nick: N,
    pub resource: R,
    __kind: PhantomData<K>,
}

impl<C, L, N, R, K> BarShape<C, L, N, R, K> {
    /// Creates the companion from its slots.
    pub const fn from_slots<'a>(
        count: C,
        label: L,
        nick: N,
        resource: R,
    ) -> Self
    where
        K: Accepts<u32, C>
            + Accepts<&'a str, L>
            + Accepts<Option<String>, N>
            + Accepts<Resource, R>,
    {
        Self {
            count,
            label,
            nick,
            resource,
            __kind: PhantomData,
        }
    }
}

impl<'a, C: Default, L: Default, N: Default, R: Default, K> Default
    for BarShape<C, L, N, R, K>
where
    K: Accepts<u32, C>
        + Accepts<&'a str, L>
        + Accepts<Option<String>, N>
        + Accepts<Resource, R>,
{
    fn default() -> Self {
        Self::from_slots(C::default(), L::default(), N::default(), R::default())
    }
}

impl Shape for Bar<'_> {
    type Fields = BarFields;

    const NAME: &'static str = "Bar";
}

impl<'a> Representation<Bar<'a>> for Bar<'a> {
    const KIND: &'static str = "";
}

impl<'a, C, L, N, R, K: Kind> Representation<Bar<'a>>
    for BarShape<C, L, N, R, K>
{
    const KIND: &'static str = K::NAME;
}

impl<'a, K> ShapeWith<K> for Bar<'a>
where
    K: SlotFor<Bar<'a>, bar_field::Count>
        + SlotFor<Bar<'a>, bar_field::Label>
        + SlotFor<Bar<'a>, bar_field::Nick>
        + SlotFor<Bar<'a>, bar_field::Resource>,
{
    type Out = BarShape<
        <K as SlotFor<Bar<'a>, bar_field::Count>>::Slot,
        <K as SlotFor<Bar<'a>, bar_field::Label>>::Slot,
        <K as SlotFor<Bar<'a>, bar_field::Nick>>::Slot,
        <K as SlotFor<Bar<'a>, bar_field::Resource>>::Slot,
        K,
    >;
}

// ----------------------------------------------------------------- per field
// The struct has a lifetime, so every accessor names its lifetimes.

impl<'a> FieldOf<Bar<'a>> for bar_field::Count {
    type Type = u32;

    const FIELD: BarFields = BarFields::Count;

    fn get<'v>(value: &'v Bar<'a>) -> &'v u32 {
        &value.count
    }

    fn get_mut<'v>(value: &'v mut Bar<'a>) -> &'v mut u32 {
        &mut value.count
    }
}

impl<'a> FieldOf<Bar<'a>> for bar_field::Label {
    type Type = &'a str;

    const FIELD: BarFields = BarFields::Label;

    fn get<'v>(value: &'v Bar<'a>) -> &'v &'a str {
        &value.label
    }

    fn get_mut<'v>(value: &'v mut Bar<'a>) -> &'v mut &'a str {
        &mut value.label
    }
}

impl<'a> FieldOf<Bar<'a>> for bar_field::Nick {
    type Type = Option<String>;

    const FIELD: BarFields = BarFields::Nick;

    fn get<'v>(value: &'v Bar<'a>) -> &'v Option<String> {
        &value.nick
    }

    fn get_mut<'v>(value: &'v mut Bar<'a>) -> &'v mut Option<String> {
        &mut value.nick
    }
}

impl<'a> FieldOf<Bar<'a>> for bar_field::Resource {
    type Type = Resource;

    const FIELD: BarFields = BarFields::Resource;

    fn get<'v>(value: &'v Bar<'a>) -> &'v Resource {
        &value.resource
    }

    fn get_mut<'v>(value: &'v mut Bar<'a>) -> &'v mut Resource {
        &mut value.resource
    }
}

macro_rules! bar_slot_of {
    ($marker:ident, $field:ident) => {
        impl<'a, K> SlotOf<Bar<'a>, K> for bar_field::$marker
        where
            K: SlotFor<Bar<'a>, bar_field::Count>
                + SlotFor<Bar<'a>, bar_field::Label>
                + SlotFor<Bar<'a>, bar_field::Nick>
                + SlotFor<Bar<'a>, bar_field::Resource>,
        {
            fn slot<'w>(
                with: &'w <Bar<'a> as ShapeWith<K>>::Out,
            ) -> &'w <K as SlotFor<Bar<'a>, Self>>::Slot {
                &with.$field
            }

            fn slot_mut<'w>(
                with: &'w mut <Bar<'a> as ShapeWith<K>>::Out,
            ) -> &'w mut <K as SlotFor<Bar<'a>, Self>>::Slot {
                &mut with.$field
            }
        }
    };
}

// A macro here only to keep the fixture short: the derive writes these out.
bar_slot_of!(Count, count);
bar_slot_of!(Label, label);
bar_slot_of!(Nick, nick);
bar_slot_of!(Resource, resource);

// ---------------------------------------------------------------- per struct

impl<'a, V> ForEach<V> for Bar<'a>
where
    V: EachField<Bar<'a>, bar_field::Count>
        + EachField<Bar<'a>, bar_field::Label>
        + EachField<Bar<'a>, bar_field::Nick>
        + EachField<Bar<'a>, bar_field::Resource>,
{
    fn for_each(visitor: &mut V) {
        EachField::<Bar<'a>, bar_field::Count>::each(visitor);
        EachField::<Bar<'a>, bar_field::Label>::each(visitor);
        EachField::<Bar<'a>, bar_field::Nick>::each(visitor);
        EachField::<Bar<'a>, bar_field::Resource>::each(visitor);
    }
}

impl<'a, K, V> Make<K, V> for Bar<'a>
where
    K: SlotFor<Bar<'a>, bar_field::Count>
        + SlotFor<Bar<'a>, bar_field::Label>
        + SlotFor<Bar<'a>, bar_field::Nick>
        + SlotFor<Bar<'a>, bar_field::Resource>,
    V: MakeField<Bar<'a>, bar_field::Count, K>
        + MakeField<Bar<'a>, bar_field::Label, K>
        + MakeField<Bar<'a>, bar_field::Nick, K>
        + MakeField<Bar<'a>, bar_field::Resource, K>,
{
    fn make(maker: &mut V) -> Self::Out {
        BarShape {
            count: MakeField::<Bar<'a>, bar_field::Count, K>::make(maker),
            label: MakeField::<Bar<'a>, bar_field::Label, K>::make(maker),
            nick: MakeField::<Bar<'a>, bar_field::Nick, K>::make(maker),
            resource: MakeField::<Bar<'a>, bar_field::Resource, K>::make(maker),
            __kind: PhantomData,
        }
    }
}

// ----------------------------------------------------------------- per field
// The value view, writes, and the slot view.

macro_rules! bar_views {
    ($marker:ident, $field:ident, $type:ty, $param:ident) => {
        impl<'a> ReadField<Bar<'a>, bar_field::$marker> for Bar<'a> {
            fn read_field(&self) -> Option<&$type> {
                Some(&self.$field)
            }
        }

        impl<'a> SetField<Bar<'a>, bar_field::$marker> for Bar<'a> {
            fn set_field(&mut self, value: $type) {
                self.$field = value;
            }
        }

        impl<'v, 'a> SlotType<'v, Bar<'a>, bar_field::$marker> for Bar<'a> {
            type Slot = &'v $type;
        }

        impl<'a> SlotView<Bar<'a>, bar_field::$marker> for Bar<'a> {
            fn slot_view(&self) -> &$type {
                &self.$field
            }
        }

        impl<'v, 'a, C, L, N, R, K> SlotType<'v, Bar<'a>, bar_field::$marker>
            for BarShape<C, L, N, R, K>
        {
            type Slot = &'v $param;
        }

        impl<'a, C, L, N, R, K> SlotView<Bar<'a>, bar_field::$marker>
            for BarShape<C, L, N, R, K>
        {
            fn slot_view(&self) -> &$param {
                &self.$field
            }
        }
    };
}

bar_views!(Count, count, u32, C);
bar_views!(Label, label, &'a str, L);
bar_views!(Nick, nick, Option<String>, N);
bar_views!(Resource, resource, Resource, R);

impl<'a, C: ReadSlot<u32>, L, N, R, K> ReadField<Bar<'a>, bar_field::Count>
    for BarShape<C, L, N, R, K>
{
    fn read_field(&self) -> Option<&u32> {
        self.count.read_slot()
    }
}

impl<'a, C, L: ReadSlot<&'a str>, N, R, K> ReadField<Bar<'a>, bar_field::Label>
    for BarShape<C, L, N, R, K>
{
    fn read_field(&self) -> Option<&&'a str> {
        self.label.read_slot()
    }
}

impl<'a, C, L, N: ReadSlot<Option<String>>, R, K>
    ReadField<Bar<'a>, bar_field::Nick> for BarShape<C, L, N, R, K>
{
    fn read_field(&self) -> Option<&Option<String>> {
        self.nick.read_slot()
    }
}

impl<'a, C, L, N, R: ReadSlot<Resource>, K>
    ReadField<Bar<'a>, bar_field::Resource> for BarShape<C, L, N, R, K>
{
    fn read_field(&self) -> Option<&Resource> {
        self.resource.read_slot()
    }
}

impl<'a, C: WriteSlot<u32>, L, N, R, K> SetField<Bar<'a>, bar_field::Count>
    for BarShape<C, L, N, R, K>
{
    fn set_field(&mut self, value: u32) {
        self.count.write_slot(value);
    }
}

impl<'a, C, L: WriteSlot<&'a str>, N, R, K> SetField<Bar<'a>, bar_field::Label>
    for BarShape<C, L, N, R, K>
{
    fn set_field(&mut self, value: &'a str) {
        self.label.write_slot(value);
    }
}

impl<'a, C, L, N: WriteSlot<Option<String>>, R, K>
    SetField<Bar<'a>, bar_field::Nick> for BarShape<C, L, N, R, K>
{
    fn set_field(&mut self, value: Option<String>) {
        self.nick.write_slot(value);
    }
}

impl<'a, C, L, N, R: WriteSlot<Resource>, K>
    SetField<Bar<'a>, bar_field::Resource> for BarShape<C, L, N, R, K>
{
    fn set_field(&mut self, value: Resource) {
        self.resource.write_slot(value);
    }
}

// ---------------------------------------------------------------- per struct

/// Writes the fields of [`Bar`] by name, to anything that accepts it.
pub trait BarSetters<'a> {
    fn set_count(&mut self, value: u32)
    where
        Self: SetField<Bar<'a>, bar_field::Count>,
    {
        SetField::<Bar<'a>, bar_field::Count>::set_field(self, value);
    }

    fn set_label(&mut self, value: &'a str)
    where
        Self: SetField<Bar<'a>, bar_field::Label>,
    {
        SetField::<Bar<'a>, bar_field::Label>::set_field(self, value);
    }

    fn set_nick(&mut self, value: Option<String>)
    where
        Self: SetField<Bar<'a>, bar_field::Nick>,
    {
        SetField::<Bar<'a>, bar_field::Nick>::set_field(self, value);
    }

    fn set_resource(&mut self, value: Resource)
    where
        Self: SetField<Bar<'a>, bar_field::Resource>,
    {
        SetField::<Bar<'a>, bar_field::Resource>::set_field(self, value);
    }
}

impl<'a, R: ?Sized> BarSetters<'a> for R {}

/// Has a slot for every field of [`Bar`]: the slot view, by name.
pub trait BarSlots<'a>:
    SlotView<Bar<'a>, bar_field::Count>
    + SlotView<Bar<'a>, bar_field::Label>
    + SlotView<Bar<'a>, bar_field::Nick>
    + SlotView<Bar<'a>, bar_field::Resource>
{
    fn count_slot(&self) -> bar_slot::Count<'_, 'a, Self> {
        SlotView::<Bar<'a>, bar_field::Count>::slot_view(self)
    }

    fn label_slot(&self) -> bar_slot::Label<'_, 'a, Self> {
        SlotView::<Bar<'a>, bar_field::Label>::slot_view(self)
    }

    fn nick_slot(&self) -> bar_slot::Nick<'_, 'a, Self> {
        SlotView::<Bar<'a>, bar_field::Nick>::slot_view(self)
    }

    fn resource_slot(&self) -> bar_slot::Resource<'_, 'a, Self> {
        SlotView::<Bar<'a>, bar_field::Resource>::slot_view(self)
    }
}

impl<'a, T> BarSlots<'a> for T where
    T: ?Sized
        + SlotView<Bar<'a>, bar_field::Count>
        + SlotView<Bar<'a>, bar_field::Label>
        + SlotView<Bar<'a>, bar_field::Nick>
        + SlotView<Bar<'a>, bar_field::Resource>
{
}

/// The type of each field's slot in a representation `V` of `Bar<'a>`.
pub mod bar_slot {
    use fieldmuck::SlotType;

    use super::{Bar, bar_field};

    /// The slot of `count`.
    pub type Count<'v, 'a, V> =
        <V as SlotType<'v, Bar<'a>, bar_field::Count>>::Slot;
    /// The slot of `label`.
    pub type Label<'v, 'a, V> =
        <V as SlotType<'v, Bar<'a>, bar_field::Label>>::Slot;
    /// The slot of `nick`.
    pub type Nick<'v, 'a, V> =
        <V as SlotType<'v, Bar<'a>, bar_field::Nick>>::Slot;
    /// The slot of `resource`.
    pub type Resource<'v, 'a, V> =
        <V as SlotType<'v, Bar<'a>, bar_field::Resource>>::Slot;
}

// `#[muck(debug = custom)]`: no default hook. Every representation's `Debug`
// defers to the user's.

impl<'a, C, L, N, R, K> Debug for BarShape<C, L, N, R, K>
where
    Bar<'a>: DebugShape<Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Bar<'a> as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ============================================================ the user's

/// Prints every representation of a `Bar` with `resource` as `<opaque>`: its
/// type has no `Debug`, and a slot the hook doesn't print needs none.
impl<'a, V> DebugShape<V> for Bar<'a>
where
    V: BarSlots<'a> + Representation<Bar<'a>> + ?Sized,
    for<'v> bar_slot::Count<'v, 'a, V>: Debug,
    for<'v> bar_slot::Label<'v, 'a, V>: Debug,
    for<'v> bar_slot::Nick<'v, 'a, V>: Debug,
{
    fn fmt_shape(view: &V, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&name_of::<Bar<'a>, V>())
            .field("count", &view.count_slot())
            .field("label", &view.label_slot())
            .field("nick", &view.nick_slot())
            .field("resource", &format_args!("<opaque>"))
            .finish()
    }
}

// ===================================================== #[derive(Partly)]

// ---------------------------------------------------------------- per struct
// Reflection, as before.

/// An owned field of [`Bar`] and its value.
pub enum BarFieldValue<'a> {
    Count(u32),
    Label(&'a str),
    Nick(Option<String>),
    Resource(Resource),
}

impl BarFieldValue<'_> {
    /// The field this value belongs to.
    #[must_use]
    pub const fn field(&self) -> BarFields {
        match self {
            Self::Count(_) => BarFields::Count,
            Self::Label(_) => BarFields::Label,
            Self::Nick(_) => BarFields::Nick,
            Self::Resource(_) => BarFields::Resource,
        }
    }
}

impl Debug for BarFieldValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Count(value) => {
                f.debug_tuple("Count")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
            Self::Label(value) => {
                f.debug_tuple("Label")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
            Self::Nick(value) => {
                f.debug_tuple("Nick")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
            Self::Resource(value) => {
                f.debug_tuple("Resource")
                    .field(&partly::__debug_value!(value))
                    .finish()
            },
        }
    }
}

impl<'a> Clone for BarFieldValue<'a>
where
    for<'x> u32: Clone,
    for<'x> &'a str: Clone,
    for<'x> Option<String>: Clone,
    for<'x> Resource: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Count(value) => Self::Count(value.clone()),
            Self::Label(value) => Self::Label(value.clone()),
            Self::Nick(value) => Self::Nick(value.clone()),
            Self::Resource(value) => Self::Resource(value.clone()),
        }
    }
}

impl<'a> PartialEq for BarFieldValue<'a>
where
    for<'x> u32: PartialEq,
    for<'x> &'a str: PartialEq,
    for<'x> Option<String>: PartialEq,
    for<'x> Resource: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Count(left), Self::Count(right)) => left == right,
            (Self::Label(left), Self::Label(right)) => left == right,
            (Self::Nick(left), Self::Nick(right)) => left == right,
            (Self::Resource(left), Self::Resource(right)) => left == right,
            _ => false,
        }
    }
}

/// A borrowed field of [`Bar`] and its value.
#[derive(Clone, Copy)]
pub enum BarFieldRef<'r, 'a> {
    Count(&'r u32),
    Label(&'r &'a str),
    Nick(&'r Option<String>),
    Resource(&'r Resource),
}

impl Debug for BarFieldRef<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Count(value) => {
                f.debug_tuple("Count")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
            Self::Label(value) => {
                f.debug_tuple("Label")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
            Self::Nick(value) => {
                f.debug_tuple("Nick")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
            Self::Resource(value) => {
                f.debug_tuple("Resource")
                    .field(&partly::__debug_value!(*value))
                    .finish()
            },
        }
    }
}

// ---------------------------------------------------------------- per struct
// The partial is the `Optional` instance of the shape. The lifetime comes
// first, because the slot parameters have defaults.

/// A [`Bar`] whose fields may or may not be set.
pub type BarPartial<
    'a,
    C = Option<u32>,
    L = Option<&'a str>,
    N = Option<Option<String>>,
    R = Option<Resource>,
> = BarShape<C, L, N, R, Optional>;

/// A partial with every field known to be set.
pub type BarCompletePartial<'a> =
    BarPartial<'a, Set<u32>, Set<&'a str>, Set<Option<String>>, Set<Resource>>;

impl BarPartial<'_> {
    /// Creates an empty partial.
    #[must_use]
    pub const fn new() -> Self {
        Self::from_slots(None, None, None, None)
    }
}

// ----------------------------------------------------------------- per field
// Accessors on the dynamic slot, the typed getter, and `require_*`, which
// builds the result directly: `self` is already valid, and `Set` is a
// partial slot.

impl<L, N, R> BarPartial<'_, Option<u32>, L, N, R> {
    #[must_use]
    pub const fn count(&self) -> Option<&u32> {
        self.count.as_ref()
    }

    #[must_use]
    pub const fn count_mut(&mut self) -> Option<&mut u32> {
        self.count.as_mut()
    }

    #[must_use]
    pub const fn has_count(&self) -> bool {
        self.count.is_some()
    }

    pub fn set_count(&mut self, value: u32) {
        self.count = Some(value);
    }

    #[must_use]
    pub fn with_count(mut self, value: u32) -> Self {
        self.count = Some(value);
        self
    }

    pub const fn take_count(&mut self) -> Option<u32> {
        self.count.take()
    }

    pub fn clear_count(&mut self) {
        self.count = None;
    }

    /// Moves `count` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if `count` isn't set.
    pub fn require_count(
        self,
    ) -> Result<BarShape<Set<u32>, L, N, R, Optional>, Self> {
        match self.count {
            Some(value) => {
                Ok(BarShape {
                    count: Set(value),
                    label: self.label,
                    nick: self.nick,
                    resource: self.resource,
                    __kind: PhantomData,
                })
            },
            None => Err(self),
        }
    }
}

impl<L, N, R> BarShape<Set<u32>, L, N, R, Optional> {
    #[must_use]
    pub const fn count(&self) -> &u32 {
        &self.count.0
    }
}

impl<'a, C, N, R> BarPartial<'a, C, Option<&'a str>, N, R> {
    #[must_use]
    pub const fn label(&self) -> Option<&&'a str> {
        self.label.as_ref()
    }

    #[must_use]
    pub const fn label_mut(&mut self) -> Option<&mut &'a str> {
        self.label.as_mut()
    }

    #[must_use]
    pub const fn has_label(&self) -> bool {
        self.label.is_some()
    }

    pub fn set_label(&mut self, value: &'a str) {
        self.label = Some(value);
    }

    #[must_use]
    pub fn with_label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }

    pub const fn take_label(&mut self) -> Option<&'a str> {
        self.label.take()
    }

    pub fn clear_label(&mut self) {
        self.label = None;
    }

    /// Moves `label` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if `label` isn't set.
    pub fn require_label(
        self,
    ) -> Result<BarShape<C, Set<&'a str>, N, R, Optional>, Self> {
        match self.label {
            Some(value) => {
                Ok(BarShape {
                    count: self.count,
                    label: Set(value),
                    nick: self.nick,
                    resource: self.resource,
                    __kind: PhantomData,
                })
            },
            None => Err(self),
        }
    }
}

impl<'a, C, N, R> BarShape<C, Set<&'a str>, N, R, Optional> {
    #[must_use]
    pub const fn label(&self) -> &&'a str {
        &self.label.0
    }
}

impl<C, L, R> BarPartial<'_, C, L, Option<Option<String>>, R> {
    #[must_use]
    pub const fn nick(&self) -> Option<&Option<String>> {
        self.nick.as_ref()
    }

    #[must_use]
    pub const fn nick_mut(&mut self) -> Option<&mut Option<String>> {
        self.nick.as_mut()
    }

    #[must_use]
    pub const fn has_nick(&self) -> bool {
        self.nick.is_some()
    }

    /// Sets `nick`. `set_nick(None)` sets it to `None`; `clear_nick` unsets
    /// it.
    pub fn set_nick(&mut self, value: Option<String>) {
        self.nick = Some(value);
    }

    #[must_use]
    pub fn with_nick(mut self, value: Option<String>) -> Self {
        self.nick = Some(value);
        self
    }

    pub const fn take_nick(&mut self) -> Option<Option<String>> {
        self.nick.take()
    }

    pub fn clear_nick(&mut self) {
        self.nick = None;
    }

    /// Moves `nick` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if `nick` isn't set.
    pub fn require_nick(
        self,
    ) -> Result<BarShape<C, L, Set<Option<String>>, R, Optional>, Self> {
        match self.nick {
            Some(value) => {
                Ok(BarShape {
                    count: self.count,
                    label: self.label,
                    nick: Set(value),
                    resource: self.resource,
                    __kind: PhantomData,
                })
            },
            None => Err(self),
        }
    }
}

impl<C, L, R> BarShape<C, L, Set<Option<String>>, R, Optional> {
    #[must_use]
    pub const fn nick(&self) -> &Option<String> {
        &self.nick.0
    }
}

impl<C, L, N> BarPartial<'_, C, L, N, Option<Resource>> {
    #[must_use]
    pub const fn resource(&self) -> Option<&Resource> {
        self.resource.as_ref()
    }

    #[must_use]
    pub const fn resource_mut(&mut self) -> Option<&mut Resource> {
        self.resource.as_mut()
    }

    #[must_use]
    pub const fn has_resource(&self) -> bool {
        self.resource.is_some()
    }

    pub fn set_resource(&mut self, value: Resource) {
        self.resource = Some(value);
    }

    #[must_use]
    pub fn with_resource(mut self, value: Resource) -> Self {
        self.resource = Some(value);
        self
    }

    pub fn take_resource(&mut self) -> Option<Resource> {
        self.resource.take()
    }

    pub fn clear_resource(&mut self) {
        self.resource = None;
    }

    /// Moves `resource` from a runtime check to a compile-time guarantee.
    ///
    /// # Errors
    ///
    /// Hands back the partial if `resource` isn't set.
    pub fn require_resource(
        self,
    ) -> Result<BarShape<C, L, N, Set<Resource>, Optional>, Self> {
        match self.resource {
            Some(value) => {
                Ok(BarShape {
                    count: self.count,
                    label: self.label,
                    nick: self.nick,
                    resource: Set(value),
                    __kind: PhantomData,
                })
            },
            None => Err(self),
        }
    }
}

impl<C, L, N> BarShape<C, L, N, Set<Resource>, Optional> {
    #[must_use]
    pub const fn resource(&self) -> &Resource {
        &self.resource.0
    }
}

// ---------------------------------------------------------------- per struct
// Conversions.

impl<'a> From<Bar<'a>> for BarPartial<'a> {
    fn from(value: Bar<'a>) -> Self {
        Self::from_slots(
            Some(value.count),
            Some(value.label),
            Some(value.nick),
            Some(value.resource),
        )
    }
}

impl<'a> From<BarCompletePartial<'a>> for BarPartial<'a> {
    fn from(value: BarCompletePartial<'a>) -> Self {
        Self::from_slots(
            Some(value.count.0),
            Some(value.label.0),
            Some(value.nick.0),
            Some(value.resource.0),
        )
    }
}

// ---------------------------------------------------------------- per struct
// The core traits.

impl<'a> Partly for Bar<'a> {
    type DefaultValidator = AcceptAll;
    type Field = BarFields;
    type Partial = BarPartial<'a>;
}

impl<'a> PartialOps for BarPartial<'a> {
    type Field = BarFields;
    type Target = Bar<'a>;

    fn has(&self, field: BarFields) -> bool {
        match field {
            BarFields::Count => self.count.is_some(),
            BarFields::Label => self.label.is_some(),
            BarFields::Nick => self.nick.is_some(),
            BarFields::Resource => self.resource.is_some(),
        }
    }

    fn merge(self, other: Self) -> Self {
        Self::from_slots(
            other.count.or(self.count),
            other.label.or(self.label),
            other.nick.or(self.nick),
            other.resource.or(self.resource),
        )
    }
}

impl FromIterator<Self> for BarPartial<'_> {
    /// Folds partials together. Later partials win.
    fn from_iter<I: IntoIterator<Item = Self>>(iter: I) -> Self {
        iter.into_iter().fold(Self::new(), PartialOps::merge)
    }
}

// ---------------------------------------------------------------- per struct
// The build strategy, with one method per field.

/// Decides how each field of [`Bar`] gets its final value.
pub trait BarBuildStrategy<'a> {
    type Error: From<MissingField<BarFields>>;

    /// Returns `true` if the strategy expects to resolve every field.
    fn can_build(&self, partial: &BarPartial<'a>) -> bool {
        partial.is_complete()
    }

    /// Produces the final `count`, given the value that was set.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_count(&self, value: Option<u32>) -> Result<u32, Self::Error> {
        value.ok_or_else(|| MissingField(BarFields::Count).into())
    }

    /// Produces the final `label`, given the value that was set.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_label(
        &self,
        value: Option<&'a str>,
    ) -> Result<&'a str, Self::Error> {
        value.ok_or_else(|| MissingField(BarFields::Label).into())
    }

    /// Produces the final `nick`, given the value that was set.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_nick(
        &self,
        value: Option<Option<String>>,
    ) -> Result<Option<String>, Self::Error> {
        value.ok_or_else(|| MissingField(BarFields::Nick).into())
    }

    /// Produces the final `resource`, given the value that was set.
    ///
    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_resource(
        &self,
        value: Option<Resource>,
    ) -> Result<Resource, Self::Error> {
        value.ok_or_else(|| MissingField(BarFields::Resource).into())
    }
}

impl BarBuildStrategy<'_> for DefaultBuildStrategy {
    type Error = MissingField<BarFields>;
}

// ---------------------------------------------------------------- per struct
// The validator, with one method per field, connected to the generic
// `ValidateField` and `ValidateWhole`.

/// Decides whether values of [`Bar`] are valid.
pub trait BarValidator<'a> {
    type Error;

    /// Checks a value for `count`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_count(&self, _value: &u32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks a value for `label`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_label(&self, _value: &&'a str) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks a value for `nick`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_nick(
        &self,
        _value: &Option<String>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks a value for `resource`.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_resource(&self, _value: &Resource) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Checks the whole struct, after every field has passed.
    ///
    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate(&self, _value: &Bar<'a>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl BarValidator<'_> for AcceptAll {
    type Error = Infallible;
}

impl<'a, V: BarValidator<'a>> ValidateField<Bar<'a>, V> for bar_field::Count {
    type Error = V::Error;

    fn validate_field(validator: &V, value: &u32) -> Result<(), V::Error> {
        validator.validate_count(value)
    }
}

impl<'a, V: BarValidator<'a>> ValidateField<Bar<'a>, V> for bar_field::Label {
    type Error = V::Error;

    fn validate_field(validator: &V, value: &&'a str) -> Result<(), V::Error> {
        validator.validate_label(value)
    }
}

impl<'a, V: BarValidator<'a>> ValidateField<Bar<'a>, V> for bar_field::Nick {
    type Error = V::Error;

    fn validate_field(
        validator: &V,
        value: &Option<String>,
    ) -> Result<(), V::Error> {
        validator.validate_nick(value)
    }
}

impl<'a, V: BarValidator<'a>> ValidateField<Bar<'a>, V>
    for bar_field::Resource
{
    type Error = V::Error;

    fn validate_field(validator: &V, value: &Resource) -> Result<(), V::Error> {
        validator.validate_resource(value)
    }
}

impl<'a, V: BarValidator<'a>> ValidateWhole<V> for Bar<'a> {
    type Error = V::Error;

    fn validate_whole(&self, validator: &V) -> Result<(), V::Error> {
        validator.validate(self)
    }
}

impl<'a, V: BarValidator<'a>> ValidateWith<V> for Bar<'a> {
    type ApplyError = ValidationErrors<BarFields, V::Error>;
    type Error = V::Error;

    fn validate_with(
        &self,
        validator: &V,
    ) -> Result<(), ValidationErrors<BarFields, V::Error>> {
        let mut errors = Vec::new();
        if let Err(error) = validator.validate_count(&self.count) {
            errors.push(FieldError {
                field: BarFields::Count,
                error,
            });
        }
        if let Err(error) = validator.validate_label(&self.label) {
            errors.push(FieldError {
                field: BarFields::Label,
                error,
            });
        }
        if let Err(error) = validator.validate_nick(&self.nick) {
            errors.push(FieldError {
                field: BarFields::Nick,
                error,
            });
        }
        if let Err(error) = validator.validate_resource(&self.resource) {
            errors.push(FieldError {
                field: BarFields::Resource,
                error,
            });
        }
        if !errors.is_empty() {
            return Err(ValidationErrors::Fields(errors));
        }
        validator.validate(self).map_err(ValidationErrors::Struct)
    }

    fn validate_partial_with(
        partial: &BarPartial<'a>,
        validator: &V,
    ) -> Result<(), ValidationErrors<BarFields, V::Error>> {
        let mut errors = Vec::new();
        if let Some(Err(error)) = partial
            .count
            .as_ref()
            .map(|value| validator.validate_count(value))
        {
            errors.push(FieldError {
                field: BarFields::Count,
                error,
            });
        }
        if let Some(Err(error)) = partial
            .label
            .as_ref()
            .map(|value| validator.validate_label(value))
        {
            errors.push(FieldError {
                field: BarFields::Label,
                error,
            });
        }
        if let Some(Err(error)) = partial
            .nick
            .as_ref()
            .map(|value| validator.validate_nick(value))
        {
            errors.push(FieldError {
                field: BarFields::Nick,
                error,
            });
        }
        if let Some(Err(error)) = partial
            .resource
            .as_ref()
            .map(|value| validator.validate_resource(value))
        {
            errors.push(FieldError {
                field: BarFields::Resource,
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
        mut input: BarPartial<'a>,
        validator: &V,
    ) -> Result<
        (),
        Rejected<BarPartial<'a>, ValidationErrors<BarFields, V::Error>>,
    > {
        if let Err(error) = Self::validate_partial_with(&input, validator) {
            return Err(Rejected {
                value: input,
                error,
            });
        }
        // Swapping means a rollback needs no `Clone`: swap again.
        if let Some(value) = &mut input.count {
            std::mem::swap(&mut self.count, value);
        }
        if let Some(value) = &mut input.label {
            std::mem::swap(&mut self.label, value);
        }
        if let Some(value) = &mut input.nick {
            std::mem::swap(&mut self.nick, value);
        }
        if let Some(value) = &mut input.resource {
            std::mem::swap(&mut self.resource, value);
        }
        if let Err(error) = validator.validate(self) {
            if let Some(value) = &mut input.count {
                std::mem::swap(&mut self.count, value);
            }
            if let Some(value) = &mut input.label {
                std::mem::swap(&mut self.label, value);
            }
            if let Some(value) = &mut input.nick {
                std::mem::swap(&mut self.nick, value);
            }
            if let Some(value) = &mut input.resource {
                std::mem::swap(&mut self.resource, value);
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
// Building: resolve fields, then validate.

impl<'a, S: BarBuildStrategy<'a>, V: BarValidator<'a>> BuildWith<S, V>
    for BarPartial<'a>
{
    type Error = BuildFailure<S::Error, ValidationErrors<BarFields, V::Error>>;
    type Output = Bar<'a>;

    fn build_with(
        mut self,
        strategy: &S,
        validator: &V,
    ) -> Result<Bar<'a>, Rejected<Self, Self::Error>> {
        let count = match strategy.resolve_count(self.count.take()) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let label = match strategy.resolve_label(self.label.take()) {
            Ok(value) => value,
            Err(error) => {
                self.count = Some(count);
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let nick = match strategy.resolve_nick(self.nick.take()) {
            Ok(value) => value,
            Err(error) => {
                self.count = Some(count);
                self.label = Some(label);
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let resource = match strategy.resolve_resource(self.resource.take()) {
            Ok(value) => value,
            Err(error) => {
                self.count = Some(count);
                self.label = Some(label);
                self.nick = Some(nick);
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let built = Bar {
            count,
            label,
            nick,
            resource,
        };
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

impl<'a> Build for BarPartial<'a> {
    type Error = <Self as BuildWith<DefaultBuildStrategy, AcceptAll>>::Error;
    type Output = Bar<'a>;

    fn can_build(&self) -> bool {
        // Fully qualified: other derives in this module implement their own
        // strategy trait for `DefaultBuildStrategy`.
        BarBuildStrategy::can_build(&DefaultBuildStrategy, self)
    }

    fn build(self) -> Result<Bar<'a>, Rejected<Self, Self::Error>> {
        self.build_with(&DefaultBuildStrategy, &AcceptAll)
    }
}

// ---------------------------------------------------------------- per struct
// Reflection.

impl<'a> Reflect for BarPartial<'a> {
    type Value = BarFieldValue<'a>;
    type ValueRef<'r>
        = BarFieldRef<'r, 'a>
    where
        Self: 'r;

    fn iter(&self) -> impl Iterator<Item = BarFieldRef<'_, 'a>> {
        [
            self.count.as_ref().map(BarFieldRef::Count),
            self.label.as_ref().map(BarFieldRef::Label),
            self.nick.as_ref().map(BarFieldRef::Nick),
            self.resource.as_ref().map(BarFieldRef::Resource),
        ]
        .into_iter()
        .flatten()
    }

    fn set_field(
        &mut self,
        value: BarFieldValue<'a>,
    ) -> Option<BarFieldValue<'a>> {
        match value {
            BarFieldValue::Count(value) => {
                self.count.replace(value).map(BarFieldValue::Count)
            },
            BarFieldValue::Label(value) => {
                self.label.replace(value).map(BarFieldValue::Label)
            },
            BarFieldValue::Nick(value) => {
                self.nick.replace(value).map(BarFieldValue::Nick)
            },
            BarFieldValue::Resource(value) => {
                self.resource.replace(value).map(BarFieldValue::Resource)
            },
        }
    }

    fn take_field(&mut self, field: BarFields) -> Option<BarFieldValue<'a>> {
        match field {
            BarFields::Count => self.count.take().map(BarFieldValue::Count),
            BarFields::Label => self.label.take().map(BarFieldValue::Label),
            BarFields::Nick => self.nick.take().map(BarFieldValue::Nick),
            BarFields::Resource => {
                self.resource.take().map(BarFieldValue::Resource)
            },
        }
    }
}

impl<'a> IntoIterator for BarPartial<'a> {
    type IntoIter =
        std::iter::Flatten<std::array::IntoIter<Option<BarFieldValue<'a>>, 4>>;
    type Item = BarFieldValue<'a>;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.count.map(BarFieldValue::Count),
            self.label.map(BarFieldValue::Label),
            self.nick.map(BarFieldValue::Nick),
            self.resource.map(BarFieldValue::Resource),
        ]
        .into_iter()
        .flatten()
    }
}

impl<'a> Extend<BarFieldValue<'a>> for BarPartial<'a> {
    fn extend<I: IntoIterator<Item = BarFieldValue<'a>>>(&mut self, iter: I) {
        for value in iter {
            Reflect::set_field(self, value);
        }
    }
}

impl<'a> FromIterator<BarFieldValue<'a>> for BarPartial<'a> {
    fn from_iter<I: IntoIterator<Item = BarFieldValue<'a>>>(iter: I) -> Self {
        let mut partial = Self::new();
        partial.extend(iter);
        partial
    }
}

// ---------------------------------------------------------------- per struct
// The patch is generic. `resource` is `Skipped`, so its type needs nothing.

/// Changes to a [`Bar`], made against the value of every patched field.
pub type BarPatch<'a> = ShapePatch<Bar<'a>>;

/// A [`BarPatch`] being built: writes are staged over a borrowed [`Bar`].
pub type BoundBarPatch<'b, 'a> = Staged<'b, Bar<'a>, ChangeRecorder<Bar<'a>>>;

impl<'a> PatchField<Bar<'a>> for bar_field::Count
where
    for<'x> u32: Clone + PartialEq,
{
    type Mode = Patched;
}

impl<'a> PatchField<Bar<'a>> for bar_field::Label
where
    for<'x> &'a str: Clone + PartialEq,
{
    type Mode = Patched;
}

impl<'a> PatchField<Bar<'a>> for bar_field::Nick
where
    for<'x> Option<String>: Clone + PartialEq,
{
    type Mode = Patched;
}

impl<'a> PatchField<Bar<'a>> for bar_field::Resource {
    type Mode = Skipped;
}

impl partly::shape_patch::PatchShape for Bar<'_> {
    const FIELDS: &'static [BarFields] =
        &[BarFields::Count, BarFields::Label, BarFields::Nick];
}

/// The old and new value of each patched field of a [`Bar`] patch, by name.
pub trait BarPatchFields<'a> {
    /// The old and new value of `count`, if it is written.
    fn count(&self) -> Option<PatchValue<&u32>>;
    /// The old and new value of `label`, if it is written.
    fn label(&self) -> Option<PatchValue<&&'a str>>;
    /// The old and new value of `nick`, if it is written.
    fn nick(&self) -> Option<PatchValue<&Option<String>>>;

    /// Returns `true` if `count` is written.
    fn has_count(&self) -> bool {
        self.count().is_some()
    }

    /// Returns `true` if `label` is written.
    fn has_label(&self) -> bool {
        self.label().is_some()
    }

    /// Returns `true` if `nick` is written.
    fn has_nick(&self) -> bool {
        self.nick().is_some()
    }
}

impl<'a> BarPatchFields<'a> for BarPatch<'a> {
    fn count(&self) -> Option<PatchValue<&u32>> {
        self.get::<bar_field::Count>()
    }

    fn label(&self) -> Option<PatchValue<&&'a str>> {
        self.get::<bar_field::Label>()
    }

    fn nick(&self) -> Option<PatchValue<&Option<String>>> {
        self.get::<bar_field::Nick>()
    }
}

impl<'a> BarPatchFields<'a> for BoundBarPatch<'_, 'a> {
    fn count(&self) -> Option<PatchValue<&u32>> {
        self.buffer()
            .get::<bar_field::Count>()
            .map(|new| PatchValue::new(&self.target().count, new))
    }

    fn label(&self) -> Option<PatchValue<&&'a str>> {
        self.buffer()
            .get::<bar_field::Label>()
            .map(|new| PatchValue::new(&self.target().label, new))
    }

    fn nick(&self) -> Option<PatchValue<&Option<String>>> {
        self.buffer()
            .get::<bar_field::Nick>()
            .map(|new| PatchValue::new(&self.target().nick, new))
    }
}

/// The named traits of [`Bar`], from both derives, for one `use`.
pub mod bar_prelude {
    pub use super::{BarPatchFields, BarSetters, BarSlots};
}

impl<'a> Patchable for Bar<'a>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> &'a str: Clone + PartialEq,
    for<'x> Option<String>: Clone + PartialEq,
{
    type Bound<'b>
        = BoundBarPatch<'b, 'a>
    where
        Self: 'b;
    type DefaultMergeStrategy = StrictMerge;
    type Patch = BarPatch<'a>;

    fn patch(&self) -> BoundBarPatch<'_, 'a> {
        Staged::new(self, ChangeRecorder::new())
    }

    fn diff(&self, new: &Self) -> BarPatch<'a> {
        partly::shape_patch::diff(self, new)
    }
}

// ---------------------------------------------------------------- per struct
// The merge strategy, with one method per patched field, connected to the
// generic `MergeField`.

/// Settles conflicts when a [`BarPatch`] is applied.
///
/// A method only runs for a field whose value differs from the one the patch
/// was made against. `new` is the value the patch writes, or `None` if it
/// doesn't write the field.
pub trait BarMergeStrategy<'a> {
    type Error: From<Conflict<BarFields>>;

    /// Settles a conflict on `count`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_count(
        &self,
        _current: &u32,
        _old: &u32,
        _new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        Err(Conflict(BarFields::Count).into())
    }

    /// Settles a conflict on `label`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_label(
        &self,
        _current: &&'a str,
        _old: &&'a str,
        _new: Option<&&'a str>,
    ) -> Result<Resolution<&'a str>, Self::Error> {
        Err(Conflict(BarFields::Label).into())
    }

    /// Settles a conflict on `nick`.
    ///
    /// # Errors
    ///
    /// By default, always fails.
    fn resolve_nick(
        &self,
        _current: &Option<String>,
        _old: &Option<String>,
        _new: Option<&Option<String>>,
    ) -> Result<Resolution<Option<String>>, Self::Error> {
        Err(Conflict(BarFields::Nick).into())
    }
}

impl BarMergeStrategy<'_> for StrictMerge {
    type Error = Conflict<BarFields>;
}

impl<'a> BarMergeStrategy<'a> for FieldMerge {
    type Error = Conflict<BarFields>;

    fn resolve_count(
        &self,
        _current: &u32,
        _old: &u32,
        new: Option<&u32>,
    ) -> Result<Resolution<u32>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => Err(Conflict(BarFields::Count)),
        }
    }

    fn resolve_label(
        &self,
        _current: &&'a str,
        _old: &&'a str,
        new: Option<&&'a str>,
    ) -> Result<Resolution<&'a str>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => Err(Conflict(BarFields::Label)),
        }
    }

    fn resolve_nick(
        &self,
        _current: &Option<String>,
        _old: &Option<String>,
        new: Option<&Option<String>>,
    ) -> Result<Resolution<Option<String>>, Self::Error> {
        match new {
            None => Ok(Resolution::KeepCurrent),
            Some(_) => Err(Conflict(BarFields::Nick)),
        }
    }
}

impl<'a, M: BarMergeStrategy<'a>> MergeField<Bar<'a>, M> for bar_field::Count {
    type Error = M::Error;

    fn resolve(
        strategy: &M,
        current: &u32,
        old: &u32,
        new: Option<&u32>,
    ) -> Result<Resolution<u32>, M::Error> {
        strategy.resolve_count(current, old, new)
    }
}

impl<'a, M: BarMergeStrategy<'a>> MergeField<Bar<'a>, M> for bar_field::Label {
    type Error = M::Error;

    fn resolve(
        strategy: &M,
        current: &&'a str,
        old: &&'a str,
        new: Option<&&'a str>,
    ) -> Result<Resolution<&'a str>, M::Error> {
        strategy.resolve_label(current, old, new)
    }
}

impl<'a, M: BarMergeStrategy<'a>> MergeField<Bar<'a>, M> for bar_field::Nick {
    type Error = M::Error;

    fn resolve(
        strategy: &M,
        current: &Option<String>,
        old: &Option<String>,
        new: Option<&Option<String>>,
    ) -> Result<Resolution<Option<String>>, M::Error> {
        strategy.resolve_nick(current, old, new)
    }
}

/// Never called: a skipped field never conflicts.
impl<'a, M: BarMergeStrategy<'a>> MergeField<Bar<'a>, M>
    for bar_field::Resource
{
    type Error = M::Error;

    fn resolve(
        _strategy: &M,
        _current: &Resource,
        _old: &Resource,
        _new: Option<&Resource>,
    ) -> Result<Resolution<Resource>, M::Error> {
        Ok(Resolution::KeepCurrent)
    }
}

impl<'a, M: BarMergeStrategy<'a>, V: BarValidator<'a>> ApplyPatchWith<M, V>
    for Bar<'a>
where
    for<'x> u32: Clone + PartialEq,
    for<'x> &'a str: Clone + PartialEq,
    for<'x> Option<String>: Clone + PartialEq,
{
    type MergeError = M::Error;

    fn apply_patch_with(
        &mut self,
        patch: BarPatch<'a>,
        strategy: &M,
        validator: &V,
    ) -> Result<
        (),
        Rejected<
            BarPatch<'a>,
            ApplyFailure<M::Error, ValidationErrors<BarFields, V::Error>>,
        >,
    > {
        partly::shape_patch::apply::<_, _, _, M::Error, V::Error>(
            self, patch, strategy, validator,
        )
        .map(|_replaced| ())
    }

    fn apply_patch_with_get_undo(
        &mut self,
        patch: BarPatch<'a>,
        strategy: &M,
        validator: &V,
    ) -> Result<
        BarPatch<'a>,
        Rejected<
            BarPatch<'a>,
            ApplyFailure<M::Error, ValidationErrors<BarFields, V::Error>>,
        >,
    > {
        partly::shape_patch::apply_get_undo::<_, _, _, M::Error, V::Error>(
            self, patch, strategy, validator,
        )
    }
}
