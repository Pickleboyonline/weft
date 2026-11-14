//! A tiny hand-written program: `a` counts and broadcasts; each `b` keeps the max it saw.

use weft_ir::Program;

const PING: &str = r#"{
  "weft_ir": 0, "name": "ping",
  "locations": [
    {"name": "a", "caps": ["storage", "clock"]},
    {"name": "b", "replicas": 3, "caps": ["storage"]}
  ],
  "messages": [{"name": "Ping", "fields": [["n", "int"]]}],
  "state": [
    {"name": "a.n", "location": "a", "type": "int", "init": 0},
    {"name": "b.seen", "location": "b", "type": "int", "init": 0}
  ],
  "handlers": [
    {"name": "tick", "location": "a", "trigger": {"kind": "timer", "every_ms": 100}, "body": [
      {"id": 0, "op": "read", "state": "a.n", "type": "int"},
      {"id": 1, "op": "const", "value": 1, "type": "int"},
      {"id": 2, "op": "bin", "fn": "add", "args": [0, 1], "type": "int"},
      {"id": 3, "op": "write", "state": "a.n", "args": [2]},
      {"id": 4, "op": "send", "to": {"location": "b"}, "message": "Ping", "fields": {"n": 2}}
    ]},
    {"name": "on_ping", "location": "b", "trigger": {"kind": "message", "message": "Ping"}, "body": [
      {"id": 0, "op": "field", "name": "n", "type": "int"},
      {"id": 1, "op": "read", "state": "b.seen", "type": "int"},
      {"id": 2, "op": "bin", "fn": "gt", "args": [0, 1], "type": "bool"},
      {"id": 3, "op": "write", "state": "b.seen", "args": [0], "guard": 2}
    ]}
  ],
  "properties": [
    {"name": "seen_monotonic", "kind": "monotonic", "state": "b.seen"},
    {"name": "b_not_ahead", "kind": "always", "forall": "b", "result": 2, "body": [
      {"id": 0, "op": "read", "state": "b.seen", "type": "int"},
      {"id": 1, "op": "read", "state": "a.n", "type": "int"},
      {"id": 2, "op": "bin", "fn": "le", "args": [0, 1], "type": "bool"}
    ]}
  ],
  "scenarios": [
    {"name": "lossy", "duration_ms": 10000, "network": {"drop_rate": 0.2, "latency_ms": [1, 300]}}
  ]
}"#;

fn program() -> Program {
    let p: Program = serde_json::from_str(PING).unwrap();
    weft_ir::validate(&p).unwrap();
    p
}

#[test]
fn same_seed_same_run() {
    let p = program();
    let a = weft_sim::run(&p, &p.scenarios[0], 7, true);
    let b = weft_sim::run(&p, &p.scenarios[0], 7, true);
    let c = weft_sim::run(&p, &p.scenarios[0], 8, true);
    assert!(!a.trace.is_empty());
    assert_eq!(a.trace, b.trace);
    assert_ne!(a.trace, c.trace);
}

#[test]
fn correct_program_passes_many_seeds() {
    let p = program();
    for seed in 0..50 {
        let out = weft_sim::run(&p, &p.scenarios[0], seed, false);
        assert!(out.violation.is_none(), "seed {seed}: {:?}", out.violation);
    }
}

#[test]
fn dropping_the_guard_is_caught() {
    // Without `guard`, a reordered older Ping overwrites a newer one.
    let mut p = program();
    p.handlers[1].body[3].guard = None;
    let found = (0..50).any(|seed| {
        let out = weft_sim::run(&p, &p.scenarios[0], seed, false);
        out.violation.is_some_and(|v| v.property == "seen_monotonic")
    });
    assert!(found, "the simulator should find the reordering bug");
}

#[test]
fn extern_without_sim_model_is_rejected() {
    let mut p = program();
    p.externs.push(weft_ir::Extern {
        name: "flash".into(),
        location: "a".into(),
        params: vec![],
        returns: weft_ir::Type::Bool,
        effects: vec![],
        sim: None,
    });
    let errs = weft_ir::validate(&p).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("simulation model")), "{errs:?}");
}

#[test]
fn remote_read_is_rejected() {
    let mut p = program();
    // on_ping (on `b`) tries to read `a.n` directly.
    p.handlers[1].body[1].kind = weft_ir::OpKind::Read { state: "a.n".into() };
    let errs = weft_ir::validate(&p).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("share no memory")), "{errs:?}");
}
