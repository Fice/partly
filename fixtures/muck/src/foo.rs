//! The base expansion for a plain struct.
//!
//! ```ignore
//! #[derive(Muck)]
//! pub struct Foo {
//!     pub bar: u32,
//!     pub zee: String,
//!     pub nick: Option<String>,
//! }
//! ```
//!
//! A companion's kind can't disagree with its slots. A partial's slot for
//! `bar: u32` must be a `u32` slot:
//!
//! ```compile_fail,E0277
//! use fieldmuck::Optional;
//! use fieldmuck_fixtures::foo::FooShape;
//!
//! let _: FooShape<_, _, _, Optional> =
//!     FooShape::from_slots(Some("not a u32"), None, None);
//! ```
//!
//! And the kind can't be set by hand:
//!
//! ```compile_fail,E0451
//! use std::marker::PhantomData;
//!
//! use fieldmuck::Optional;
//! use fieldmuck_fixtures::foo::FooShape;
//!
//! let _: FooShape<_, _, _, Optional> = FooShape {
//!     bar: Some(1),
//!     zee: None,
//!     nick: None,
//!     __kind: PhantomData,
//! };
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use fieldmuck::{
    Accepts, Apply, ApplyField, DebugShape, EachField, Field, FieldOf, ForEach,
    Kind, Make, MakeField, MakeFieldFrom, MakeFrom, ReadField, ReadSlot,
    ReplaceSlot, Representation, SetField, Shape, ShapeWith, SlotFor, SlotOf,
    SlotType, SlotView, Visit, VisitField, VisitFieldWithValue, VisitWithValue,
    WriteSlot, name_of,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Foo {
    pub bar:  u32,
    pub zee:  String,
    pub nick: Option<String>,
}

// ---------------------------------------------------------------- per struct
// The field enum, as in `partly`.

/// Names a field of [`Foo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FooFields {
    Bar,
    Zee,
    Nick,
}

impl Field for FooFields {
    const ALL: &'static [Self] = &[Self::Bar, Self::Zee, Self::Nick];

    fn name(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Zee => "zee",
            Self::Nick => "nick",
        }
    }
}

// ---------------------------------------------------------------- per struct
// A type per field. Libraries key per-field behaviour on these.

/// The fields of [`Foo`], as types.
pub mod foo_field {
    /// The field `bar`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Bar;
    /// The field `zee`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Zee;
    /// The field `nick`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Nick;
}

// ---------------------------------------------------------------- per struct
// The one companion struct. Every companion type is an instance of it, so it
// can derive: each derive only holds when every slot type has the trait.

/// [`Foo`] with a slot of any type in place of each field, of kind `K`. The
/// kind names it: the partial prints as `FooPartial`.
///
/// The kind is private, and every constructor checks that `K` accepts each
/// slot, so the kind can't disagree with the slots. Build one with
/// [`from_slots`](Self::from_slots) or a kind's own constructor.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FooShape<B, Z, N, K = ()> {
    pub bar:  B,
    pub zee:  Z,
    pub nick: N,
    __kind:   PhantomData<K>,
}

impl<B, Z, N, K> FooShape<B, Z, N, K> {
    /// Creates the companion from its slots.
    pub const fn from_slots(bar: B, zee: Z, nick: N) -> Self
    where
        K: Accepts<u32, B> + Accepts<String, Z> + Accepts<Option<String>, N>,
    {
        Self {
            bar,
            zee,
            nick,
            __kind: PhantomData,
        }
    }
}

impl<B: Default, Z: Default, N: Default, K> Default for FooShape<B, Z, N, K>
where
    K: Accepts<u32, B> + Accepts<String, Z> + Accepts<Option<String>, N>,
{
    fn default() -> Self {
        Self::from_slots(B::default(), Z::default(), N::default())
    }
}

impl Shape for Foo {
    type Fields = FooFields;

    const NAME: &'static str = "Foo";
}

impl Representation<Foo> for Foo {
    const KIND: &'static str = "";
}

impl<B, Z, N, K: Kind> Representation<Foo> for FooShape<B, Z, N, K> {
    const KIND: &'static str = K::NAME;
}

impl<K> ShapeWith<K> for Foo
where
    K: SlotFor<Foo, foo_field::Bar>
        + SlotFor<Foo, foo_field::Zee>
        + SlotFor<Foo, foo_field::Nick>,
{
    type Out = FooShape<
        <K as SlotFor<Foo, foo_field::Bar>>::Slot,
        <K as SlotFor<Foo, foo_field::Zee>>::Slot,
        <K as SlotFor<Foo, foo_field::Nick>>::Slot,
        K,
    >;
}

// ----------------------------------------------------------------- per field

impl FieldOf<Foo> for foo_field::Bar {
    type Type = u32;

    const FIELD: FooFields = FooFields::Bar;

    fn get(value: &Foo) -> &u32 {
        &value.bar
    }

    fn get_mut(value: &mut Foo) -> &mut u32 {
        &mut value.bar
    }
}

impl FieldOf<Foo> for foo_field::Zee {
    type Type = String;

    const FIELD: FooFields = FooFields::Zee;

    fn get(value: &Foo) -> &String {
        &value.zee
    }

    fn get_mut(value: &mut Foo) -> &mut String {
        &mut value.zee
    }
}

impl FieldOf<Foo> for foo_field::Nick {
    type Type = Option<String>;

    const FIELD: FooFields = FooFields::Nick;

    fn get(value: &Foo) -> &Option<String> {
        &value.nick
    }

    fn get_mut(value: &mut Foo) -> &mut Option<String> {
        &mut value.nick
    }
}

// ----------------------------------------------------------------- per field
// Slot access needs every field's bound, because the companion type does.

impl<K> SlotOf<Foo, K> for foo_field::Bar
where
    K: SlotFor<Foo, foo_field::Bar>
        + SlotFor<Foo, foo_field::Zee>
        + SlotFor<Foo, foo_field::Nick>,
{
    fn slot(
        with: &<Foo as ShapeWith<K>>::Out,
    ) -> &<K as SlotFor<Foo, Self>>::Slot {
        &with.bar
    }

    fn slot_mut(
        with: &mut <Foo as ShapeWith<K>>::Out,
    ) -> &mut <K as SlotFor<Foo, Self>>::Slot {
        &mut with.bar
    }
}

impl<K> SlotOf<Foo, K> for foo_field::Zee
where
    K: SlotFor<Foo, foo_field::Bar>
        + SlotFor<Foo, foo_field::Zee>
        + SlotFor<Foo, foo_field::Nick>,
{
    fn slot(
        with: &<Foo as ShapeWith<K>>::Out,
    ) -> &<K as SlotFor<Foo, Self>>::Slot {
        &with.zee
    }

    fn slot_mut(
        with: &mut <Foo as ShapeWith<K>>::Out,
    ) -> &mut <K as SlotFor<Foo, Self>>::Slot {
        &mut with.zee
    }
}

impl<K> SlotOf<Foo, K> for foo_field::Nick
where
    K: SlotFor<Foo, foo_field::Bar>
        + SlotFor<Foo, foo_field::Zee>
        + SlotFor<Foo, foo_field::Nick>,
{
    fn slot(
        with: &<Foo as ShapeWith<K>>::Out,
    ) -> &<K as SlotFor<Foo, Self>>::Slot {
        &with.nick
    }

    fn slot_mut(
        with: &mut <Foo as ShapeWith<K>>::Out,
    ) -> &mut <K as SlotFor<Foo, Self>>::Slot {
        &mut with.nick
    }
}

// ---------------------------------------------------------------- per struct
// The traversals: one call per field, in declaration order.

impl<K, V> Visit<K, V> for Foo
where
    K: SlotFor<Foo, foo_field::Bar>
        + SlotFor<Foo, foo_field::Zee>
        + SlotFor<Foo, foo_field::Nick>,
    V: VisitField<Foo, foo_field::Bar, K>
        + VisitField<Foo, foo_field::Zee, K>
        + VisitField<Foo, foo_field::Nick, K>,
{
    fn visit(with: &Self::Out, visitor: &mut V) {
        VisitField::<Foo, foo_field::Bar, K>::visit(visitor, &with.bar);
        VisitField::<Foo, foo_field::Zee, K>::visit(visitor, &with.zee);
        VisitField::<Foo, foo_field::Nick, K>::visit(visitor, &with.nick);
    }
}

impl<K, V> VisitWithValue<K, V> for Foo
where
    K: SlotFor<Foo, foo_field::Bar>
        + SlotFor<Foo, foo_field::Zee>
        + SlotFor<Foo, foo_field::Nick>,
    V: VisitFieldWithValue<Foo, foo_field::Bar, K>
        + VisitFieldWithValue<Foo, foo_field::Zee, K>
        + VisitFieldWithValue<Foo, foo_field::Nick, K>,
{
    fn visit_with_value(with: &Self::Out, value: &Foo, visitor: &mut V) {
        VisitFieldWithValue::<Foo, foo_field::Bar, K>::visit(
            visitor, &with.bar, &value.bar,
        );
        VisitFieldWithValue::<Foo, foo_field::Zee, K>::visit(
            visitor, &with.zee, &value.zee,
        );
        VisitFieldWithValue::<Foo, foo_field::Nick, K>::visit(
            visitor,
            &with.nick,
            &value.nick,
        );
    }
}

impl<K, V> Make<K, V> for Foo
where
    K: SlotFor<Foo, foo_field::Bar>
        + SlotFor<Foo, foo_field::Zee>
        + SlotFor<Foo, foo_field::Nick>,
    V: MakeField<Foo, foo_field::Bar, K>
        + MakeField<Foo, foo_field::Zee, K>
        + MakeField<Foo, foo_field::Nick, K>,
{
    fn make(maker: &mut V) -> Self::Out {
        FooShape {
            bar:    MakeField::<Foo, foo_field::Bar, K>::make(maker),
            zee:    MakeField::<Foo, foo_field::Zee, K>::make(maker),
            nick:   MakeField::<Foo, foo_field::Nick, K>::make(maker),
            __kind: PhantomData,
        }
    }
}

impl<K, V> MakeFrom<K, V> for Foo
where
    K: SlotFor<Foo, foo_field::Bar>
        + SlotFor<Foo, foo_field::Zee>
        + SlotFor<Foo, foo_field::Nick>,
    V: MakeFieldFrom<Foo, foo_field::Bar, K>
        + MakeFieldFrom<Foo, foo_field::Zee, K>
        + MakeFieldFrom<Foo, foo_field::Nick, K>,
{
    fn make_from(value: &Foo, maker: &mut V) -> Self::Out {
        FooShape {
            bar:    MakeFieldFrom::<Foo, foo_field::Bar, K>::make_from(
                maker, &value.bar,
            ),
            zee:    MakeFieldFrom::<Foo, foo_field::Zee, K>::make_from(
                maker, &value.zee,
            ),
            nick:   MakeFieldFrom::<Foo, foo_field::Nick, K>::make_from(
                maker,
                &value.nick,
            ),
            __kind: PhantomData,
        }
    }
}

impl<K, V> Apply<K, V> for Foo
where
    K: SlotFor<Foo, foo_field::Bar>
        + SlotFor<Foo, foo_field::Zee>
        + SlotFor<Foo, foo_field::Nick>,
    V: ApplyField<Foo, foo_field::Bar, K>
        + ApplyField<Foo, foo_field::Zee, K>
        + ApplyField<Foo, foo_field::Nick, K>,
{
    fn apply(with: Self::Out, target: &mut Foo, visitor: &mut V) {
        ApplyField::<Foo, foo_field::Bar, K>::apply(
            visitor,
            with.bar,
            &mut target.bar,
        );
        ApplyField::<Foo, foo_field::Zee, K>::apply(
            visitor,
            with.zee,
            &mut target.zee,
        );
        ApplyField::<Foo, foo_field::Nick, K>::apply(
            visitor,
            with.nick,
            &mut target.nick,
        );
    }
}

// ----------------------------------------------------------------- per field
// Interop: `Foo` reads and writes its own fields, and a companion reads a
// field when its slot type opts in with `ReadSlot`.

impl ReadField<Foo, foo_field::Bar> for Foo {
    fn read_field(&self) -> Option<&u32> {
        Some(&self.bar)
    }
}

impl ReadField<Foo, foo_field::Zee> for Foo {
    fn read_field(&self) -> Option<&String> {
        Some(&self.zee)
    }
}

impl ReadField<Foo, foo_field::Nick> for Foo {
    fn read_field(&self) -> Option<&Option<String>> {
        Some(&self.nick)
    }
}

impl<B: ReadSlot<u32>, Z, N, K> ReadField<Foo, foo_field::Bar>
    for FooShape<B, Z, N, K>
{
    fn read_field(&self) -> Option<&u32> {
        self.bar.read_slot()
    }
}

impl<B, Z: ReadSlot<String>, N, K> ReadField<Foo, foo_field::Zee>
    for FooShape<B, Z, N, K>
{
    fn read_field(&self) -> Option<&String> {
        self.zee.read_slot()
    }
}

impl<B, Z, N: ReadSlot<Option<String>>, K> ReadField<Foo, foo_field::Nick>
    for FooShape<B, Z, N, K>
{
    fn read_field(&self) -> Option<&Option<String>> {
        self.nick.read_slot()
    }
}

impl SetField<Foo, foo_field::Bar> for Foo {
    fn set_field(&mut self, value: u32) {
        self.bar = value;
    }
}

impl SetField<Foo, foo_field::Zee> for Foo {
    fn set_field(&mut self, value: String) {
        self.zee = value;
    }
}

impl SetField<Foo, foo_field::Nick> for Foo {
    fn set_field(&mut self, value: Option<String>) {
        self.nick = value;
    }
}

impl<B: WriteSlot<u32>, Z, N, K> SetField<Foo, foo_field::Bar>
    for FooShape<B, Z, N, K>
{
    fn set_field(&mut self, value: u32) {
        self.bar.write_slot(value);
    }
}

impl<B, Z: WriteSlot<String>, N, K> SetField<Foo, foo_field::Zee>
    for FooShape<B, Z, N, K>
{
    fn set_field(&mut self, value: String) {
        self.zee.write_slot(value);
    }
}

impl<B, Z, N: WriteSlot<Option<String>>, K> SetField<Foo, foo_field::Nick>
    for FooShape<B, Z, N, K>
{
    fn set_field(&mut self, value: Option<String>) {
        self.nick.write_slot(value);
    }
}

// ----------------------------------------------------------------- per field
// Replacing one slot changes one type parameter, which typestate needs.

impl<B, Z, N, K: Accepts<u32, X>, X> ReplaceSlot<foo_field::Bar, X>
    for FooShape<B, Z, N, K>
{
    type Old = B;
    type Out = FooShape<X, Z, N, K>;

    fn replace_slot(self, slot: X) -> (B, FooShape<X, Z, N, K>) {
        (self.bar, FooShape {
            bar:    slot,
            zee:    self.zee,
            nick:   self.nick,
            __kind: self.__kind,
        })
    }
}

impl<B, Z, N, K: Accepts<String, X>, X> ReplaceSlot<foo_field::Zee, X>
    for FooShape<B, Z, N, K>
{
    type Old = Z;
    type Out = FooShape<B, X, N, K>;

    fn replace_slot(self, slot: X) -> (Z, FooShape<B, X, N, K>) {
        (self.zee, FooShape {
            bar:    self.bar,
            zee:    slot,
            nick:   self.nick,
            __kind: self.__kind,
        })
    }
}

impl<B, Z, N, K: Accepts<Option<String>, X>, X> ReplaceSlot<foo_field::Nick, X>
    for FooShape<B, Z, N, K>
{
    type Old = N;
    type Out = FooShape<B, Z, X, K>;

    fn replace_slot(self, slot: X) -> (N, FooShape<B, Z, X, K>) {
        (self.nick, FooShape {
            bar:    self.bar,
            zee:    self.zee,
            nick:   slot,
            __kind: self.__kind,
        })
    }
}

// ---------------------------------------------------------------- per struct

impl<V> ForEach<V> for Foo
where
    V: EachField<Foo, foo_field::Bar>
        + EachField<Foo, foo_field::Zee>
        + EachField<Foo, foo_field::Nick>,
{
    fn for_each(visitor: &mut V) {
        EachField::<Foo, foo_field::Bar>::each(visitor);
        EachField::<Foo, foo_field::Zee>::each(visitor);
        EachField::<Foo, foo_field::Nick>::each(visitor);
    }
}

// ---------------------------------------------------------------- per struct
// Named methods for anything that reads or writes `Foo`'s fields. A method is
// only callable where the bound holds.

/// Reads the fields of [`Foo`] by name, from anything that can.
pub trait FooReaders {
    fn read_bar(&self) -> Option<&u32>
    where
        Self: ReadField<Foo, foo_field::Bar>,
    {
        ReadField::<Foo, foo_field::Bar>::read_field(self)
    }

    fn read_zee(&self) -> Option<&String>
    where
        Self: ReadField<Foo, foo_field::Zee>,
    {
        ReadField::<Foo, foo_field::Zee>::read_field(self)
    }

    fn read_nick(&self) -> Option<&Option<String>>
    where
        Self: ReadField<Foo, foo_field::Nick>,
    {
        ReadField::<Foo, foo_field::Nick>::read_field(self)
    }

    fn bar_slot(&self) -> foo_slot::Bar<'_, Self>
    where
        Self: SlotView<Foo, foo_field::Bar>,
    {
        SlotView::<Foo, foo_field::Bar>::slot_view(self)
    }

    fn zee_slot(&self) -> foo_slot::Zee<'_, Self>
    where
        Self: SlotView<Foo, foo_field::Zee>,
    {
        SlotView::<Foo, foo_field::Zee>::slot_view(self)
    }

    fn nick_slot(&self) -> foo_slot::Nick<'_, Self>
    where
        Self: SlotView<Foo, foo_field::Nick>,
    {
        SlotView::<Foo, foo_field::Nick>::slot_view(self)
    }
}

impl<R: ?Sized> FooReaders for R {}

// ---------------------------------------------------------------- per struct
// The views, as single bounds. Every representation that holds values is a
// `FooView`; every representation is a `FooSlots`. Both work as trait
// objects, so algorithms can take a mix.

/// Can read every field of [`Foo`]: the value view.
pub trait FooView:
    ReadField<Foo, foo_field::Bar>
    + ReadField<Foo, foo_field::Zee>
    + ReadField<Foo, foo_field::Nick>
{
}

impl<T> FooView for T where
    T: ?Sized
        + ReadField<Foo, foo_field::Bar>
        + ReadField<Foo, foo_field::Zee>
        + ReadField<Foo, foo_field::Nick>
{
}

/// Has a slot for every field of [`Foo`]: the slot view.
pub trait FooSlots:
    SlotView<Foo, foo_field::Bar>
    + SlotView<Foo, foo_field::Zee>
    + SlotView<Foo, foo_field::Nick>
{
}

impl<T> FooSlots for T where
    T: ?Sized
        + SlotView<Foo, foo_field::Bar>
        + SlotView<Foo, foo_field::Zee>
        + SlotView<Foo, foo_field::Nick>
{
}

/// The type of each field's slot in a representation `V`, for bounds like
/// `for<'a> foo_slot::Bar<'a, V>: Debug`.
pub mod foo_slot {
    use fieldmuck::SlotType;

    use super::{Foo, foo_field};

    /// The slot of `bar` in `V`.
    pub type Bar<'a, V> = <V as SlotType<'a, Foo, foo_field::Bar>>::Slot;
    /// The slot of `zee` in `V`.
    pub type Zee<'a, V> = <V as SlotType<'a, Foo, foo_field::Zee>>::Slot;
    /// The slot of `nick` in `V`.
    pub type Nick<'a, V> = <V as SlotType<'a, Foo, foo_field::Nick>>::Slot;
}

// ----------------------------------------------------------------- per field
// The slot view of `Foo` itself is its fields; of the companion, its slots.

impl<'a> SlotType<'a, Foo, foo_field::Bar> for Foo {
    type Slot = &'a u32;
}

impl SlotView<Foo, foo_field::Bar> for Foo {
    fn slot_view(&self) -> &u32 {
        &self.bar
    }
}

impl<'a> SlotType<'a, Foo, foo_field::Zee> for Foo {
    type Slot = &'a String;
}

impl SlotView<Foo, foo_field::Zee> for Foo {
    fn slot_view(&self) -> &String {
        &self.zee
    }
}

impl<'a> SlotType<'a, Foo, foo_field::Nick> for Foo {
    type Slot = &'a Option<String>;
}

impl SlotView<Foo, foo_field::Nick> for Foo {
    fn slot_view(&self) -> &Option<String> {
        &self.nick
    }
}

impl<'a, B, Z, N, K> SlotType<'a, Foo, foo_field::Bar>
    for FooShape<B, Z, N, K>
{
    type Slot = &'a B;
}

impl<B, Z, N, K> SlotView<Foo, foo_field::Bar> for FooShape<B, Z, N, K> {
    fn slot_view(&self) -> &B {
        &self.bar
    }
}

impl<'a, B, Z, N, K> SlotType<'a, Foo, foo_field::Zee>
    for FooShape<B, Z, N, K>
{
    type Slot = &'a Z;
}

impl<B, Z, N, K> SlotView<Foo, foo_field::Zee> for FooShape<B, Z, N, K> {
    fn slot_view(&self) -> &Z {
        &self.zee
    }
}

impl<'a, B, Z, N, K> SlotType<'a, Foo, foo_field::Nick>
    for FooShape<B, Z, N, K>
{
    type Slot = &'a N;
}

impl<B, Z, N, K> SlotView<Foo, foo_field::Nick> for FooShape<B, Z, N, K> {
    fn slot_view(&self) -> &N {
        &self.nick
    }
}

// ---------------------------------------------------------------- per struct
// `Debug` for every representation goes through one hook. This is the
// default, which prints every slot, so each needs `Debug`. With
// `#[muck(debug = custom)]`, the user writes this impl instead.

impl<V: FooSlots + Representation<Foo> + ?Sized> DebugShape<V> for Foo
where
    for<'a> foo_slot::Bar<'a, V>: Debug,
    for<'a> foo_slot::Zee<'a, V>: Debug,
    for<'a> foo_slot::Nick<'a, V>: Debug,
{
    fn fmt_shape(
        view: &V,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct(&name_of::<Foo, V>())
            .field("bar", &view.bar_slot())
            .field("zee", &view.zee_slot())
            .field("nick", &view.nick_slot())
            .finish()
    }
}

impl<B, Z, N, K> Debug for FooShape<B, Z, N, K>
where
    Foo: DebugShape<Self>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Foo as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ---------------------------------------------------------------- per struct

/// The named traits of [`Foo`], for one `use`. Named after the type, so two
/// derives in one module don't clash.
pub mod foo_prelude {
    pub use super::{FooReaders, FooSetters, FooSlots, FooView};
}

/// Writes the fields of [`Foo`] by name, to anything that accepts it.
pub trait FooSetters {
    fn set_bar(&mut self, value: u32)
    where
        Self: SetField<Foo, foo_field::Bar>,
    {
        SetField::<Foo, foo_field::Bar>::set_field(self, value);
    }

    fn set_zee(&mut self, value: String)
    where
        Self: SetField<Foo, foo_field::Zee>,
    {
        SetField::<Foo, foo_field::Zee>::set_field(self, value);
    }

    fn set_nick(&mut self, value: Option<String>)
    where
        Self: SetField<Foo, foo_field::Nick>,
    {
        SetField::<Foo, foo_field::Nick>::set_field(self, value);
    }
}

impl<R: ?Sized> FooSetters for R {}
