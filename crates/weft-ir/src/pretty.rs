//! Human-readable form of the IR, in the spirit of `jax.make_jaxpr`.

use crate::*;
use std::fmt::Write;

pub fn pretty(p: &Program) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "program {}  (weft_ir v{})", p.name, p.weft_ir);
    for l in &p.locations {
        let _ = writeln!(s, "location {} x{}  caps[{}]", l.name, l.replicas, l.caps.join(", "));
    }
    for m in &p.messages {
        let fields: Vec<String> = m.fields.iter().map(|(n, t)| format!("{n}: {t}")).collect();
        let _ = writeln!(s, "message {} {{ {} }}", m.name, fields.join(", "));
    }
    for st in &p.state {
        let _ = writeln!(s, "state {}: {} = {}", st.name, st.ty, st.init);
    }
    for e in &p.externs {
        let params: Vec<String> = e.params.iter().map(|(n, t)| format!("{n}: {t}")).collect();
        let sim = match &e.sim {
            None => "MISSING".to_string(),
            Some(SimModel::Fallible { fail_rate, latency_ms, ok, err }) => format!(
                "fallible(fail_rate={fail_rate}, latency={}..{}ms, ok={ok}, err={err})",
                latency_ms[0], latency_ms[1]
            ),
            Some(SimModel::Constant { value, latency_ms }) => {
                format!("constant({value}, latency={}..{}ms)", latency_ms[0], latency_ms[1])
            }
        };
        let _ = writeln!(
            s,
            "extern {}.{}({}) -> {}  effects[{}]  sim={}",
            e.location,
            e.name,
            params.join(", "),
            e.returns,
            e.effects.join(", "),
            sim
        );
    }
    for h in &p.handlers {
        let trig = match &h.trigger {
            Trigger::Message { message } => format!("message {message}"),
            Trigger::Input { message } => format!("input {message}"),
            Trigger::Timer { every_ms } => format!("every {every_ms}ms"),
        };
        let _ = writeln!(s, "\nhandler {} @{} on {}:", h.name, h.location, trig);
        for op in &h.body {
            let _ = writeln!(s, "  {}", op_line(op));
        }
    }
    for prop in &p.properties {
        match &prop.kind {
            PropertyKind::Always { forall, body, result } | PropertyKind::Eventually { forall, body, result } => {
                let kind = if matches!(prop.kind, PropertyKind::Always { .. }) { "always" } else { "eventually" };
                let q = forall.as_ref().map(|l| format!(" forall {l}")).unwrap_or_default();
                let _ = writeln!(s, "\n{kind} {}{q}:", prop.name);
                for op in body {
                    let _ = writeln!(s, "  {}", op_line(op));
                }
                let _ = writeln!(s, "  => %{result}");
            }
            PropertyKind::Monotonic { state } => {
                let _ = writeln!(s, "\nmonotonic {}: {state}", prop.name);
            }
        }
    }
    for sc in &p.scenarios {
        let heal = sc.heal_at_ms.map(|h| format!("{h}ms")).unwrap_or_else(|| "never".into());
        let _ = writeln!(
            s,
            "\nscenario {}: {}ms, {} inputs, drop={} latency={}..{}ms heal_at={}",
            sc.name,
            sc.duration_ms,
            sc.inputs.len(),
            sc.network.drop_rate,
            sc.network.latency_ms[0],
            sc.network.latency_ms[1],
            heal
        );
    }
    s
}

fn op_line(op: &Op) -> String {
    let refs = |ids: &[usize]| ids.iter().map(|i| format!("%{i}")).collect::<Vec<_>>().join(" ");
    let rhs = match &op.kind {
        OpKind::Const { value } => format!("const {value}"),
        OpKind::Field { name } => format!("field {name}"),
        OpKind::Sender => "sender".to_string(),
        OpKind::Read { state } => format!("read {state}"),
        OpKind::Bin { func, args } => format!("{} {}", func.name(), refs(args)),
        OpKind::Not { args } => format!("not {}", refs(args)),
        OpKind::Select { args } => format!("select {}", refs(args)),
        OpKind::Call { ext, args } => format!(
            "call {ext}({})",
            args.iter().map(|i| format!("%{i}")).collect::<Vec<_>>().join(", ")
        ),
        OpKind::Write { state, args } => format!("write {state} {}", refs(args)),
        OpKind::Send { to, message, fields } => {
            let to = match to {
                Target::Location(l) => l.clone(),
                Target::Value(v) => format!("%{v}"),
            };
            let f: Vec<String> = fields.iter().map(|(k, v)| format!("{k}: %{v}")).collect();
            format!("send {to} {message}{{{}}}", f.join(", "))
        }
    };
    let lhs = match op.ty {
        Some(t) => format!("%{}: {} = ", op.id, t),
        None => String::new(),
    };
    let guard = op.guard.map(|g| format!("    if %{g}")).unwrap_or_default();
    format!("{lhs}{rhs}{guard}")
}
