use serde::{ser, Serialize};

use crate::{Error, Message};

#[derive(Debug, Default)]
pub struct Serializer {
    pub args: Vec<Box<str>>,
}

#[derive(Debug)]
pub struct Sequence<'a>(&'a mut Serializer, Vec<Box<str>>);

impl Serializer {
    pub fn new<T: Serialize>(value: T) -> Result<Self, Error> {
        let mut ser = Self::default();
        value.serialize(&mut ser)?;
        Ok(ser)
    }

    pub fn argument<T: Serialize>(mut self, value: T) -> Result<Self, Error> {
        value.serialize(&mut self)?;
        Ok(self)
    }

    pub fn to_message(&self) -> Result<Message, Error> {
        let (command, param) = self.args.split_first().ok_or(Error::Eof)?;
        Ok(Message {
            source: None,
            command,
            parameters: param.iter().map(|c| c.as_ref()).collect(),
        })
    }
}

macro_rules! pushes {
    ($($f:ident($($p:ident: $t:ty),*) { $v:expr })*) => {
        $(fn $f(self, $($p: $t),*) -> Result<Self::Ok, Self::Error> {
            self.args.push($v.into());
            Ok(())
        })*
    };
}

macro_rules! pushes_string {
    ($($f:ident($t:ty))*) => {
        pushes! { $($f(v: $t) { v.to_string() })* }
    };
}

macro_rules! noop {
    ($($f:ident($($t:ty),*))*) => {
        $(fn $f(self, $(_: $t),*) -> Result<Self::Ok, Self::Error> {
            Ok(())
        })*
    };
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Sequence<'a>;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    pushes! {
        serialize_bool(v: bool) { if v { "1" } else { "0" } }
        serialize_str(v: &str) { v }
        serialize_bytes(v: &[u8]) { std::str::from_utf8(v).unwrap() }
        serialize_unit_variant(_name: &'static str, _index: u32, variant: &'static str) {
            variant
        }
    }

    pushes_string! {
        serialize_i8(i8) serialize_i16(i16) serialize_i32(i32) serialize_i64(i64)
        serialize_u8(u8) serialize_u16(u16) serialize_u32(u32) serialize_u64(u64)
        serialize_f32(f32) serialize_f64(f64) serialize_char(char)
    }

    noop! {
        serialize_unit() serialize_none() serialize_unit_struct(&'static str)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.args.push(variant.into());
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(Sequence(self, Vec::new()))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.args.push(variant.into());
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Error::InvalidType)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.args.push(variant.into());
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for Sequence<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let mut ser = Serializer::default();
        value.serialize(&mut ser)?;
        self.1.extend(ser.args);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.args.push(self.1.join(",").into());
        Ok(())
    }
}

macro_rules! serialize_fields {
    ($($trait:ident::$fun:ident($($type:ty),*))*) => {
        $(impl<'a> ser::$trait for &'a mut Serializer {
            type Ok = ();
            type Error = Error;

            fn $fun<T>(&mut self, $(_: $type,)* value: &T) -> Result<(), Self::Error>
            where
                T: Serialize + ?Sized,
            {
                value.serialize(&mut **self)
            }

            fn end(self) -> Result<Self::Ok, Self::Error> {
                Ok(())
            }
        })*
    };
}

serialize_fields! {
    SerializeTuple::serialize_element()
    SerializeTupleStruct::serialize_field()
    SerializeTupleVariant::serialize_field()
    SerializeStruct::serialize_field(&'static str)
    SerializeStructVariant::serialize_field(&'static str)
}
