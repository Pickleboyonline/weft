//! The simulator: an interpreter for handler bodies plus a discrete-event world.
//!
//! Execution model (see docs/ir-v0.md):
//! - A handler runs to completion on one replica: an atomic transaction. Reads see
//!   writes made earlier in the same handler.
//! - Sends are buffered and leave when the handler commits.
//! - Extern latency keeps the replica busy: its sends leave late, and other events
//!   for that replica wait until it is free.
//! - The network drops messages (until `heal_at_ms`) and delays each one by a
//!   uniform random latency, so messages reorder.

use crate::rng::Rng;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use weft_ir::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    /// Result slot of an effect that produces nothing (write, send).
    Unit,
    Int(i64),
    Bool(bool),
    /// Global replica index.
    Loc(usize),
}

impl Value {
    fn int(self) -> i64 {
        match self {
            Value::Int(i) => i,
            v => panic!("expected int, got {v:?} (the validator should have caught this)"),
        }
    }
    fn bool(self) -> bool {
        match self {
            Value::Bool(b) => b,
            v => panic!("expected bool, got {v:?} (the validator should have caught this)"),
        }
    }
    fn from_json(v: &serde_json::Value, ty: Type) -> Value {
        match ty {
            Type::Int => Value::Int(v.as_i64().expect("validated int")),
            Type::Bool => Value::Bool(v.as_bool().expect("validated bool")),
            Type::Loc => panic!("loc has no literals"),
        }
    }
    /// What a guarded-off extern call yields.
    fn zero(ty: Type) -> Value {
        match ty {
            Type::Int => Value::Int(0),
            Type::Bool => Value::Bool(false),
            Type::Loc => Value::Loc(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub property: String,
    pub at_ms: u64,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub events: u64,
    pub sent: u64,
    pub dropped: u64,
    pub extern_calls: u64,
    pub extern_failures: u64,
}

pub struct Outcome {
    pub scenario: String,
    pub seed: u64,
    pub end_ms: u64,
    pub violation: Option<Violation>,
    /// Empty unless the run was recorded.
    pub trace: Vec<String>,
    /// `(replica, [(state, value)])`
    pub final_state: Vec<(String, Vec<(String, String)>)>,
    pub stats: Stats,
}

/// Run one scenario with one seed.
pub fn run(p: &Program, sc: &Scenario, seed: u64, record: bool) -> Outcome {
    let mut sim = Sim::new(p, sc, seed, record);
    let violation = sim.run();
    let final_state = sim
        .replicas
        .iter()
        .map(|r| {
            let st = r
                .state
                .iter()
                .map(|(k, v)| (k.split_once('.').map_or(k.as_str(), |x| x.1).to_string(), sim.show(*v)))
                .collect();
            (r.name.clone(), st)
        })
        .collect();
    Outcome {
        scenario: sc.name.clone(),
        seed,
        end_ms: sim.now,
        violation,
        trace: std::mem::take(&mut sim.trace),
        final_state,
        stats: sim.stats.clone(),
    }
}

struct Event {
    time: u64,
    seq: u64,
    kind: EventKind,
}

// Min-heap on (time, seq): BinaryHeap is a max-heap, so compare reversed.
impl Ord for Event {
    fn cmp(&self, o: &Self) -> Ordering {
        (o.time, o.seq).cmp(&(self.time, self.seq))
    }
}
impl PartialOrd for Event {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Event {
    fn eq(&self, o: &Self) -> bool {
        self.time == o.time && self.seq == o.seq
    }
}
impl Eq for Event {}

enum EventKind {
    Deliver { to: usize, from: Option<usize>, message: String, fields: Vec<Value>, input: bool },
    Timer { replica: usize, handler: usize },
}

impl EventKind {
    fn replica(&self) -> usize {
        match self {
            EventKind::Deliver { to, .. } => *to,
            EventKind::Timer { replica, .. } => *replica,
        }
    }
}

struct Replica {
    loc: usize,
    name: String,
    state: BTreeMap<String, Value>,
    busy_until: u64,
}

struct Outgoing {
    targets: Vec<usize>,
    message: String,
    fields: Vec<Value>,
}

struct Sim<'a> {
    p: &'a Program,
    sc: &'a Scenario,
    rng: Rng,
    now: u64,
    seq: u64,
    queue: BinaryHeap<Event>,
    replicas: Vec<Replica>,
    /// location index -> global replica indices
    by_loc: Vec<Vec<usize>>,
    mono_prev: HashMap<(usize, String), i64>,
    record: bool,
    trace: Vec<String>,
    stats: Stats,
}

impl<'a> Sim<'a> {
    fn new(p: &'a Program, sc: &'a Scenario, seed: u64, record: bool) -> Self {
        let mut replicas = Vec::new();
        let mut by_loc = Vec::new();
        for (li, l) in p.locations.iter().enumerate() {
            let mut ids = Vec::new();
            for r in 0..l.replicas {
                let name = if l.replicas == 1 { l.name.clone() } else { format!("{}#{r}", l.name) };
                let state = p
                    .state
                    .iter()
                    .filter(|s| s.location == l.name)
                    .map(|s| (s.name.clone(), Value::from_json(&s.init, s.ty)))
                    .collect();
                ids.push(replicas.len());
                replicas.push(Replica { loc: li, name, state, busy_until: 0 });
            }
            by_loc.push(ids);
        }
        let mut sim = Sim {
            p,
            sc,
            rng: Rng::new(seed),
            now: 0,
            seq: 0,
            queue: BinaryHeap::new(),
            replicas,
            by_loc,
            mono_prev: HashMap::new(),
            record,
            trace: Vec::new(),
            stats: Stats::default(),
        };

        for inp in &sc.inputs {
            let (li, _) = p.location(&inp.location).expect("validated");
            let m = p.message(&inp.message).expect("validated");
            let fields: Vec<Value> = m.fields.iter().map(|(n, t)| Value::from_json(&inp.fields[n], *t)).collect();
            let targets = match inp.replica {
                Some(r) => vec![sim.by_loc[li][r]],
                None => sim.by_loc[li].clone(),
            };
            for to in targets {
                let kind = EventKind::Deliver { to, from: None, message: inp.message.clone(), fields: fields.clone(), input: true };
                sim.push(inp.at_ms, kind);
            }
        }

        // Timers start at a random phase so replicas don't tick in lockstep.
        for (hi, h) in p.handlers.iter().enumerate() {
            if let Trigger::Timer { every_ms } = &h.trigger {
                let (li, _) = p.location(&h.location).expect("validated");
                for r in sim.by_loc[li].clone() {
                    let first = sim.rng.range(1, *every_ms);
                    sim.push(first, EventKind::Timer { replica: r, handler: hi });
                }
            }
        }
        sim
    }

    fn push(&mut self, time: u64, kind: EventKind) {
        self.seq += 1;
        self.queue.push(Event { time, seq: self.seq, kind });
    }

    fn log(&mut self, who: usize, what: impl FnOnce(&Self) -> String) {
        if !self.record {
            return;
        }
        let line = format!("{:>9.3}s  {:<10} {}", self.now as f64 / 1000.0, self.replicas[who].name, what(self));
        self.trace.push(line);
    }

    fn show(&self, v: Value) -> String {
        match v {
            Value::Unit => "()".into(),
            Value::Int(i) => i.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Loc(r) => self.replicas[r].name.clone(),
        }
    }

    fn show_msg(&self, name: &str, fields: &[Value]) -> String {
        let m = self.p.message(name).expect("validated");
        let f: Vec<String> = m.fields.iter().zip(fields).map(|((n, _), v)| format!("{n}={}", self.show(*v))).collect();
        format!("{name}{{{}}}", f.join(", "))
    }

    fn run(&mut self) -> Option<Violation> {
        if let Some(v) = self.check_always() {
            return Some(v);
        }
        while let Some(ev) = self.queue.pop() {
            if ev.time > self.sc.duration_ms {
                break;
            }
            self.now = ev.time;
            let r = ev.kind.replica();
            if self.replicas[r].busy_until > self.now {
                let t = self.replicas[r].busy_until;
                self.push(t, ev.kind);
                continue;
            }
            self.stats.events += 1;
            self.dispatch(ev.kind);
            if let Some(v) = self.check_always() {
                return Some(v);
            }
        }
        self.now = self.sc.duration_ms;
        self.check_eventually()
    }

    fn dispatch(&mut self, kind: EventKind) {
        let p = self.p;
        match kind {
            EventKind::Timer { replica, handler } => {
                let h = &p.handlers[handler];
                let Trigger::Timer { every_ms } = &h.trigger else { unreachable!() };
                self.log(replica, |_| format!("tick {}", h.name));
                self.exec(h, replica, None, None);
                self.push(self.now + every_ms, EventKind::Timer { replica, handler });
            }
            EventKind::Deliver { to, from, message, fields, input } => {
                let loc = &p.locations[self.replicas[to].loc];
                let m = p.message(&message).expect("validated");
                let h = p.handlers.iter().find(|h| {
                    h.location == loc.name
                        && match &h.trigger {
                            Trigger::Message { message: mm } => !input && *mm == message,
                            Trigger::Input { message: mm } => input && *mm == message,
                            Trigger::Timer { .. } => false,
                        }
                });
                self.log(to, |s| {
                    let src = match from {
                        Some(f) => format!("from {}", s.replicas[f].name),
                        None => "(input)".to_string(),
                    };
                    let tail = if h.is_none() { "  [no handler; ignored]" } else { "" };
                    format!("<- {} {src}{tail}", s.show_msg(&message, &fields))
                });
                if let Some(h) = h {
                    self.exec(h, to, Some((m, &fields)), from);
                }
            }
        }
    }

    fn exec(&mut self, h: &Handler, replica: usize, msg: Option<(&Message, &[Value])>, sender: Option<usize>) {
        let p = self.p;
        let mut vals: Vec<Value> = Vec::with_capacity(h.body.len());
        let mut outbox: Vec<Outgoing> = Vec::new();
        let mut latency = 0u64;
        for op in &h.body {
            let active = op.guard.map_or(true, |g| vals[g].bool());
            let v = match &op.kind {
                OpKind::Call { ext, args } => {
                    if active {
                        let a: Vec<Value> = args.iter().map(|&i| vals[i]).collect();
                        let (v, lat) = self.call_extern(replica, ext, &a);
                        latency += lat;
                        v
                    } else {
                        Value::zero(op.ty.expect("validated"))
                    }
                }
                OpKind::Write { state, args } => {
                    if active {
                        let v = vals[args[0]];
                        self.log(replica, |s| format!("write {state} = {}", s.show(v)));
                        self.replicas[replica].state.insert(state.clone(), v);
                    }
                    Value::Unit
                }
                OpKind::Send { to, message, fields } => {
                    if active {
                        let targets = match to {
                            Target::Location(l) => self.by_loc[p.location(l).expect("validated").0].clone(),
                            Target::Value(i) => match vals[*i] {
                                Value::Loc(r) => vec![r],
                                v => panic!("send target {v:?} is not a loc"),
                            },
                        };
                        let m = p.message(message).expect("validated");
                        let fields = m.fields.iter().map(|(n, _)| vals[fields[n]]).collect();
                        outbox.push(Outgoing { targets, message: message.clone(), fields });
                    }
                    Value::Unit
                }
                _ => self.eval_pure(op, &vals, replica, msg, sender),
            };
            vals.push(v);
        }
        // Commit. Extern latency holds the replica and delays what it sends.
        let depart = self.now + latency;
        self.replicas[replica].busy_until = depart;
        for out in outbox {
            self.transmit(replica, depart, out);
        }
    }

    fn eval_pure(&self, op: &Op, vals: &[Value], replica: usize, msg: Option<(&Message, &[Value])>, sender: Option<usize>) -> Value {
        match &op.kind {
            OpKind::Const { value } => Value::from_json(value, op.ty.expect("validated")),
            OpKind::Field { name } => {
                let (m, f) = msg.expect("validated: field only in message handlers");
                f[m.field(name).expect("validated").0]
            }
            OpKind::Sender => Value::Loc(sender.expect("validated: sender only in message handlers")),
            OpKind::Read { state } => self.read(state, replica),
            OpKind::Bin { func, args } => bin(*func, vals[args[0]], vals[args[1]]),
            OpKind::Not { args } => Value::Bool(!vals[args[0]].bool()),
            OpKind::Select { args } => {
                if vals[args[0]].bool() {
                    vals[args[1]]
                } else {
                    vals[args[2]]
                }
            }
            OpKind::Call { .. } | OpKind::Write { .. } | OpKind::Send { .. } => unreachable!("effects run in exec"),
        }
    }

    /// Read `state` as seen from `replica`: its own copy if the state lives on its
    /// location, otherwise the (validated) singleton location's copy.
    fn read(&self, state: &str, replica: usize) -> Value {
        let decl = self.p.state_decl(state).expect("validated");
        let li = self.p.location(&decl.location).expect("validated").0;
        let r = if self.replicas[replica].loc == li { replica } else { self.by_loc[li][0] };
        self.replicas[r].state[state]
    }

    fn call_extern(&mut self, replica: usize, name: &str, args: &[Value]) -> (Value, u64) {
        let e = self.p.extern_decl(name).expect("validated");
        self.stats.extern_calls += 1;
        let (v, lat, failed) = match e.sim.as_ref().expect("validated: externs have sim models") {
            SimModel::Fallible { fail_rate, latency_ms, ok, err } => {
                let lat = self.rng.range(latency_ms[0], latency_ms[1]);
                let failed = self.rng.chance(*fail_rate);
                (Value::from_json(if failed { err } else { ok }, e.returns), lat, failed)
            }
            SimModel::Constant { value, latency_ms } => {
                (Value::from_json(value, e.returns), self.rng.range(latency_ms[0], latency_ms[1]), false)
            }
        };
        if failed {
            self.stats.extern_failures += 1;
        }
        self.log(replica, |s| {
            let a: Vec<String> = e.params.iter().zip(args).map(|((n, _), v)| format!("{n}={}", s.show(*v))).collect();
            let note = if failed { "  (sim model: failure)" } else { "" };
            format!("call {name}({}) = {} [{lat}ms]{note}", a.join(", "), s.show(v))
        });
        (v, lat)
    }

    fn transmit(&mut self, from: usize, depart: u64, out: Outgoing) {
        let net = &self.sc.network;
        let faulty = self.sc.heal_at_ms.map_or(true, |h| depart < h);
        for to in out.targets {
            self.stats.sent += 1;
            if faulty && self.rng.chance(net.drop_rate) {
                self.stats.dropped += 1;
                self.log(from, |s| format!("-> {} {}  DROPPED", s.replicas[to].name, s.show_msg(&out.message, &out.fields)));
                continue;
            }
            let arrive = depart + self.rng.range(net.latency_ms[0], net.latency_ms[1]);
            self.log(from, |s| {
                format!(
                    "-> {} {}  (arrives {:.3}s)",
                    s.replicas[to].name,
                    s.show_msg(&out.message, &out.fields),
                    arrive as f64 / 1000.0
                )
            });
            let kind = EventKind::Deliver { to, from: Some(from), message: out.message.clone(), fields: out.fields.clone(), input: false };
            self.push(arrive, kind);
        }
    }

    // ------------------------------------------------------------ properties

    fn check_always(&mut self) -> Option<Violation> {
        if let Some(v) = self.check_monotonic() {
            return Some(v);
        }
        let p = self.p;
        for prop in &p.properties {
            if let PropertyKind::Always { forall, body, result } = &prop.kind {
                if let Some(detail) = self.eval_property(forall, body, *result) {
                    return Some(Violation { property: prop.name.clone(), at_ms: self.now, detail });
                }
            }
        }
        None
    }

    fn check_eventually(&mut self) -> Option<Violation> {
        let p = self.p;
        for prop in &p.properties {
            if let PropertyKind::Eventually { forall, body, result } = &prop.kind {
                if let Some(detail) = self.eval_property(forall, body, *result) {
                    let detail = format!("still false at end of run: {detail}");
                    return Some(Violation { property: prop.name.clone(), at_ms: self.now, detail });
                }
            }
        }
        None
    }

    fn check_monotonic(&mut self) -> Option<Violation> {
        let p = self.p;
        for prop in &p.properties {
            let PropertyKind::Monotonic { state } = &prop.kind else { continue };
            let decl = p.state_decl(state).expect("validated");
            let li = p.location(&decl.location).expect("validated").0;
            for &r in &self.by_loc[li] {
                let cur = self.replicas[r].state[state.as_str()].int();
                if let Some(prev) = self.mono_prev.insert((r, state.clone()), cur) {
                    if cur < prev {
                        let detail = format!("{}: {state} went {prev} -> {cur}", self.replicas[r].name);
                        return Some(Violation { property: prop.name.clone(), at_ms: self.now, detail });
                    }
                }
            }
        }
        None
    }

    /// Evaluate a property on each replica it quantifies over. Returns a
    /// description of the first replica where it is false.
    fn eval_property(&self, forall: &Option<String>, body: &[Op], result: usize) -> Option<String> {
        let bound: Vec<usize> = match forall {
            Some(l) => self.by_loc[self.p.location(l).expect("validated").0].clone(),
            None => vec![0],
        };
        for r in bound {
            let mut vals = Vec::with_capacity(body.len());
            for op in body {
                let v = self.eval_pure(op, &vals, r, None, None);
                vals.push(v);
            }
            if !vals[result].bool() {
                let reads: Vec<String> = body
                    .iter()
                    .filter_map(|op| match &op.kind {
                        OpKind::Read { state } => Some(format!("{state}={}", self.show(vals[op.id]))),
                        _ => None,
                    })
                    .collect();
                let who = if forall.is_some() { format!("{}: ", self.replicas[r].name) } else { String::new() };
                return Some(format!("{who}{}", reads.join(", ")));
            }
        }
        None
    }
}

fn bin(f: BinFn, a: Value, b: Value) -> Value {
    use BinFn::*;
    match f {
        Add => Value::Int(a.int().wrapping_add(b.int())),
        Sub => Value::Int(a.int().wrapping_sub(b.int())),
        Mul => Value::Int(a.int().wrapping_mul(b.int())),
        Lt => Value::Bool(a.int() < b.int()),
        Le => Value::Bool(a.int() <= b.int()),
        Gt => Value::Bool(a.int() > b.int()),
        Ge => Value::Bool(a.int() >= b.int()),
        Eq => Value::Bool(a == b),
        Ne => Value::Bool(a != b),
        And => Value::Bool(a.bool() && b.bool()),
        Or => Value::Bool(a.bool() || b.bool()),
    }
}
