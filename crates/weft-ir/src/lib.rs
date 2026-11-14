//! Weft IR v0: the program graph that every front end emits and every backend consumes.
//!
//! See `docs/ir-v0.md` for semantics. In short, a program is:
//! - **locations**: places code runs, described by capabilities (not device types)
//! - **state**: typed durable cells, each owned by exactly one location
//! - **messages**: typed records, the only way data moves between locations
//! - **handlers**: straight-line SSA bodies with *guarded* effects, triggered by a
//!   message, an outside input, or a timer
//! - **externs**: escape hatches with declared effects and a required simulation model
//! - **properties**: what the simulator checks (`always`, `eventually`, `monotonic`)
//! - **scenarios**: test harness for the simulator; never lowered
//!
//! v0 is serialized as JSON. An MLIR dialect can replace the encoding later
//! without changing these semantics.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

mod pretty;
mod validate;

pub use pretty::pretty;
pub use validate::validate;

pub const IR_VERSION: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Int,
    Bool,
    /// A reference to one replica of a location (e.g. the sender of a message).
    Loc,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Type::Int => "int",
            Type::Bool => "bool",
            Type::Loc => "loc",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub weft_ir: u32,
    pub name: String,
    pub locations: Vec<Location>,
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub state: Vec<StateDecl>,
    #[serde(default)]
    pub externs: Vec<Extern>,
    #[serde(default)]
    pub handlers: Vec<Handler>,
    #[serde(default)]
    pub properties: Vec<Property>,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
}

impl Program {
    pub fn location(&self, name: &str) -> Option<(usize, &Location)> {
        self.locations.iter().enumerate().find(|(_, l)| l.name == name)
    }
    pub fn message(&self, name: &str) -> Option<&Message> {
        self.messages.iter().find(|m| m.name == name)
    }
    pub fn state_decl(&self, name: &str) -> Option<&StateDecl> {
        self.state.iter().find(|s| s.name == name)
    }
    pub fn extern_decl(&self, name: &str) -> Option<&Extern> {
        self.externs.iter().find(|e| e.name == name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    #[serde(default = "one")]
    pub replicas: usize,
    #[serde(default)]
    pub caps: Vec<String>,
}

impl Location {
    pub fn has_cap(&self, cap: &str) -> bool {
        self.caps.iter().any(|c| c == cap)
    }
}

fn one() -> usize {
    1
}

/// `(name, type)`, serialized as `["version", "int"]`.
pub type Field = (String, Type);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub name: String,
    pub fields: Vec<Field>,
}

impl Message {
    pub fn field(&self, name: &str) -> Option<(usize, Type)> {
        let i = self.fields.iter().position(|(n, _)| n == name)?;
        Some((i, self.fields[i].1))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDecl {
    /// Qualified: `<location>.<name>`, e.g. `device.installed`.
    pub name: String,
    pub location: String,
    #[serde(rename = "type")]
    pub ty: Type,
    pub init: serde_json::Value,
}

/// An escape hatch: code the compiler can't see into. It must declare which
/// capabilities it uses (`effects`) and how it behaves in simulation (`sim`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extern {
    pub name: String,
    pub location: String,
    pub params: Vec<Field>,
    pub returns: Type,
    pub effects: Vec<String>,
    /// Optional only so the validator, not the parser, reports a missing model.
    #[serde(default)]
    pub sim: Option<SimModel>,
}

/// Behavioral models the simulator substitutes for an extern's real implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "snake_case")]
pub enum SimModel {
    /// Returns `ok`, or `err` with probability `fail_rate`, after a uniform latency.
    Fallible {
        fail_rate: f64,
        latency_ms: [u64; 2],
        ok: serde_json::Value,
        err: serde_json::Value,
    },
    /// Always returns `value` after a uniform latency.
    Constant {
        value: serde_json::Value,
        latency_ms: [u64; 2],
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handler {
    pub name: String,
    pub location: String,
    pub trigger: Trigger,
    pub body: Vec<Op>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Trigger {
    /// A message from another location.
    Message { message: String },
    /// A message from outside the system (an operator, a user, a test scenario).
    Input { message: String },
    /// Periodic; requires the `clock` capability.
    Timer { every_ms: u64 },
}

/// One SSA op. `id` is its index in the body; args refer to earlier ids.
/// Effects (`call`, `write`, `send`) may carry a `guard`: a bool op id that must be
/// true for the effect to happen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Op {
    pub id: usize,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub ty: Option<Type>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<usize>,
    #[serde(flatten)]
    pub kind: OpKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum OpKind {
    // ---- pure
    Const { value: serde_json::Value },
    /// A field of the triggering message.
    Field { name: String },
    /// The replica that sent the triggering message.
    Sender,
    Read { state: String },
    Bin {
        #[serde(rename = "fn")]
        func: BinFn,
        args: [usize; 2],
    },
    Not { args: [usize; 1] },
    Select { args: [usize; 3] },
    // ---- effects
    Call {
        #[serde(rename = "extern")]
        ext: String,
        args: Vec<usize>,
    },
    Write { state: String, args: [usize; 1] },
    Send {
        to: Target,
        message: String,
        fields: BTreeMap<String, usize>,
    },
}

impl OpKind {
    pub fn is_effect(&self) -> bool {
        matches!(self, OpKind::Call { .. } | OpKind::Write { .. } | OpKind::Send { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BinFn {
    Add,
    Sub,
    Mul,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinFn {
    pub fn name(self) -> &'static str {
        use BinFn::*;
        match self {
            Add => "add",
            Sub => "sub",
            Mul => "mul",
            Eq => "eq",
            Ne => "ne",
            Lt => "lt",
            Le => "le",
            Gt => "gt",
            Ge => "ge",
            And => "and",
            Or => "or",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// Every replica of a location: `{"location": "device"}`.
    Location(String),
    /// One replica, from a `loc` value: `{"value": 3}`.
    Value(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    #[serde(flatten)]
    pub kind: PropertyKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PropertyKind {
    /// Holds after every event. `forall` names the replicated location quantified over.
    Always {
        #[serde(default)]
        forall: Option<String>,
        body: Vec<Op>,
        result: usize,
    },
    /// Holds at the end of the run (after faults heal).
    Eventually {
        #[serde(default)]
        forall: Option<String>,
        body: Vec<Op>,
        result: usize,
    },
    /// An int state never decreases, on any replica.
    Monotonic { state: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub duration_ms: u64,
    /// After this time the network stops dropping messages.
    #[serde(default)]
    pub heal_at_ms: Option<u64>,
    #[serde(default)]
    pub network: Network,
    #[serde(default)]
    pub inputs: Vec<Input>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    #[serde(default)]
    pub drop_rate: f64,
    #[serde(default = "default_latency")]
    pub latency_ms: [u64; 2],
}

impl Default for Network {
    fn default() -> Self {
        Network { drop_rate: 0.0, latency_ms: default_latency() }
    }
}

fn default_latency() -> [u64; 2] {
    [1, 1]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub at_ms: u64,
    pub location: String,
    /// `None` delivers to every replica.
    #[serde(default)]
    pub replica: Option<usize>,
    pub message: String,
    #[serde(default)]
    pub fields: BTreeMap<String, serde_json::Value>,
}

/// Whether a JSON constant is a valid literal of `ty`. `loc` has no literals.
pub fn const_fits(v: &serde_json::Value, ty: Type) -> bool {
    match ty {
        Type::Int => v.is_i64(),
        Type::Bool => v.is_boolean(),
        Type::Loc => false,
    }
}
