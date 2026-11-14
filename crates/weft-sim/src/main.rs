use std::collections::BTreeMap;
use std::process::ExitCode;
use std::time::Instant;
use weft_ir::{Program, Scenario};
use weft_sim::Outcome;

const USAGE: &str = "\
weft-sim: deterministic simulator for Weft IR

usage:
  weft-sim check <program.weft.json>      validate and pretty-print the IR
  weft-sim run   <program.weft.json> [--seed N] [--scenario NAME] [--trace]
  weft-sim sweep <program.weft.json> [--seeds N] [--start N] [--scenario NAME]

exit codes: 0 ok, 1 a property was violated, 2 bad input";

struct Opts {
    seed: u64,
    seeds: u64,
    start: u64,
    scenario: Option<String>,
    trace: bool,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match real_main(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

fn num(s: &str) -> Result<u64, String> {
    s.replace('_', "").parse().map_err(|_| format!("not a number: {s}"))
}

fn real_main(args: &[String]) -> Result<ExitCode, String> {
    let [cmd, path, rest @ ..] = args else { return Err(USAGE.into()) };
    let mut o = Opts { seed: 1, seeds: 100, start: 1, scenario: None, trace: false };
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        let mut val = || it.next().map(String::as_str).ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--seed" => o.seed = num(val()?)?,
            "--seeds" => o.seeds = num(val()?)?,
            "--start" => o.start = num(val()?)?,
            "--scenario" => o.scenario = Some(val()?.to_string()),
            "--trace" => o.trace = true,
            other => return Err(format!("unknown flag {other}\n\n{USAGE}")),
        }
    }

    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let program: Program = serde_json::from_str(&text).map_err(|e| format!("{path} is not valid Weft IR: {e}"))?;
    if let Err(errs) = weft_ir::validate(&program) {
        eprintln!("{path}: {} error(s)", errs.len());
        for e in errs {
            eprintln!("  - {e}");
        }
        return Ok(ExitCode::from(2));
    }

    match cmd.as_str() {
        "check" => {
            print!("{}", weft_ir::pretty(&program));
            println!("\nok: {path} is valid Weft IR v{}", program.weft_ir);
            Ok(ExitCode::SUCCESS)
        }
        "run" => {
            let sc = pick(&program, &o.scenario)?;
            let out = weft_sim::run(&program, sc, o.seed, true);
            report_run(&program, &out, o.trace);
            Ok(if out.violation.is_some() { ExitCode::from(1) } else { ExitCode::SUCCESS })
        }
        "sweep" => {
            let sc = pick(&program, &o.scenario)?;
            let t0 = Instant::now();
            let mut failures = Vec::new();
            let mut events = 0;
            for seed in o.start..o.start + o.seeds {
                let out = weft_sim::run(&program, sc, seed, false);
                events += out.stats.events;
                if let Some(v) = out.violation {
                    failures.push((seed, v));
                }
            }
            let secs = t0.elapsed().as_secs_f64();
            println!(
                "{}/{}: swept seeds {}..{} ({} sim events in {secs:.2}s)",
                program.name,
                sc.name,
                o.start,
                o.start + o.seeds - 1,
                events
            );
            println!("  {} passed, {} failed", o.seeds - failures.len() as u64, failures.len());
            let Some((seed, v)) = failures.first() else { return Ok(ExitCode::SUCCESS) };
            let mut by_prop: BTreeMap<&str, usize> = BTreeMap::new();
            for (_, v) in &failures {
                *by_prop.entry(v.property.as_str()).or_default() += 1;
            }
            for (prop, n) in by_prop {
                println!("  `{prop}` violated in {n} seeds");
            }
            println!(
                "  first: seed {seed}, `{}` at {:.3}s: {}",
                v.property,
                v.at_ms as f64 / 1000.0,
                v.detail
            );
            println!("  replay: weft-sim run {path} --seed {seed} --trace");
            Ok(ExitCode::from(1))
        }
        _ => Err(USAGE.into()),
    }
}

fn pick<'a>(p: &'a Program, name: &Option<String>) -> Result<&'a Scenario, String> {
    match name {
        Some(n) => p.scenarios.iter().find(|s| &s.name == n).ok_or(format!("no scenario named `{n}`")),
        None => p.scenarios.first().ok_or("program has no scenarios".to_string()),
    }
}

fn report_run(p: &Program, out: &Outcome, full_trace: bool) {
    if full_trace {
        for l in &out.trace {
            println!("{l}");
        }
        println!();
    } else if out.violation.is_some() {
        let tail = out.trace.len().saturating_sub(25);
        println!("... last {} of {} trace lines (--trace for all):", out.trace.len() - tail, out.trace.len());
        for l in &out.trace[tail..] {
            println!("{l}");
        }
        println!();
    }
    let s = &out.stats;
    let status = if out.violation.is_some() { "VIOLATION" } else { "ok" };
    println!("{}/{} seed {}: {status}", p.name, out.scenario, out.seed);
    println!(
        "  {} events, {} messages ({} dropped), {} extern calls ({} failed), {:.1}s simulated",
        s.events,
        s.sent,
        s.dropped,
        s.extern_calls,
        s.extern_failures,
        out.end_ms as f64 / 1000.0
    );
    if let Some(v) = &out.violation {
        println!("  property `{}` violated at {:.3}s: {}", v.property, v.at_ms as f64 / 1000.0, v.detail);
    }
    println!("  final state:");
    for (who, st) in &out.final_state {
        if !st.is_empty() {
            let kv: Vec<String> = st.iter().map(|(k, v)| format!("{k}={v}")).collect();
            println!("    {who:<10} {}", kv.join(" "));
        }
    }
}
