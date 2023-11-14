use serde::de::{self, Visitor};

use crate::{Error, Result};

pub struct Deserializer<'de> {
    input: (Option<&'de str>, Vec<&'de str>),
    fields: usize,
}

impl<'de> Deserializer<'de> {
    pub fn from_message(mut msg: crate::Message<'de>) -> Self {
        msg.parameters.reverse();
        Self {
            input: (Some(msg.command), msg.parameters),
            fields: 0,
        }
    }
}

impl<'de> Deserializer<'de> {
    fn read_part(&mut self) -> Result<&'de str> {
        if let Some(p) = self.input.0.take() {
            Ok(p)
        } else {
            self.input.1.pop().ok_or(Error::Eof)
        }
    }

    fn available(&self) -> usize {
        self.input.0.is_some() as usize + self.input.1.len()
    }
}

macro_rules! visits_fromstr {
    ($($fun:ident:$visitor:ident)*) => {
        $(fn $fun<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            visitor.$visitor(self.read_part()?.parse()?)
        })*
    };
}

macro_rules! unsupported {
    ($($fun:ident)*) => {
        $(fn $fun<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
            Err(Error::UnsupportedType)
        })*
    };
}

macro_rules! forward_tuple {
    ($($fun:ident($($param:ident: $type:ty),*) { $len:expr })*) => {
        $(fn $fun<V: Visitor<'de>>(self, $($param: $type,)* visitor: V) -> Result<V::Value> {
            de::Deserializer::deserialize_tuple(self, $len, visitor)
        })*
    };
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    visits_fromstr! {
        deserialize_i8:visit_i8 deserialize_i16:visit_i16 deserialize_i32:visit_i32 deserialize_i64:visit_i64
        deserialize_u8:visit_u8 deserialize_u16:visit_u16 deserialize_u32:visit_u32 deserialize_u64:visit_u64
        deserialize_f32:visit_f32 deserialize_f64:visit_f64 deserialize_char:visit_char deserialize_bool:visit_bool
    }

    unsupported! {
        deserialize_any deserialize_map deserialize_ignored_any
    }

    forward_tuple! {
        deserialize_tuple_struct(_name: &'static str, len: usize) { len }
        deserialize_struct(_name: &'static str, fields: &'static [&'static str]) { fields.len() }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.fields += len;
        visitor.visit_seq(self)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_borrowed_str(self.read_part()?)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_borrowed_str(self.read_part()?)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_bytes(self.read_part()?.as_bytes())
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_bytes(self.read_part()?.as_bytes())
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.available() >= self.fields {
            visitor.visit_some(self)
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(Sequence(self))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_enum(self)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_borrowed_str(self.read_part()?)
    }
}

impl<'de, 'a> de::EnumAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        Ok((seed.deserialize(&mut *self)?, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    forward_tuple! {
        tuple_variant(len: usize) { len }
        struct_variant(fields: &'static [&'static str]) { fields.len() }
    }
}

impl<'de, 'a> de::SeqAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        let v = seed.deserialize(&mut **self)?;
        self.fields -= 1;
        Ok(Some(v))
    }
}

struct Sequence<'de, 'a>(&'a mut Deserializer<'de>);

impl<'de, 'a> de::SeqAccess<'de> for Sequence<'de, 'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        // okay so,
        let part = self.0.read_part()?;
        if part.is_empty() {
            return Ok(None);
        }

        let (p, rest) = part.split_once(',').unwrap_or((part, ""));

        self.0.input.0 = Some(p);
        let v = seed.deserialize(&mut *self.0).map(Some);
        self.0.input.0 = Some(rest);
        v
    }
}
