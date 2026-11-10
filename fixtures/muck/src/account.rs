//! The base expansion with a custom `Debug` hook. Only the blocks the tests
//! use are written out; the rest follows `foo.rs`.
//!
//! ```ignore
//! #[derive(Muck)]
//! #[muck(debug = custom)]
//! pub struct Account {
//!     pub name: String,
//!     pub password: String,
//! }
//!
//! // Written by the user: every representation prints through this.
//! impl<V> DebugShape<V> for Account
//! where
//!     V: AccountSlots + Representation<Account> + ?Sized,
//!     for<'a> account_slot::Name<'a, V>: Debug,
//! {
//!     fn fmt_shape(view: &V, f: &mut Formatter<'_>) -> fmt::Result {
//!         f.debug_struct(&name_of::<Account, V>())
//!             .field("name", &view.name_slot())
//!             .field("password", &"***")
//!             .finish()
//!     }
//! }
//! ```

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use fieldmuck::{
    Accepts, DebugShape, Field, FieldOf, Kind, Make, MakeField, ReadField,
    ReadSlot, Representation, SetField, Shape, ShapeWith, SlotFor, SlotOf,
    SlotType, SlotView, WriteSlot, name_of,
};

/// No `Debug` derive: `#[derive(Muck)]` implements it through the hook.
pub struct Account {
    pub name: String,
    pub password: String,
}

// ======================================================= #[derive(Muck)]

/// Names a field of [`Account`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountFields {
    Name,
    Password,
}

impl Field for AccountFields {
    const ALL: &'static [Self] = &[Self::Name, Self::Password];

    fn name(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Password => "password",
        }
    }
}

/// The fields of [`Account`], as types.
pub mod account_field {
    /// The field `name`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Name;
    /// The field `password`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Password;
}

/// [`Account`] with a slot of any type in place of each field, of kind `K`.
/// The kind is private and checked by every constructor, as in `foo.rs`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountShape<N, P, K = ()> {
    pub name: N,
    pub password: P,
    __kind: PhantomData<K>,
}

impl<N, P, K> AccountShape<N, P, K> {
    /// Creates the companion from its slots.
    pub const fn from_slots(name: N, password: P) -> Self
    where
        K: Accepts<String, N> + Accepts<String, P>,
    {
        Self {
            name,
            password,
            __kind: PhantomData,
        }
    }
}

impl Shape for Account {
    type Fields = AccountFields;

    const NAME: &'static str = "Account";
}

impl Representation<Account> for Account {
    const KIND: &'static str = "";
}

impl<N, P, K: Kind> Representation<Account> for AccountShape<N, P, K> {
    const KIND: &'static str = K::NAME;
}

impl<K> ShapeWith<K> for Account
where
    K: SlotFor<Account, account_field::Name>
        + SlotFor<Account, account_field::Password>,
{
    type Out = AccountShape<
        <K as SlotFor<Account, account_field::Name>>::Slot,
        <K as SlotFor<Account, account_field::Password>>::Slot,
        K,
    >;
}

impl FieldOf<Account> for account_field::Name {
    type Type = String;

    const FIELD: AccountFields = AccountFields::Name;

    fn get(value: &Account) -> &String {
        &value.name
    }

    fn get_mut(value: &mut Account) -> &mut String {
        &mut value.name
    }
}

impl FieldOf<Account> for account_field::Password {
    type Type = String;

    const FIELD: AccountFields = AccountFields::Password;

    fn get(value: &Account) -> &String {
        &value.password
    }

    fn get_mut(value: &mut Account) -> &mut String {
        &mut value.password
    }
}

impl<K> SlotOf<Account, K> for account_field::Name
where
    K: SlotFor<Account, account_field::Name>
        + SlotFor<Account, account_field::Password>,
{
    fn slot(
        with: &<Account as ShapeWith<K>>::Out,
    ) -> &<K as SlotFor<Account, Self>>::Slot {
        &with.name
    }

    fn slot_mut(
        with: &mut <Account as ShapeWith<K>>::Out,
    ) -> &mut <K as SlotFor<Account, Self>>::Slot {
        &mut with.name
    }
}

impl<K> SlotOf<Account, K> for account_field::Password
where
    K: SlotFor<Account, account_field::Name>
        + SlotFor<Account, account_field::Password>,
{
    fn slot(
        with: &<Account as ShapeWith<K>>::Out,
    ) -> &<K as SlotFor<Account, Self>>::Slot {
        &with.password
    }

    fn slot_mut(
        with: &mut <Account as ShapeWith<K>>::Out,
    ) -> &mut <K as SlotFor<Account, Self>>::Slot {
        &mut with.password
    }
}

impl<K, V> Make<K, V> for Account
where
    K: SlotFor<Account, account_field::Name>
        + SlotFor<Account, account_field::Password>,
    V: MakeField<Account, account_field::Name, K>
        + MakeField<Account, account_field::Password, K>,
{
    fn make(maker: &mut V) -> Self::Out {
        AccountShape {
            name: MakeField::<Account, account_field::Name, K>::make(maker),
            password: MakeField::<Account, account_field::Password, K>::make(
                maker,
            ),
            __kind: PhantomData,
        }
    }
}

impl ReadField<Account, account_field::Name> for Account {
    fn read_field(&self) -> Option<&String> {
        Some(&self.name)
    }
}

impl ReadField<Account, account_field::Password> for Account {
    fn read_field(&self) -> Option<&String> {
        Some(&self.password)
    }
}

impl<N: ReadSlot<String>, P, K> ReadField<Account, account_field::Name>
    for AccountShape<N, P, K>
{
    fn read_field(&self) -> Option<&String> {
        self.name.read_slot()
    }
}

impl<N, P: ReadSlot<String>, K> ReadField<Account, account_field::Password>
    for AccountShape<N, P, K>
{
    fn read_field(&self) -> Option<&String> {
        self.password.read_slot()
    }
}

impl<N, P: WriteSlot<String>, K> SetField<Account, account_field::Password>
    for AccountShape<N, P, K>
{
    fn set_field(&mut self, value: String) {
        self.password.write_slot(value);
    }
}

impl<'a> SlotType<'a, Account, account_field::Name> for Account {
    type Slot = &'a String;
}

impl SlotView<Account, account_field::Name> for Account {
    fn slot_view(&self) -> &String {
        &self.name
    }
}

impl<'a> SlotType<'a, Account, account_field::Password> for Account {
    type Slot = &'a String;
}

impl SlotView<Account, account_field::Password> for Account {
    fn slot_view(&self) -> &String {
        &self.password
    }
}

impl<'a, N, P, K> SlotType<'a, Account, account_field::Name>
    for AccountShape<N, P, K>
{
    type Slot = &'a N;
}

impl<N, P, K> SlotView<Account, account_field::Name> for AccountShape<N, P, K> {
    fn slot_view(&self) -> &N {
        &self.name
    }
}

impl<'a, N, P, K> SlotType<'a, Account, account_field::Password>
    for AccountShape<N, P, K>
{
    type Slot = &'a P;
}

impl<N, P, K> SlotView<Account, account_field::Password>
    for AccountShape<N, P, K>
{
    fn slot_view(&self) -> &P {
        &self.password
    }
}

/// Has a slot for every field of [`Account`].
pub trait AccountSlots:
    SlotView<Account, account_field::Name>
    + SlotView<Account, account_field::Password>
{
    fn name_slot(&self) -> account_slot::Name<'_, Self> {
        SlotView::<Account, account_field::Name>::slot_view(self)
    }

    fn password_slot(&self) -> account_slot::Password<'_, Self> {
        SlotView::<Account, account_field::Password>::slot_view(self)
    }
}

impl<T> AccountSlots for T where
    T: ?Sized
        + SlotView<Account, account_field::Name>
        + SlotView<Account, account_field::Password>
{
}

/// The type of each field's slot in a representation `V`.
pub mod account_slot {
    use fieldmuck::SlotType;

    use super::{Account, account_field};

    /// The slot of `name` in `V`.
    pub type Name<'a, V> =
        <V as SlotType<'a, Account, account_field::Name>>::Slot;
    /// The slot of `password` in `V`.
    pub type Password<'a, V> =
        <V as SlotType<'a, Account, account_field::Password>>::Slot;
}

// `#[muck(debug = custom)]`: no default hook, and `Debug` for `Account`
// itself, since it has no derive.

impl Debug for Account
where
    Account: DebugShape<Account>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Account as DebugShape<Account>>::fmt_shape(self, f)
    }
}

impl<N, P, K> Debug for AccountShape<N, P, K>
where
    Account: DebugShape<Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Account as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ============================================================ the user's

/// Prints every representation of an account without its password. The
/// password's slot needs no `Debug`, since it isn't printed.
impl<V> DebugShape<V> for Account
where
    V: AccountSlots + Representation<Account> + ?Sized,
    for<'a> account_slot::Name<'a, V>: Debug,
{
    fn fmt_shape(view: &V, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&name_of::<Account, V>())
            .field("name", &view.name_slot())
            .field("password", &"***")
            .finish()
    }
}
