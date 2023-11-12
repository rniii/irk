use serde::{ser, Serialize};

use crate::{Error, Message, Result};

#[derive(Debug, Default)]
pub struct Serializer {
    pub args: Vec<Box<str>>,
}

#[derive(Debug)]
pub struct Sequence<'a>(&'a mut Serializer, Vec<Box<str>>);

impl Serializer {
    pub fn new<T: Serialize>(value: T) -> Result<Self> {
        let mut ser = Self::default();
        value.serialize(&mut ser)?;
        Ok(ser)
    }

    pub fn argument<T: Serialize>(mut self, value: T) -> Result<Self> {
        value.serialize(&mut self)?;
        Ok(self)
    }

    pub fn to_message(&self) -> Result<Message> {
        let (command, param) = self.args.split_first().ok_or(Error::Eof)?;
        Ok(Message {
            source: None,
            command,
            parameters: param.iter().map(|c| c.as_ref()).collect(),
        })
    }
}

macro_rules! pushes {
    ($($fun:ident($($param:ident: $type:ty),*) { $val:expr })*) => {
        $(fn $fun(self, $($param: $type),*) -> Result<()> {
            self.args.push($val.into());
            Ok(())
        })*
    };
}

macro_rules! pushes_string {
    ($($fun:ident($type:ty))*) => {
        pushes! { $($fun(v: $type) { v.to_string() })* }
    };
}

macro_rules! noop {
    ($($fun:ident($($type:ty),*))*) => {
        $(fn $fun(self, $(_: $type),*) -> Result<()> {
            Ok(())
        })*
    };
}

macro_rules! serializes_self {
    ($($fun:ident($($type:ty),*))*) => {
        $(fn $fun(self, $(_: $type),*) -> Result<Self> {
            Ok(self)
        })*
    };
}

macro_rules! forwards_self {
    ($($fun:ident($($type:ty),*))*) => {
        $(fn $fun<T: Serialize + ?Sized>(self, $(_: $type,)* value: &T) -> Result<()> {
            value.serialize(self)
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
        serialize_str(v: &str) { v }
        serialize_bytes(v: &[u8]) { std::str::from_utf8(v).unwrap() }
        serialize_unit_variant(_name: &'static str, _index: u32, variant: &'static str) {
            variant
        }
    }

    pushes_string! {
        serialize_i8(i8) serialize_i16(i16) serialize_i32(i32) serialize_i64(i64)
        serialize_u8(u8) serialize_u16(u16) serialize_u32(u32) serialize_u64(u64)
        serialize_f32(f32) serialize_f64(f64) serialize_char(char) serialize_bool(bool)
    }

    noop! {
        serialize_unit() serialize_none() serialize_unit_struct(&'static str)
    }

    forwards_self! {
        serialize_some() serialize_newtype_struct(&'static str)
    }

    serializes_self! {
        serialize_tuple(usize) serialize_tuple_struct(&'static str, usize)
        serialize_struct(&'static str, usize)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Sequence<'a>> {
        Ok(Sequence(self, Vec::new()))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<ser::Impossible<(), Error>> {
        Err(Error::UnsupportedType)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.args.push(variant.into());
        value.serialize(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.args.push(variant.into());
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.args.push(variant.into());
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for Sequence<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        let mut ser = Serializer::default();
        value.serialize(&mut ser)?;
        self.1.extend(ser.args);

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.0.args.push(self.1.join(",").into());
        Ok(())
    }
}

macro_rules! serialize_fields {
    ($($trait:ident::$fun:ident($($type:ty),*))*) => {
        $(impl<'a> ser::$trait for &'a mut Serializer {
            type Ok = ();
            type Error = Error;

            fn $fun<T>(&mut self, $(_: $type,)* value: &T) -> Result<()>
            where
                T: Serialize + ?Sized,
            {
                value.serialize(&mut **self)
            }

            fn end(self) -> Result<()> {
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
