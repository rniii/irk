pub mod proto;

pub use proto::{ser::Serializer, Command};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Message<'a> {
    source: Option<&'a str>,
    command: &'a str,
    parameters: Vec<&'a str>,
}

impl std::fmt::Display for Message<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(src) = self.source {
            write!(f, ":{src} ")?;
        }

        write!(f, "{}", self.command)?;

        if let Some((last, rest)) = self.parameters.split_last() {
            rest.iter().try_for_each(|v| write!(f, " {v}"))?;
            write!(f, " :{last}")?;
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Eof,
    InvalidType,
    Serialize(String),
    Deserialize(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Eof => write!(f, "Unexpected end of input"),
            Error::InvalidType => write!(f, "Unsupported type"),
            Error::Serialize(e) => write!(f, "Serialize error: {e}"),
            Error::Deserialize(e) => write!(f, "Deserialize error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Serialize(msg.to_string())
    }
}

struct Lexer<'a> {
    input: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input }
    }

    fn current(&self) -> Option<char> {
        self.input.chars().next()
    }

    fn read_part(&mut self) -> &'a str {
        let (part, input) = self
            .input
            .split_once(' ')
            .unwrap_or((self.input, &self.input[..0]));
        self.input = input.trim_start_matches(' ');
        part
    }

    fn parse(&mut self) -> Result<Message<'a>, Error> {
        let source = match self.current() {
            Some(':') => {
                self.input = &self.input[1..];
                Some(self.read_part())
            }
            _ => None,
        };

        let command = self.read_part();

        let mut parameters = Vec::new();
        loop {
            match self.current() {
                Some(':') => break parameters.push(&self.input[1..]),
                Some(_) => parameters.push(self.read_part()),
                None => break,
            }
        }

        Ok(Message {
            source,
            command,
            parameters,
        })
    }
}

impl<'a> TryFrom<&'a str> for Message<'a> {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Lexer::new(value).parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_parse {
        ($name:ident; $($in:literal => $out:expr),*) => {
            #[test]
            fn $name() {
                $(assert_eq!($in.try_into(), $out);)*
            }
        };
    }

    test_parse! {
        parse_rfc;
        ":irc.example.com CAP * LIST :" => Ok(Message {
            source: Some("irc.example.com"),
            command: "CAP",
            parameters: vec!["*", "LIST", ""],
        }),
        "CAP * LS :multi-prefix sasl" => Ok(Message {
            source: None,
            command: "CAP",
            parameters: vec!["*", "LS", "multi-prefix sasl"],
        }),
        "CAP REQ :sasl message-tags foo" => Ok(Message {
            source: None,
            command: "CAP",
            parameters: vec!["REQ", "sasl message-tags foo"],
        })
    }
}
