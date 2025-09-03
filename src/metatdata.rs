use std::fmt::{self, Display};
use serde::de::{Deserialize, Visitor};
use serde::{de, ser};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess};

// Errors ---------------------------------------------------------------------
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Eof,
    Syntax,
    ExpectedBoolean,
    ExpectedString,
    ExpectedStringTerminal,
    ExpectedArray,
    ExpectedArrayEnd,
    TrailingCharacters,
    ExpectedArrayComma,
    ExpectedTuple,
    ExpectedTupleEnd,
    ExpectedMap,
    ExpectedMapEnd,
    ExpectedMapComma,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::Eof => formatter.write_str("unexpected end of input"),
            Error::Syntax => formatter.write_str("invalid syntax"),
            Error::ExpectedBoolean => formatter.write_str("expected boolean (true|false)"),
            Error::ExpectedString => formatter.write_str("expected string: (<<\"string\">>|<<\"string\"/uft-8>>)"),
            Error::ExpectedStringTerminal => formatter.write_str("expected end of string (\">>|\"/uft-8>>)"),
            Error::ExpectedArray => formatter.write_str("expected array"),
            Error::ExpectedArrayEnd => formatter.write_str("expected end of array (])"),
            Error::TrailingCharacters => formatter.write_str("parse finished with content in the buffer"),
            Error::ExpectedArrayComma => formatter.write_str("expected array comma"),
            Error::ExpectedTupleEnd => formatter.write_str("expected tuple to end"),
            Error::ExpectedTuple => formatter.write_str("expected tuple"),
            Error::ExpectedMapComma => formatter.write_str("expected map comma"),
            Error::ExpectedMap => formatter.write_str("expected map"),
            Error::ExpectedMapEnd => formatter.write_str("expected map end (})"),
        }
    }
}

impl std::error::Error for Error {}


// Deserialization ------------------------------------------------------------

pub struct Deserializer<'de> {
    input: &'de str,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Self { input: input }
    }
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;

    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

impl<'de> Deserializer<'de> {
    fn peek_char(&mut self) -> Result<char> {
        self.input.chars().next().ok_or(Error::Eof)
    }

    fn next_char(& mut self) -> Result<char>{
        let ch = self.peek_char()?;
        self.input = &self.input[ch.len_utf8()..];
        Ok(ch)
    }

    fn parse_bool(&mut self) -> Result<bool> {
        if self.input.starts_with("true") {
            self.input = &self.input["true".len()..];
            Ok(true)
        } else if self.input.starts_with("false") {
            self.input = &self.input["false".len()..];
            Ok(false)
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    fn starts_with_read(&mut self, other: &str) -> bool {
        let starts = self.input.starts_with(other);
        if starts {
            self.input = &self.input[other.len()..]
        }
        starts
    }

    fn clear_whitespace(&mut self) -> Result<char> {
        let mut ch = self.peek_char()?;
        while ch.is_whitespace() {
            self.next_char()?;
            ch = self.peek_char()?;
        }
        Ok(ch)
    }

    fn parse_string(&mut self) -> Result<&'de str> {
        if !self.starts_with_read("<<\"") {
            return Err(Error::ExpectedString);
        }

        match self.input.find('"') {
            Some(len) => {
                let s = &self.input[..len];
                let save = self.input;
                self.input = &self.input[len..];
                if self.starts_with_read("\">>") {
                    Ok(s)
                } else if self.starts_with_read("\"/utf-8>>") {
                    Ok(s)
                } else {
                    self.input = save;
                    Err(Error::ExpectedStringTerminal)
                }
            }
            None => Err(Error::Eof),
        }
    }
}


impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        match self.peek_char()? {
            't' | 'f' => self.deserialize_bool(visitor),
            '<' => self.deserialize_str(visitor),
            '{' => self.deserialize_tuple(2, visitor),
            _ => Err(Error::Syntax),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        visitor.visit_borrowed_str(self.parse_string()?)
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
            unimplemented!()
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
            unimplemented!()
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        self.clear_whitespace()?;

        if self.next_char()? == '[' {
            self.clear_whitespace()?;

            let value = visitor.visit_seq(CommaSeparated::new(self, ']'))?;
            if self.next_char()? == ']' {
                Ok(value)
            } else {
                Err(Error::ExpectedArrayEnd)
            }
        } else {
            Err(Error::ExpectedArray)
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        if self.next_char()? == '{' {
            let value = visitor.visit_seq(CommaSeparatedFixed::new(self, len))?;

            self.clear_whitespace()?;

            if self.next_char()? == '}' {
                Ok(value)
            }
            else {
                Err(Error::ExpectedTupleEnd)
            }
        } else {
            Err(Error::ExpectedTuple)
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de> {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        if self.next_char()? == '[' {
            let value = visitor.visit_map(CommaSeparated::new(self, ']'))?;
            if self.next_char()? == ']' {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedMap)
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de> {
        unimplemented!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de> {
        self.deserialize_any(visitor)
    }
}

// sequences ------------------------------------------------------------------

struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
    end: char,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, end: char) -> Self {
        CommaSeparated {
            de,
            first: true,
            end: end
        }
    }
}

impl<'de, 'a> SeqAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.de.clear_whitespace()? == self.end {
            return Ok(None);
        }

        if !self.first && self.de.next_char()? != ',' {
            return Err(Error::ExpectedArrayComma);
        }

        self.de.clear_whitespace()?;

        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

// tuples ---------------------------------------------------------------------

struct CommaSeparatedFixed<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
    size: usize,
    seen: usize,
}

impl<'a, 'de> CommaSeparatedFixed<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, size: usize) -> Self {
        CommaSeparatedFixed {
            de,
            first: true,
            size: size,
            seen: 0,
        }
    }
}

impl<'de, 'a> SeqAccess<'de> for CommaSeparatedFixed<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        let ch = self.de.clear_whitespace()?;

        if self.seen == self.size && ch == '}' {
            return Ok(None);
        } else if self.seen == self.size {
            return Err(Error::ExpectedTupleEnd);
        } else if !self.first && self.de.next_char()? != ',' {
            return Err(Error::ExpectedArrayComma);
        }

        self.de.clear_whitespace()?;

        self.first = false;
        self.seen += 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

// maps -----------------------------------------------------------------------

impl<'de, 'a> MapAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.de.clear_whitespace()? == ']' {
            return Ok(None);
        }

        if !self.first && self.de.next_char()? != ',' {
            return Err(Error::ExpectedArrayComma);
        }

        self.de.clear_whitespace()?;

        if self.de.next_char()? != '{' {
            return Err(Error::ExpectedTuple);
        }

        self.de.clear_whitespace()?;

        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.clear_whitespace()?;

        if self.de.next_char()? != ',' {
            return Err(Error::ExpectedMapComma);
        }

        self.de.clear_whitespace()?;

        let val = seed.deserialize(&mut *self.de);

        self.de.clear_whitespace()?;

        if self.de.next_char()? != '}' {
            return Err(Error::ExpectedTupleEnd)
        }

        val
    }
}

// tests ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::metatdata::from_str;

    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Requirement<'a> {
        app: &'a str,
        optional: bool,
        requirement: &'a str,
    }

    #[test]
    fn test_true() {
        assert_eq!(true, from_str::<bool>("true").unwrap())
    }

    #[test]
    fn test_false() {
        assert_eq!(false, from_str::<bool>("false").unwrap())
    }

    #[test]
    fn test_string() {
        let s = "<<\"basic string\">>";
        let expected = "basic string";
        assert_eq!(expected, from_str::<&str>(s).unwrap())
    }

    #[test]
    fn test_string_utf() {
        let s = "<<\"basic string\"/utf-8>>";
        let expected = "basic string";
        assert_eq!(expected, from_str::<&str>(s).unwrap())
    }

    #[test]
    fn test_pair_no_white() {
        let s = r#"{<<"name">>,<<"gleam_erlang">>}"#;
        let expected = ("name", "gleam_erlang");
        assert_eq!(expected, from_str::<(&str, &str)>(s).unwrap())
    }

    #[test]
    fn test_pair_normal_white() {
        let s = r#"{<<"name">>, <<"gleam_erlang">>}"#;
        let expected = ("name", "gleam_erlang");
        assert_eq!(expected, from_str::<(&str, &str)>(s).unwrap())
    }

    #[test]

    fn test_pair_excess_white() {
        let s = r#"{ <<"name">>   ,      <<"gleam_erlang">> }"#;
        let expected = ("name", "gleam_erlang");
        assert_eq!(expected, from_str::<(&str, &str)>(s).unwrap())
    }

    #[test]
    fn test_requirement() {
        let s =
            r#"[
                {<<"app">>, <<"gleam_stdlib">>},
                {<<"optional">>, false},
                {<<"requirement">>, <<">= 0.33.0 and < 2.0.0">>}
            ]"#;
        let expected = Requirement {
            app: "gleam_stdlib",
            optional: false,
            requirement: ">= 0.33.0 and < 2.0.0",
        };

        assert_eq!(expected, from_str(s).unwrap());
    }

    #[test]
    fn test_seq() {
        let s = r#"[true,false,true]"#;
        let expected = vec![true, false, true];
        assert_eq!(expected, from_str::<Vec<bool>>(s).unwrap());
    }

    #[test]
    fn test_seq_normal_white() {
        let s = r#"[true, false, true]"#;
        let expected = vec![true, false, true];
        assert_eq!(expected, from_str::<Vec<bool>>(s).unwrap());
    }

    #[test]
    fn test_seq_excess_white() {
        let s = r#"[ true , false , true ]"#;
        let expected = vec![true, false, true];
        assert_eq!(expected, from_str::<Vec<bool>>(s).unwrap());
    }
}