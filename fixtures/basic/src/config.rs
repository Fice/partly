//! Skipped fields, an `Option` field, and a custom strategy and validator.
//!
//! Only the blocks that differ from `foo.rs`, or that the partial needs, are
//! written out.
//!
//! ```ignore
//! #[derive(Muck, Partly)]
//! #[partly(build_strategy = ConfigDefaults, validator = ConfigRules)]
//! pub struct Config {
//!     pub name: String,
//!     pub nick: Option<String>,
//!     pub started_at: u64,
//!     #[partly(skip, required)]
//!     pub slug: String,
//!     #[partly(skip)]
//!     pub retries: u32,
//! }
//! ```
//!
//! `skip` is a `partly` attribute, so the shape still has every field, and so
//! does the field enum. The partial's slot for a skipped field is `Unset`: it
//! never holds a value, and has no type parameter. The partial's required
//! fields leave the skipped ones out.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use fieldmuck::{
    Accepts, DebugShape, FieldOf, Kind, Optional, Representation, Shape,
    SlotType, SlotView, Unset, name_of,
};
use partly::prelude::*;
use partly::{
    BuildFailure, Field, FieldError, MissingField, Rejected, Set,
    ValidationErrors,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub name: String,
    pub nick: Option<String>,
    pub started_at: u64,
    pub slug: String,
    pub retries: u32,
}

// ======================================================= #[derive(Muck)]

// ---------------------------------------------------------------- per struct

/// Names a field of [`Config`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigFields {
    Name,
    Nick,
    StartedAt,
    Slug,
    Retries,
}

impl Field for ConfigFields {
    const ALL: &'static [Self] = &[
        Self::Name,
        Self::Nick,
        Self::StartedAt,
        Self::Slug,
        Self::Retries,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Nick => "nick",
            Self::StartedAt => "started_at",
            Self::Slug => "slug",
            Self::Retries => "retries",
        }
    }
}

/// The fields of [`Config`], as types.
pub mod config_field {
    /// The field `name`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Name;
    /// The field `nick`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Nick;
    /// The field `started_at`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct StartedAt;
    /// The field `slug`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Slug;
    /// The field `retries`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Retries;
}

/// [`Config`] with a slot of any type in place of each field, of kind `K`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConfigShape<N, I, T, S, R, K = ()> {
    pub name: N,
    pub nick: I,
    pub started_at: T,
    pub slug: S,
    pub retries: R,
    __kind: PhantomData<K>,
}

impl<N, I, T, S, R, K> ConfigShape<N, I, T, S, R, K> {
    /// Creates the companion from its slots.
    pub const fn from_slots(
        name: N,
        nick: I,
        started_at: T,
        slug: S,
        retries: R,
    ) -> Self
    where
        K: Accepts<String, N>
            + Accepts<Option<String>, I>
            + Accepts<u64, T>
            + Accepts<String, S>
            + Accepts<u32, R>,
    {
        Self {
            name,
            nick,
            started_at,
            slug,
            retries,
            __kind: PhantomData,
        }
    }
}

impl<N, I, T, S, R, K> Default for ConfigShape<N, I, T, S, R, K>
where
    N: Default,
    I: Default,
    T: Default,
    S: Default,
    R: Default,
    K: Accepts<String, N>
        + Accepts<Option<String>, I>
        + Accepts<u64, T>
        + Accepts<String, S>
        + Accepts<u32, R>,
{
    fn default() -> Self {
        Self::from_slots(
            N::default(),
            I::default(),
            T::default(),
            S::default(),
            R::default(),
        )
    }
}

impl Shape for Config {
    type Fields = ConfigFields;

    const NAME: &'static str = "Config";
}

impl<N, I, T, S, R, K: Kind> Representation<Config>
    for ConfigShape<N, I, T, S, R, K>
{
    const KIND: &'static str = K::NAME;
}

// ----------------------------------------------------------------- per field

macro_rules! config_field {
    ($marker:ident, $field:ident, $type:ty, $param:ident) => {
        impl FieldOf<Config> for config_field::$marker {
            type Type = $type;

            const FIELD: ConfigFields = ConfigFields::$marker;

            fn get(value: &Config) -> &$type {
                &value.$field
            }

            fn get_mut(value: &mut Config) -> &mut $type {
                &mut value.$field
            }
        }

        impl<'a, N, I, T, S, R, K> SlotType<'a, Config, config_field::$marker>
            for ConfigShape<N, I, T, S, R, K>
        {
            type Slot = &'a $param;
        }

        impl<N, I, T, S, R, K> SlotView<Config, config_field::$marker>
            for ConfigShape<N, I, T, S, R, K>
        {
            fn slot_view(&self) -> &$param {
                &self.$field
            }
        }
    };
}

// A macro here only to keep the fixture short: the derive writes these out.
config_field!(Name, name, String, N);
config_field!(Nick, nick, Option<String>, I);
config_field!(StartedAt, started_at, u64, T);
config_field!(Slug, slug, String, S);
config_field!(Retries, retries, u32, R);

// ---------------------------------------------------------------- per struct

/// Has a slot for every field of [`Config`]: the slot view, by name.
pub trait ConfigSlots:
    SlotView<Config, config_field::Name>
    + SlotView<Config, config_field::Nick>
    + SlotView<Config, config_field::StartedAt>
    + SlotView<Config, config_field::Slug>
    + SlotView<Config, config_field::Retries>
{
    fn name_slot(&self) -> config_slot::Name<'_, Self> {
        SlotView::<Config, config_field::Name>::slot_view(self)
    }

    fn nick_slot(&self) -> config_slot::Nick<'_, Self> {
        SlotView::<Config, config_field::Nick>::slot_view(self)
    }

    fn started_at_slot(&self) -> config_slot::StartedAt<'_, Self> {
        SlotView::<Config, config_field::StartedAt>::slot_view(self)
    }

    fn slug_slot(&self) -> config_slot::Slug<'_, Self> {
        SlotView::<Config, config_field::Slug>::slot_view(self)
    }

    fn retries_slot(&self) -> config_slot::Retries<'_, Self> {
        SlotView::<Config, config_field::Retries>::slot_view(self)
    }
}

impl<V> ConfigSlots for V where
    V: ?Sized
        + SlotView<Config, config_field::Name>
        + SlotView<Config, config_field::Nick>
        + SlotView<Config, config_field::StartedAt>
        + SlotView<Config, config_field::Slug>
        + SlotView<Config, config_field::Retries>
{
}

/// The type of each field's slot in a representation `V`.
pub mod config_slot {
    use fieldmuck::SlotType;

    use super::{Config, config_field};

    /// The slot of `name` in `V`.
    pub type Name<'a, V> =
        <V as SlotType<'a, Config, config_field::Name>>::Slot;
    /// The slot of `nick` in `V`.
    pub type Nick<'a, V> =
        <V as SlotType<'a, Config, config_field::Nick>>::Slot;
    /// The slot of `started_at` in `V`.
    pub type StartedAt<'a, V> =
        <V as SlotType<'a, Config, config_field::StartedAt>>::Slot;
    /// The slot of `slug` in `V`.
    pub type Slug<'a, V> =
        <V as SlotType<'a, Config, config_field::Slug>>::Slot;
    /// The slot of `retries` in `V`.
    pub type Retries<'a, V> =
        <V as SlotType<'a, Config, config_field::Retries>>::Slot;
}

/// The default hook: prints every slot. A partial prints its skipped fields
/// as `Unset`.
impl<V: ConfigSlots + Representation<Config> + ?Sized> DebugShape<V> for Config
where
    for<'a> config_slot::Name<'a, V>: Debug,
    for<'a> config_slot::Nick<'a, V>: Debug,
    for<'a> config_slot::StartedAt<'a, V>: Debug,
    for<'a> config_slot::Slug<'a, V>: Debug,
    for<'a> config_slot::Retries<'a, V>: Debug,
{
    fn fmt_shape(view: &V, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&name_of::<Config, V>())
            .field("name", &view.name_slot())
            .field("nick", &view.nick_slot())
            .field("started_at", &view.started_at_slot())
            .field("slug", &view.slug_slot())
            .field("retries", &view.retries_slot())
            .finish()
    }
}

impl<N, I, T, S, R, K> Debug for ConfigShape<N, I, T, S, R, K>
where
    Config: DebugShape<Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Config as DebugShape<Self>>::fmt_shape(self, f)
    }
}

// ===================================================== #[derive(Partly)]

// ---------------------------------------------------------------- per struct
// Skipped fields get `Unset` slots, and no parameter. `nick` is not
// special-cased, because a macro can't reliably spot an `Option` behind a type
// alias. Its slot is simply `Option<Option<String>>`.

/// A [`Config`] whose fields may or may not be set.
pub type ConfigPartial<
    N = Option<String>,
    I = Option<Option<String>>,
    T = Option<u64>,
> = ConfigShape<N, I, T, Unset, Unset, Optional>;

/// A partial with every field known to be set.
pub type ConfigCompletePartial =
    ConfigPartial<Set<String>, Set<Option<String>>, Set<u64>>;

impl ConfigPartial {
    #[must_use]
    pub const fn new() -> Self {
        Self::from_slots(None, None, None, Unset, Unset)
    }
}

// ----------------------------------------------------------------- per field

impl<I, T> ConfigPartial<Option<String>, I, T> {
    pub fn set_name(&mut self, value: String) {
        self.name = Some(value);
    }

    #[must_use]
    pub const fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

impl<I, T> ConfigPartial<Set<String>, I, T> {
    #[must_use]
    pub const fn name(&self) -> &String {
        &self.name.0
    }
}

impl<N, T> ConfigPartial<N, Option<Option<String>>, T> {
    /// `set_nick(None)` stores an explicit `None`. Use `clear_nick` to unset.
    pub fn set_nick(&mut self, value: Option<String>) {
        self.nick = Some(value);
    }

    #[must_use]
    pub const fn nick(&self) -> Option<&Option<String>> {
        self.nick.as_ref()
    }

    pub fn clear_nick(&mut self) {
        self.nick = None;
    }
}

impl<N, I> ConfigPartial<N, I, Option<u64>> {
    pub fn set_started_at(&mut self, value: u64) {
        self.started_at = Some(value);
    }

    #[must_use]
    pub const fn started_at(&self) -> Option<&u64> {
        self.started_at.as_ref()
    }
}

impl<N, I> ConfigPartial<N, I, Set<u64>> {
    #[must_use]
    pub const fn started_at(&self) -> &u64 {
        &self.started_at.0
    }
}

// ---------------------------------------------------------------- per struct

impl From<Config> for ConfigPartial {
    fn from(value: Config) -> Self {
        Self::from_slots(
            Some(value.name),
            Some(value.nick),
            Some(value.started_at),
            Unset,
            Unset,
        )
    }
}

impl Partly for Config {
    type DefaultValidator = ConfigRules;
    type Field = ConfigFields;
    type Partial = ConfigPartial;
}

impl PartialOps for ConfigPartial {
    type Field = ConfigFields;
    type Target = Config;

    fn has(&self, field: ConfigFields) -> bool {
        match field {
            ConfigFields::Name => self.name.is_some(),
            ConfigFields::Nick => self.nick.is_some(),
            ConfigFields::StartedAt => self.started_at.is_some(),
            ConfigFields::Slug | ConfigFields::Retries => false,
        }
    }

    fn required_fields(&self) -> &'static [ConfigFields] {
        &[
            ConfigFields::Name,
            ConfigFields::Nick,
            ConfigFields::StartedAt,
        ]
    }

    fn merge(self, other: Self) -> Self {
        Self::from_slots(
            other.name.or(self.name),
            other.nick.or(self.nick),
            other.started_at.or(self.started_at),
            Unset,
            Unset,
        )
    }
}

// ---------------------------------------------------------------- per struct
// `skip, required` has no body, so every strategy must supply it. Plain `skip`
// has a `Default::default()` body.

pub trait ConfigBuildStrategy {
    type Error: From<MissingField<ConfigFields>>;

    fn can_build(&self, partial: &ConfigPartial) -> bool {
        partial.is_complete()
    }

    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_name(
        &self,
        value: Option<String>,
    ) -> Result<String, Self::Error> {
        value.ok_or_else(|| MissingField(ConfigFields::Name).into())
    }

    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_nick(
        &self,
        value: Option<Option<String>>,
    ) -> Result<Option<String>, Self::Error> {
        value.ok_or_else(|| MissingField(ConfigFields::Nick).into())
    }

    /// # Errors
    ///
    /// By default, fails if `value` is `None`.
    fn resolve_started_at(
        &self,
        value: Option<u64>,
    ) -> Result<u64, Self::Error> {
        value.ok_or_else(|| MissingField(ConfigFields::StartedAt).into())
    }

    /// Produces `slug` from the resolved fields.
    ///
    /// # Errors
    ///
    /// Returns why the value can't be produced.
    fn skipped_slug(
        &self,
        partial: &ConfigCompletePartial,
    ) -> Result<String, Self::Error>;

    /// Produces `retries` from the resolved fields.
    ///
    /// # Errors
    ///
    /// By default, never fails.
    fn skipped_retries(
        &self,
        _partial: &ConfigCompletePartial,
    ) -> Result<u32, Self::Error> {
        Ok(Default::default())
    }
}

// ---------------------------------------------------------------- user's code
// Everything down to the next marker is written by the user, not the macro.

/// Fills in a missing nick, rounds `started_at` to the hour and derives `slug`.
#[derive(Debug, Default)]
pub struct ConfigDefaults;

impl ConfigBuildStrategy for ConfigDefaults {
    type Error = MissingField<ConfigFields>;

    fn resolve_nick(
        &self,
        value: Option<Option<String>>,
    ) -> Result<Option<String>, Self::Error> {
        Ok(value.flatten())
    }

    fn resolve_started_at(
        &self,
        value: Option<u64>,
    ) -> Result<u64, Self::Error> {
        value
            .map(|seconds| seconds / 3600 * 3600)
            .ok_or(MissingField(ConfigFields::StartedAt))
    }

    fn skipped_slug(
        &self,
        partial: &ConfigCompletePartial,
    ) -> Result<String, Self::Error> {
        Ok(format!("{}-{}", partial.name(), partial.started_at()))
    }
}

/// Rejects empty names, and any nick equal to the name.
#[derive(Debug, Default)]
pub struct ConfigRules;

impl ConfigValidator for ConfigRules {
    type Error = &'static str;

    fn validate_name(&self, value: &String) -> Result<(), Self::Error> {
        if value.is_empty() {
            Err("must not be empty")
        } else {
            Ok(())
        }
    }

    fn validate_started_at(&self, value: &u64) -> Result<(), Self::Error> {
        if *value == 0 {
            Err("must not be zero")
        } else {
            Ok(())
        }
    }

    fn validate(&self, value: &Config) -> Result<(), Self::Error> {
        if value.nick.as_ref() == Some(&value.name) {
            Err("nick must differ from name")
        } else {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------- per struct

pub trait ConfigValidator {
    type Error;

    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_name(&self, _value: &String) -> Result<(), Self::Error> {
        Ok(())
    }

    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_nick(
        &self,
        _value: &Option<String>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate_started_at(&self, _value: &u64) -> Result<(), Self::Error> {
        Ok(())
    }

    /// # Errors
    ///
    /// Returns why the value isn't valid.
    fn validate(&self, _value: &Config) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<V: ConfigValidator> ValidateWith<V> for Config {
    type ApplyError = ValidationErrors<ConfigFields, V::Error>;
    type Error = V::Error;

    fn validate_with(
        &self,
        validator: &V,
    ) -> Result<(), ValidationErrors<ConfigFields, V::Error>> {
        let mut errors = Vec::new();
        if let Err(error) = validator.validate_name(&self.name) {
            errors.push(FieldError {
                field: ConfigFields::Name,
                error,
            });
        }
        if let Err(error) = validator.validate_nick(&self.nick) {
            errors.push(FieldError {
                field: ConfigFields::Nick,
                error,
            });
        }
        if let Err(error) = validator.validate_started_at(&self.started_at) {
            errors.push(FieldError {
                field: ConfigFields::StartedAt,
                error,
            });
        }
        if !errors.is_empty() {
            return Err(ValidationErrors::Fields(errors));
        }
        validator.validate(self).map_err(ValidationErrors::Struct)
    }

    fn validate_partial_with(
        partial: &ConfigPartial,
        validator: &V,
    ) -> Result<(), ValidationErrors<ConfigFields, V::Error>> {
        let mut errors = Vec::new();
        if let Some(Err(error)) = partial
            .name
            .as_ref()
            .map(|value| validator.validate_name(value))
        {
            errors.push(FieldError {
                field: ConfigFields::Name,
                error,
            });
        }
        if let Some(Err(error)) = partial
            .nick
            .as_ref()
            .map(|value| validator.validate_nick(value))
        {
            errors.push(FieldError {
                field: ConfigFields::Nick,
                error,
            });
        }
        if let Some(Err(error)) = partial
            .started_at
            .as_ref()
            .map(|value| validator.validate_started_at(value))
        {
            errors.push(FieldError {
                field: ConfigFields::StartedAt,
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
        mut input: ConfigPartial,
        validator: &V,
    ) -> Result<
        (),
        Rejected<ConfigPartial, ValidationErrors<ConfigFields, V::Error>>,
    > {
        if let Err(error) = Self::validate_partial_with(&input, validator) {
            return Err(Rejected {
                value: input,
                error,
            });
        }
        if let Some(value) = &mut input.name {
            std::mem::swap(&mut self.name, value);
        }
        if let Some(value) = &mut input.nick {
            std::mem::swap(&mut self.nick, value);
        }
        if let Some(value) = &mut input.started_at {
            std::mem::swap(&mut self.started_at, value);
        }
        if let Err(error) = validator.validate(self) {
            if let Some(value) = &mut input.name {
                std::mem::swap(&mut self.name, value);
            }
            if let Some(value) = &mut input.nick {
                std::mem::swap(&mut self.nick, value);
            }
            if let Some(value) = &mut input.started_at {
                std::mem::swap(&mut self.started_at, value);
            }
            return Err(Rejected {
                value: input,
                error: ValidationErrors::Struct(error),
            });
        }
        Ok(())
    }
}

impl<S: ConfigBuildStrategy, V: ConfigValidator> BuildWith<S, V>
    for ConfigPartial
{
    type Error =
        BuildFailure<S::Error, ValidationErrors<ConfigFields, V::Error>>;
    type Output = Config;

    fn build_with(
        mut self,
        strategy: &S,
        validator: &V,
    ) -> Result<Config, Rejected<Self, Self::Error>> {
        let name = match strategy.resolve_name(self.name.take()) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let nick = match strategy.resolve_nick(self.nick.take()) {
            Ok(value) => value,
            Err(error) => {
                self.name = Some(name);
                return Err(Rejected {
                    value: self,
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let started_at =
            match strategy.resolve_started_at(self.started_at.take()) {
                Ok(value) => value,
                Err(error) => {
                    self.name = Some(name);
                    self.nick = Some(nick);
                    return Err(Rejected {
                        value: self,
                        error: BuildFailure::Strategy(error),
                    });
                },
            };
        let complete = ConfigCompletePartial::from_slots(
            Set(name),
            Set(nick),
            Set(started_at),
            Unset,
            Unset,
        );
        let slug = match strategy.skipped_slug(&complete) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: complete.into(),
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let retries = match strategy.skipped_retries(&complete) {
            Ok(value) => value,
            Err(error) => {
                return Err(Rejected {
                    value: complete.into(),
                    error: BuildFailure::Strategy(error),
                });
            },
        };
        let config = Config {
            name: complete.name.0,
            nick: complete.nick.0,
            started_at: complete.started_at.0,
            slug,
            retries,
        };
        match config.validate_with(validator) {
            Ok(()) => Ok(config),
            Err(error) => {
                Err(Rejected {
                    value: config.into(),
                    error: BuildFailure::Validation(error),
                })
            },
        }
    }
}

impl From<ConfigCompletePartial> for ConfigPartial {
    fn from(value: ConfigCompletePartial) -> Self {
        Self::from_slots(
            Some(value.name.0),
            Some(value.nick.0),
            Some(value.started_at.0),
            Unset,
            Unset,
        )
    }
}

impl Build for ConfigPartial {
    type Error = <Self as BuildWith<ConfigDefaults, ConfigRules>>::Error;
    type Output = Config;

    fn can_build(&self) -> bool {
        ConfigDefaults.can_build(self)
    }

    fn build(self) -> Result<Config, Rejected<Self, Self::Error>> {
        self.build_with(&ConfigDefaults, &ConfigRules)
    }
}
