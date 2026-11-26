//! A deserializer that fills every slot of an event (tests only): each string of a type of `words.rs` gets a valid
//! sample of its type, but one slot, which gets the trapped data. An option is always there, a list has one item, an
//! enum takes its variant by `pick`. The test of the trapped data walks every slot of every event with it.

use std::cell::RefCell;
use std::rc::Rc;

use serde::de::value::{Error, StrDeserializer, StringDeserializer};
use serde::de::{self, DeserializeSeed, IntoDeserializer, Visitor};
use serde::forward_to_deserialize_any;

use crate::words::Digest;

/// What a fill saw: the types of its slots, in order.
#[derive(Default)]
pub struct Seen {
    pub kinds: Vec<&'static str>,
}

#[derive(Clone)]
pub struct Filler {
    trap: Option<(usize, Rc<str>)>,
    pick: usize,
    seen: Rc<RefCell<Seen>>,
}

/// The types whose strings are free (names, reasons): scrubbed, not refused (`never.rs`).
pub const FREE: &[&str] = &["Name", "Reason"];

impl Filler {
    /// A filler that puts `trap` in slot `at` (none: every slot its sample), its enums at their variant `pick`.
    pub fn new(trap: Option<(usize, &str)>, pick: usize) -> Filler {
        Filler {
            trap: trap.map(|(i, t)| (i, Rc::from(t))),
            pick,
            seen: Rc::default(),
        }
    }
    pub fn kinds(&self) -> Vec<&'static str> {
        self.seen.borrow().kinds.clone()
    }
}

/// A valid sample of a type of `words.rs`.
pub fn sample(kind: &str) -> String {
    match kind {
        "Id" => "d-77e0".into(),
        "TicketKey" => "ESC-608".into(),
        "SettingKey" => "agents.engines".into(),
        "Token" => "git".into(),
        "PluginId" => "dev.escouade.slack".into(),
        "Namespace" => "fr.acme.*".into(),
        "Version" => "2.3.0".into(),
        "Digest" => Digest::of(b"sample").as_str().to_owned(),
        "PublicKey" => format!("ed25519:{}", "A".repeat(43) + "="),
        "SixWords" => "abandon ability able about above absent".into(),
        "Ip" => "192.0.2.1".into(),
        "Name" => "Claire Martin".into(),
        "Reason" => "Seuls les fournisseurs sous contrat.".into(),
        "Time" => "2026-11-21T10:42:07.120Z".into(),
        other => panic!("no sample of {other}"),
    }
}

impl<'de> de::Deserializer<'de> for Filler {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Error> {
        Err(de::Error::custom("the filler fills typed slots only"))
    }

    fn deserialize_bool<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
        v.visit_bool(true)
    }
    fn deserialize_u8<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
        v.visit_u8(80)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
        v.visit_u32(2)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
        v.visit_u64(7)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
        v.visit_i64(3)
    }
    fn deserialize_option<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
        v.visit_some(self)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Error> {
        v.visit_seq(One(Some(self)))
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        v: V,
    ) -> Result<V::Value, Error> {
        let at = {
            let mut seen = self.seen.borrow_mut();
            seen.kinds.push(name);
            seen.kinds.len() - 1
        };
        let s = match &self.trap {
            Some((i, trap)) if *i == at => trap.to_string(),
            _ => sample(name),
        };
        let d: StringDeserializer<Error> = s.into_deserializer();
        v.visit_newtype_struct(d)
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        v: V,
    ) -> Result<V::Value, Error> {
        v.visit_map(Fields {
            fields: fields.iter(),
            filler: self,
        })
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        variants: &'static [&'static str],
        v: V,
    ) -> Result<V::Value, Error> {
        let name = variants[self.pick % variants.len()];
        v.visit_enum(Variant { name, filler: self })
    }

    forward_to_deserialize_any! {
        i8 i16 i32 i128 u16 u128 f32 f64 char str string bytes byte_buf unit unit_struct tuple tuple_struct map
        identifier ignored_any
    }
}

struct One(Option<Filler>);

impl<'de> de::SeqAccess<'de> for One {
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        match self.0.take() {
            Some(f) => seed.deserialize(f).map(Some),
            None => Ok(None),
        }
    }
}

struct Fields {
    fields: std::slice::Iter<'static, &'static str>,
    filler: Filler,
}

impl<'de> de::MapAccess<'de> for Fields {
    type Error = Error;
    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match self.fields.next() {
            Some(f) => {
                let d: StrDeserializer<Error> = (*f).into_deserializer();
                seed.deserialize(d).map(Some)
            }
            None => Ok(None),
        }
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        seed.deserialize(self.filler.clone())
    }
}

struct Variant {
    name: &'static str,
    filler: Filler,
}

impl<'de> de::EnumAccess<'de> for Variant {
    type Error = Error;
    type Variant = Filler;
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Filler), Error> {
        let d: StrDeserializer<Error> = self.name.into_deserializer();
        Ok((seed.deserialize(d)?, self.filler))
    }
}

impl<'de> de::VariantAccess<'de> for Filler {
    type Error = Error;
    fn unit_variant(self) -> Result<(), Error> {
        Ok(())
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(self)
    }
    fn tuple_variant<V: Visitor<'de>>(self, _: usize, _: V) -> Result<V::Value, Error> {
        Err(de::Error::custom("no tuple variant in an event"))
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        v: V,
    ) -> Result<V::Value, Error> {
        de::Deserializer::deserialize_struct(self, "", fields, v)
    }
}
