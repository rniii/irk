use serde::{Deserialize, Serialize};

pub mod de;
pub mod ser;

pub use de::Deserializer;
pub use ser::Serializer;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Command<'a> {
    Cap(Cap<'a>),
    Authenticate {
        mechanism: &'a str,
    },
    Pong {
        server: Option<&'a str>,
        token: &'a str,
    },
    Nick {
        nickname: &'a str,
    },
    User {
        username: &'a str,
        realname: &'a str,
    },
    Kick {
        channel: &'a str,
        users: Vec<&'a str>,
        reason: Option<&'a str>,
    },
    Topic {
        channel: &'a str,
        topic: Option<&'a str>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Cap<'a> {
    Req { caps: &'a str },
    End,
}
