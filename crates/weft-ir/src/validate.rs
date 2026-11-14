//! Structural and type checks. Front ends should catch most of these with nicer
//! messages, but the IR is the contract: nothing reaches a backend or the
//! simulator without passing here.

use crate::*;
use std::collections::{BTreeSet, HashSet};

/// Check a program. Returns every problem found, not just the first.
pub fn validate(p: &Program) -> Result<(), Vec<String>> {
    let mut v = Validator { p, errs: Vec::new() };
    v.program();
    if v.errs.is_empty() {
        Ok(())
    } else {
        Err(v.errs)
    }
}

struct Validator<'a> {
    p: &'a Program,
    errs: Vec<String>,
}

/// What a body may touch.
struct Scope<'a> {
    what: String,
    /// `None` for properties: pure, may read any location.
    location: Option<&'a Location>,
    message: Option<&'a Message>,
    sender: bool,
}

fn ty_name(t: Option<Type>) -> String {
    t.map(|t| t.to_string()).unwrap_or_else(|| "?".into())
}

impl<'a> Validator<'a> {
    fn err(&mut self, msg: String) {
        self.errs.push(msg);
    }

    fn unique<'b>(&mut self, what: &str, names: impl Iterator<Item = &'b str>) {
        let mut seen = HashSet::new();
        for n in names {
            if !seen.insert(n) {
                self.err(format!("duplicate {what} `{n}`"));
            }
        }
    }

    fn program(&mut self) {
        let p = self.p;
        if p.weft_ir != IR_VERSION {
            self.err(format!("weft_ir version {} is not supported (this build reads {IR_VERSION})", p.weft_ir));
        }
        self.unique("location", p.locations.iter().map(|x| x.name.as_str()));
        self.unique("message", p.messages.iter().map(|x| x.name.as_str()));
        self.unique("state", p.state.iter().map(|x| x.name.as_str()));
        self.unique("extern", p.externs.iter().map(|x| x.name.as_str()));
        self.unique("handler", p.handlers.iter().map(|x| x.name.as_str()));
        self.unique("property", p.properties.iter().map(|x| x.name.as_str()));
        self.unique("scenario", p.scenarios.iter().map(|x| x.name.as_str()));

        for l in &p.locations {
            if l.replicas == 0 {
                self.err(format!("location `{}` has 0 replicas", l.name));
            }
        }
        for m in &p.messages {
            self.unique(&format!("field of message `{}`", m.name), m.fields.iter().map(|f| f.0.as_str()));
        }

        for s in &p.state {
            match p.location(&s.location) {
                None => self.err(format!("state `{}`: unknown location `{}`", s.name, s.location)),
                Some((_, l)) if !l.has_cap("storage") => self.err(format!(
                    "state `{}` lives on `{}`, which lacks the `storage` capability",
                    s.name, l.name
                )),
                _ => {}
            }
            if !s.name.starts_with(&format!("{}.", s.location)) {
                self.err(format!("state `{}`: names are qualified, expected `{}.<name>`", s.name, s.location));
            }
            if !const_fits(&s.init, s.ty) {
                self.err(format!("state `{}`: initial value {} is not a {}", s.name, s.init, s.ty));
            }
        }

        for e in &p.externs {
            match p.location(&e.location) {
                None => self.err(format!("extern `{}`: unknown location `{}`", e.name, e.location)),
                Some((_, l)) => {
                    for eff in &e.effects {
                        if !l.has_cap(eff) {
                            self.err(format!(
                                "extern `{}` declares effect `{eff}`, but location `{}` lacks that capability",
                                e.name, l.name
                            ));
                        }
                    }
                }
            }
            match &e.sim {
                None => self.err(format!(
                    "extern `{}` has no simulation model; every escape hatch must say how it behaves in simulation",
                    e.name
                )),
                Some(SimModel::Fallible { fail_rate, latency_ms, ok, err }) => {
                    if !(0.0..=1.0).contains(fail_rate) {
                        self.err(format!("extern `{}`: fail_rate must be in [0, 1]", e.name));
                    }
                    if latency_ms[0] > latency_ms[1] {
                        self.err(format!("extern `{}`: latency range is backwards", e.name));
                    }
                    if !const_fits(ok, e.returns) || !const_fits(err, e.returns) {
                        self.err(format!("extern `{}`: sim model values must be {}", e.name, e.returns));
                    }
                }
                Some(SimModel::Constant { value, latency_ms }) => {
                    if latency_ms[0] > latency_ms[1] {
                        self.err(format!("extern `{}`: latency range is backwards", e.name));
                    }
                    if !const_fits(value, e.returns) {
                        self.err(format!("extern `{}`: sim model value must be {}", e.name, e.returns));
                    }
                }
            }
        }

        let mut triggers = HashSet::new();
        for h in &p.handlers {
            let Some((_, loc)) = p.location(&h.location) else {
                self.err(format!("handler `{}`: unknown location `{}`", h.name, h.location));
                continue;
            };
            let (message, sender) = match &h.trigger {
                Trigger::Message { message } | Trigger::Input { message } => {
                    let input = matches!(h.trigger, Trigger::Input { .. });
                    if !triggers.insert((h.location.as_str(), message.as_str(), input)) {
                        self.err(format!(
                            "two handlers on `{}` for {} `{message}`",
                            h.location,
                            if input { "input" } else { "message" }
                        ));
                    }
                    match p.message(message) {
                        Some(m) => (Some(m), !input),
                        None => {
                            self.err(format!("handler `{}`: unknown message `{message}`", h.name));
                            continue;
                        }
                    }
                }
                Trigger::Timer { every_ms } => {
                    if *every_ms == 0 {
                        self.err(format!("handler `{}`: timer period must be > 0", h.name));
                    }
                    if !loc.has_cap("clock") {
                        self.err(format!(
                            "handler `{}` is a timer, but location `{}` lacks the `clock` capability",
                            h.name, loc.name
                        ));
                    }
                    (None, false)
                }
            };
            let scope = Scope { what: format!("handler `{}`", h.name), location: Some(loc), message, sender };
            self.body(&h.body, &scope);
        }

        for prop in &p.properties {
            match &prop.kind {
                PropertyKind::Always { forall, body, result } | PropertyKind::Eventually { forall, body, result } => {
                    let scope = Scope {
                        what: format!("property `{}`", prop.name),
                        location: None,
                        message: None,
                        sender: false,
                    };
                    let types = self.body(body, &scope);
                    if types.get(*result).copied().flatten() != Some(Type::Bool) {
                        self.err(format!("property `{}` must produce a bool", prop.name));
                    }
                    let replicated: BTreeSet<&str> = body
                        .iter()
                        .filter_map(|op| match &op.kind {
                            OpKind::Read { state } => p.state_decl(state),
                            _ => None,
                        })
                        .filter_map(|s| p.location(&s.location))
                        .filter(|(_, l)| l.replicas > 1)
                        .map(|(_, l)| l.name.as_str())
                        .collect();
                    if replicated.len() > 1 {
                        self.err(format!(
                            "property `{}` reads several replicated locations ({replicated:?}); v0 quantifies over one",
                            prop.name
                        ));
                    } else if replicated.iter().next().copied() != forall.as_deref() {
                        self.err(format!(
                            "property `{}`: `forall` must name the replicated location it reads ({:?})",
                            prop.name,
                            replicated.iter().next()
                        ));
                    }
                }
                PropertyKind::Monotonic { state } => match p.state_decl(state) {
                    None => self.err(format!("property `{}`: unknown state `{state}`", prop.name)),
                    Some(s) if s.ty != Type::Int => {
                        self.err(format!("property `{}`: monotonic needs an int state", prop.name))
                    }
                    _ => {}
                },
            }
        }

        for sc in &p.scenarios {
            let net = &sc.network;
            if !(0.0..=1.0).contains(&net.drop_rate) {
                self.err(format!("scenario `{}`: drop_rate must be in [0, 1]", sc.name));
            }
            if net.latency_ms[0] > net.latency_ms[1] {
                self.err(format!("scenario `{}`: latency range is backwards", sc.name));
            }
            for inp in &sc.inputs {
                let at = format!("scenario `{}` input at {}ms", sc.name, inp.at_ms);
                let Some((_, loc)) = p.location(&inp.location) else {
                    self.err(format!("{at}: unknown location `{}`", inp.location));
                    continue;
                };
                if inp.replica.is_some_and(|r| r >= loc.replicas) {
                    self.err(format!("{at}: `{}` has only {} replicas", loc.name, loc.replicas));
                }
                let handled = p.handlers.iter().any(|h| {
                    h.location == inp.location
                        && matches!(&h.trigger, Trigger::Input { message } if *message == inp.message)
                });
                if !handled {
                    self.err(format!(
                        "{at}: injects `{}` into `{}`, but no on_input handler there takes it",
                        inp.message, inp.location
                    ));
                }
                let Some(m) = p.message(&inp.message) else {
                    self.err(format!("{at}: unknown message `{}`", inp.message));
                    continue;
                };
                for (f, t) in &m.fields {
                    match inp.fields.get(f) {
                        None => self.err(format!("{at}: missing field `{f}`")),
                        Some(v) if !const_fits(v, *t) => self.err(format!("{at}: field `{f}` must be {t}")),
                        _ => {}
                    }
                }
                for f in inp.fields.keys() {
                    if m.field(f).is_none() {
                        self.err(format!("{at}: `{}` has no field `{f}`", m.name));
                    }
                }
            }
        }
    }

    fn arg(&mut self, at: &str, types: &[Option<Type>], a: usize) -> Option<Type> {
        match types.get(a) {
            Some(t) => *t,
            None => {
                self.err(format!("{at}: uses %{a} before it is defined"));
                None
            }
        }
    }

    fn state_access(&mut self, at: &str, state: &str, sc: &Scope, verb: &str) -> Option<Type> {
        let p = self.p;
        let Some(s) = p.state_decl(state) else {
            self.err(format!("{at}: unknown state `{state}`"));
            return None;
        };
        if let Some(loc) = sc.location {
            if s.location != loc.name {
                self.err(format!(
                    "{at}: cannot {verb} `{state}` from `{}`; it lives on `{}`. \
                     Locations share no memory: send a message instead.",
                    loc.name, s.location
                ));
            }
        }
        Some(s.ty)
    }

    /// Type-check a body; returns the type of every op (None for pure effects).
    fn body(&mut self, ops: &[Op], sc: &Scope) -> Vec<Option<Type>> {
        let p = self.p;
        let mut types: Vec<Option<Type>> = Vec::with_capacity(ops.len());
        for (i, op) in ops.iter().enumerate() {
            let at = format!("{} %{i}", sc.what);
            if op.id != i {
                self.err(format!("{at}: op has id {}, expected {i}", op.id));
            }
            let effect = op.kind.is_effect();
            if effect && sc.location.is_none() {
                self.err(format!("{at}: properties are pure and cannot have effects"));
            }
            if let Some(g) = op.guard {
                if !effect {
                    self.err(format!("{at}: only effects (call / write / send) take a guard"));
                }
                if self.arg(&at, &types, g) != Some(Type::Bool) {
                    self.err(format!("{at}: guard %{g} must be bool"));
                }
            }

            let inferred: Option<Type> = match &op.kind {
                OpKind::Const { value } => match op.ty {
                    Some(t) if const_fits(value, t) => Some(t),
                    _ => {
                        self.err(format!("{at}: constant {value} does not fit type {}", ty_name(op.ty)));
                        op.ty
                    }
                },
                OpKind::Field { name } => match sc.message.and_then(|m| m.field(name)) {
                    Some((_, t)) => Some(t),
                    None => {
                        self.err(format!("{at}: no message field `{name}` here"));
                        None
                    }
                },
                OpKind::Sender => {
                    if !sc.sender {
                        self.err(format!(
                            "{at}: `sender` only exists in handlers triggered by a network message"
                        ));
                    }
                    Some(Type::Loc)
                }
                OpKind::Read { state } => self.state_access(&at, state, sc, "read"),
                OpKind::Bin { func, args } => {
                    let a = self.arg(&at, &types, args[0]);
                    let b = self.arg(&at, &types, args[1]);
                    use BinFn::*;
                    let (want, out) = match func {
                        Add | Sub | Mul => (Some(Type::Int), Type::Int),
                        Lt | Le | Gt | Ge => (Some(Type::Int), Type::Bool),
                        And | Or => (Some(Type::Bool), Type::Bool),
                        Eq | Ne => (None, Type::Bool),
                    };
                    let ok = match want {
                        Some(w) => a == Some(w) && b == Some(w),
                        None => a.is_some() && a == b,
                    };
                    if !ok {
                        self.err(format!("{at}: `{}` cannot take {} and {}", func.name(), ty_name(a), ty_name(b)));
                    }
                    Some(out)
                }
                OpKind::Not { args } => {
                    if self.arg(&at, &types, args[0]) != Some(Type::Bool) {
                        self.err(format!("{at}: `not` needs a bool"));
                    }
                    Some(Type::Bool)
                }
                OpKind::Select { args } => {
                    let c = self.arg(&at, &types, args[0]);
                    let a = self.arg(&at, &types, args[1]);
                    let b = self.arg(&at, &types, args[2]);
                    if c != Some(Type::Bool) || a.is_none() || a != b {
                        self.err(format!(
                            "{at}: `select` needs (bool, T, T), got ({}, {}, {})",
                            ty_name(c),
                            ty_name(a),
                            ty_name(b)
                        ));
                    }
                    a
                }
                OpKind::Call { ext, args } => match p.extern_decl(ext) {
                    None => {
                        self.err(format!("{at}: unknown extern `{ext}`"));
                        op.ty
                    }
                    Some(e) => {
                        if let Some(loc) = sc.location {
                            if e.location != loc.name {
                                self.err(format!(
                                    "{at}: extern `{ext}` runs on `{}`, not `{}`",
                                    e.location, loc.name
                                ));
                            }
                        }
                        if args.len() != e.params.len() {
                            self.err(format!("{at}: `{ext}` takes {} args, got {}", e.params.len(), args.len()));
                        }
                        for (a, (pname, pt)) in args.iter().zip(&e.params) {
                            let t = self.arg(&at, &types, *a);
                            if t != Some(*pt) {
                                self.err(format!("{at}: `{ext}` param `{pname}` is {pt}, got {}", ty_name(t)));
                            }
                        }
                        Some(e.returns)
                    }
                },
                OpKind::Write { state, args } => {
                    let st = self.state_access(&at, state, sc, "write");
                    let v = self.arg(&at, &types, args[0]);
                    if st.is_some() && v != st {
                        self.err(format!("{at}: writing {} into {} state `{state}`", ty_name(v), ty_name(st)));
                    }
                    None
                }
                OpKind::Send { to, message, fields } => {
                    match to {
                        Target::Location(l) => {
                            if p.location(l).is_none() {
                                self.err(format!("{at}: unknown location `{l}`"));
                            }
                        }
                        Target::Value(v) => {
                            if self.arg(&at, &types, *v) != Some(Type::Loc) {
                                self.err(format!("{at}: send target %{v} must be a loc"));
                            }
                        }
                    }
                    match p.message(message) {
                        None => self.err(format!("{at}: unknown message `{message}`")),
                        Some(m) => {
                            for (f, t) in &m.fields {
                                match fields.get(f) {
                                    None => self.err(format!("{at}: `{message}` is missing field `{f}`")),
                                    Some(v) => {
                                        let vt = self.arg(&at, &types, *v);
                                        if vt != Some(*t) {
                                            self.err(format!("{at}: field `{f}` is {t}, got {}", ty_name(vt)));
                                        }
                                    }
                                }
                            }
                            for f in fields.keys() {
                                if m.field(f).is_none() {
                                    self.err(format!("{at}: `{message}` has no field `{f}`"));
                                }
                            }
                        }
                    }
                    None
                }
            };

            let has_value = !matches!(op.kind, OpKind::Write { .. } | OpKind::Send { .. });
            if has_value && inferred.is_some() && op.ty != inferred {
                self.err(format!(
                    "{at}: declared type {} but the op produces {}",
                    ty_name(op.ty),
                    ty_name(inferred)
                ));
            }
            types.push(inferred);
        }
        types
    }
}
