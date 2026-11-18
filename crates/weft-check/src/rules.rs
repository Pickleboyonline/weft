//! Whole-program effects (each unit's effects plus those of what it calls),
//! and the rules about where effects are allowed (reference: Effects, "Where
//! effects are allowed").

use std::collections::{BTreeMap, HashMap};

use weft_syntax::ast::Span;
use weft_syntax::Diagnostic;

use crate::effects::Effect;
use crate::walk::{Unit, UnitKind};

/// An effect, where it shows up in the unit, and the call it came through.
#[derive(Clone, Debug)]
pub struct Origin {
    pub span: Span,
    pub via: Option<String>,
}

pub type EffectSet = BTreeMap<Effect, Origin>;

/// Each unit's total effects: its own, plus everything its calls do.
pub fn propagate(units: &[Unit]) -> Vec<EffectSet> {
    let by_name: HashMap<&str, usize> = units
        .iter()
        .enumerate()
        .filter(|(_, u)| {
            matches!(u.kind, UnitKind::Function | UnitKind::Stateful | UnitKind::Query | UnitKind::Command)
                || (u.kind == UnitKind::Method && u.name.ends_with(" defaults"))
        })
        .map(|(i, u)| (u.name.as_str(), i))
        .collect();
    let mut total: Vec<EffectSet> = units
        .iter()
        .map(|u| u.direct.iter().map(|(e, s)| (e.clone(), Origin { span: *s, via: None })).collect())
        .collect();
    loop {
        let mut changed = false;
        for (i, u) in units.iter().enumerate() {
            for (callee, span) in &u.calls {
                let Some(&j) = by_name.get(callee.as_str()) else { continue };
                if i == j {
                    continue;
                }
                let add: Vec<Effect> = total[j].keys().filter(|e| !total[i].contains_key(*e)).cloned().collect();
                for e in add {
                    total[i].insert(e, Origin { span: *span, via: Some(callee.clone()) });
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    total
}

/// Reports each effect that isn't allowed where it occurs.
pub fn check(units: &[Unit], totals: &[EffectSet], out: &mut Vec<(usize, Diagnostic)>) {
    for (u, effects) in units.iter().zip(totals) {
        for (effect, origin) in effects {
            if let Some((code, msg)) = violation(u, effect) {
                let through = match &origin.via {
                    Some(c) => format!(" (through `{c}`)"),
                    None => String::new(),
                };
                let mut d = Diagnostic::error(code, format!("{msg}{through}"), origin.span);
                if let Some(c) = &origin.via {
                    d = d.with_note(format!("the effect isn't in this code itself: `{c}` (or something it calls) has it"));
                }
                out.push((u.file, d));
            }
        }
    }
}

fn is_server_table(e: &Effect) -> Option<&str> {
    match e {
        Effect::Read(t) | Effect::Write(t) if !t.contains('(') => Some(t),
        _ => None,
    }
}

fn violation(u: &Unit, e: &Effect) -> Option<(&'static str, String)> {
    let name = short(&u.name);
    match u.kind {
        UnitKind::Expect => Some(("expect-effect", format!("expect can't {}: it must be pure", e.verb()))),
        UnitKind::Constant => match e {
            Effect::Read(t) => Some((
                "constant-effect",
                format!(
                    "{name} can't read {t}: a top-level constant is computed once. Make it a function: {name} = || …"
                ),
            )),
            _ => Some(("constant-effect", format!("{name} can't {}: a top-level constant is computed once", e.verb()))),
        },
        UnitKind::Query => match e {
            Effect::Read(_) => None,
            _ => Some(("query-effect", format!("{name} can't {}: a query only reads", e.verb()))),
        },
        UnitKind::Check => match e {
            Effect::Read(_) => None,
            _ => Some(("check-effect", format!("{name} can't {}: a check only reads", e.verb()))),
        },
        UnitKind::Command => match e {
            Effect::Await | Effect::Wait | Effect::Outside(_) => Some((
                "command-effect",
                format!(
                    "{name} can't {}: a command runs atomically and may be retried. Fetch first and pass the result in",
                    e.verb()
                ),
            )),
            Effect::Navigate | Effect::Assign(_) | Effect::Status | Effect::Dom | Effect::Browser => {
                Some(("command-effect", format!("{name} can't {}: a command runs on the server", e.verb())))
            }
            _ => None,
        },
        UnitKind::View => match e {
            Effect::CallQuery(_) | Effect::Status => None,
            Effect::Read(t) | Effect::Write(t) if is_server_table(e).is_some() => Some((
                "server-only",
                format!("{name} can't read `{t}`: it's a server table. Read it in a Query and call that"),
            )),
            Effect::Read(_) => None,
            Effect::Send(_) | Effect::Write(_) | Effect::Assign(_) | Effect::Navigate => {
                Some(("view-effect", format!("view can't {}: only handlers can change data", e.verb())))
            }
            _ => Some(("view-effect", format!("view can't {}: a view only reads", e.verb()))),
        },
        UnitKind::Handler | UnitKind::Task | UnitKind::Stateful => match is_server_table(e) {
            Some(t) => Some((
                "server-only",
                format!("{name} can't use `{t}`: it's a server table. Use a Query or a Command"),
            )),
            None => None,
        },
        UnitKind::Function | UnitKind::Method | UnitKind::HandlerImpl => None,
    }
}

fn short(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}
