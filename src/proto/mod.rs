use serde::{Deserialize, Serialize};

pub mod ser;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Command<'a> {
    Cap(Cap<'a>),
    Authenticate {
        mechanism: &'a str,
    },
    Nick {
        nickname: &'a str,
    },
    User {
        username: &'a str,
        realname: &'a str,
    },
}

#[derive(Serialize, Deserialize)]
pub enum Cap<'a> {
    Req { caps: &'a str },
    End,
}
