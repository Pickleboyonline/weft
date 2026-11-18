//! Effects: what a function does besides compute its result.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effect {
    /// Reads a table.
    Read(String),
    /// Writes a table.
    Write(String),
    Time,
    Random,
    /// `sleep(…)`.
    Wait,
    /// `.await`.
    Await,
    Navigate,
    Status,
    /// A call to another service: `http.get`, a socket.
    Outside(String),
    Secret,
    Dom,
    Browser,
    /// Assigns a component's `state`.
    Assign(String),
    /// Sends a command from client code.
    Send(String),
    /// Calls a query from client code.
    CallQuery(String),
    Extern(String),
    /// Uses a user-defined effect.
    User(String),
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effect::Read(t) => write!(f, "reads {t}"),
            Effect::Write(t) => write!(f, "writes {t}"),
            Effect::Time => write!(f, "time"),
            Effect::Random => write!(f, "random"),
            Effect::Wait => write!(f, "sleeps"),
            Effect::Await => write!(f, "waits (.await)"),
            Effect::Navigate => write!(f, "navigates"),
            Effect::Status => write!(f, "reads loading status"),
            Effect::Outside(c) => write!(f, "calls {c}"),
            Effect::Secret => write!(f, "reads secrets"),
            Effect::Dom => write!(f, "uses the dom"),
            Effect::Browser => write!(f, "uses the browser"),
            Effect::Assign(s) => write!(f, "assigns {s}"),
            Effect::Send(c) => write!(f, "sends {c}"),
            Effect::CallQuery(q) => write!(f, "calls {q}"),
            Effect::Extern(e) => write!(f, "calls extern {e}"),
            Effect::User(e) => write!(f, "uses {e}"),
        }
    }
}

impl Effect {
    /// How the effect reads in an error message: "can't {verb}".
    pub fn verb(&self) -> String {
        match self {
            Effect::Read(t) => format!("read `{t}`"),
            Effect::Write(t) => format!("write `{t}`"),
            Effect::Time => "read the clock".into(),
            Effect::Random => "use randomness".into(),
            Effect::Wait => "sleep".into(),
            Effect::Await => "wait with `.await`".into(),
            Effect::Navigate => "navigate".into(),
            Effect::Status => "read loading status".into(),
            Effect::Outside(c) => format!("call `{c}`"),
            Effect::Secret => "read secrets".into(),
            Effect::Dom => "use the dom".into(),
            Effect::Browser => "use the browser".into(),
            Effect::Assign(s) => format!("assign `{s}`"),
            Effect::Send(c) => format!("send the command `{c}`"),
            Effect::CallQuery(q) => format!("call the query `{q}`"),
            Effect::Extern(e) => format!("call the extern `{e}`"),
            Effect::User(e) => format!("use the effect `{e}`"),
        }
    }
}
