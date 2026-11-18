//! Type inference: Hindley-Milner with row polymorphism, as in Roc.
//!
//! Rows appear twice: anonymous records (`{ list: TodoList, open: Int }`, and
//! field access on records not yet known) and tag unions, which grow by
//! inference (`[EmptyTitle, NoSuchList, ..]`). User records are nominal. A
//! closed tag union in a function's *result* is opened when the function is
//! called, so `problem = err` can widen `problem` (reference: UI, "State").
//!
//! The checker reports a mismatch only when it is sure: anything it can't
//! work out (an unknown name in a doc snippet, a method on a type it doesn't
//! know) becomes a fresh type variable rather than an error.

use std::collections::{BTreeMap, HashMap, HashSet};

use weft_syntax::ast::*;
use weft_syntax::Diagnostic;

use crate::program::{Def, DefKind, Globals, Source};

type Var = u32;

#[derive(Clone, Debug)]
enum Ty {
    Var(Var),
    Con(String, Vec<Ty>),
    Fn(Vec<Ty>, Box<Ty>),
    Rec(BTreeMap<String, Ty>, Option<Var>),
    Tags(BTreeMap<String, Vec<Ty>>, Option<Var>),
}

fn con(n: &str) -> Ty {
    Ty::Con(n.into(), vec![])
}
fn con1(n: &str, a: Ty) -> Ty {
    Ty::Con(n.into(), vec![a])
}
fn unit() -> Ty {
    Ty::Rec(BTreeMap::new(), None)
}
fn html() -> Ty {
    con("Html")
}
fn func(params: Vec<Ty>, ret: Ty) -> Ty {
    Ty::Fn(params, Box::new(ret))
}
fn tags_closed(tags: &[(&str, Vec<Ty>)]) -> Ty {
    Ty::Tags(tags.iter().map(|(n, a)| (n.to_string(), a.clone())).collect(), None)
}
fn bool_ty() -> Ty {
    tags_closed(&[("False", vec![]), ("True", vec![])])
}
fn option(a: Ty) -> Ty {
    tags_closed(&[("None", vec![]), ("Some", vec![a])])
}
fn result(o: Ty, e: Ty) -> Ty {
    tags_closed(&[("Err", vec![e]), ("Ok", vec![o])])
}

struct Record {
    fields: Vec<(String, Type, bool)>,
}

struct Scheme {
    vars: Vec<Var>,
    ty: Ty,
}

struct Component {
    params: Vec<(String, Type, bool)>,
}

pub fn check(sources: &[Source], globals: &Globals, strict: bool) -> Vec<(usize, Diagnostic)> {
    let mut cx = Cx::new(sources, globals, strict);
    cx.collect();
    for fi in 0..sources.len() {
        cx.file = fi;
        for item in &sources[fi].file.items {
            cx.item(item);
        }
    }
    cx.diags
}

struct Cx<'a> {
    sources: &'a [Source],
    g: &'a Globals,
    strict: bool,
    subst: Vec<Option<Ty>>,
    diags: Vec<(usize, Diagnostic)>,
    file: usize,
    records: HashMap<String, Record>,
    components: HashMap<String, Component>,
    aliases: HashMap<String, (Vec<String>, Type)>,
    schemes: HashMap<String, Scheme>,
    in_progress: HashSet<String>,
    scopes: Vec<HashMap<String, Ty>>,
    ret: Vec<Ty>,
    client: bool,
    reported: HashSet<(usize, usize)>,
    /// Variables standing for a whole-number literal: they can only become
    /// `Int`, `Dec` or `Float` (reference: Lexical structure, "Numbers").
    numeric: HashSet<Var>,
    /// Inferring a query or command: calls to commands run, not send.
    server: bool,
}

impl<'a> Cx<'a> {
    fn new(sources: &'a [Source], g: &'a Globals, strict: bool) -> Self {
        Cx {
            sources,
            g,
            strict,
            subst: Vec::new(),
            diags: Vec::new(),
            file: 0,
            records: HashMap::new(),
            components: HashMap::new(),
            aliases: HashMap::new(),
            schemes: HashMap::new(),
            in_progress: HashSet::new(),
            scopes: Vec::new(),
            ret: Vec::new(),
            client: false,
            reported: HashSet::new(),
            numeric: HashSet::new(),
            server: false,
        }
    }

    fn fresh(&mut self) -> Ty {
        self.subst.push(None);
        Ty::Var(self.subst.len() as Var - 1)
    }
    fn fresh_var(&mut self) -> Var {
        self.subst.push(None);
        self.subst.len() as Var - 1
    }

    fn report(&mut self, span: Span, msg: String) {
        if self.reported.insert((self.file, span.start)) {
            self.diags.push((self.file, Diagnostic::error("type", msg, span)));
        }
    }

    fn collect(&mut self) {
        for src in self.sources {
            for item in &src.file.items {
                match &item.kind {
                    ItemKind::Record(r) => {
                        let fields = r
                            .fields
                            .iter()
                            .map(|f| (f.name.name.clone(), f.ty.clone(), f.default.is_some()))
                            .collect();
                        self.records.entry(r.name.name.clone()).or_insert(Record { fields });
                    }
                    ItemKind::Component(c) => {
                        let params = c
                            .params
                            .iter()
                            .map(|p| (p.name.name.clone(), p.ty.clone(), p.default.is_some()))
                            .collect();
                        self.components.entry(c.name.name.clone()).or_insert(Component { params });
                    }
                    ItemKind::Alias(a) => {
                        self.aliases
                            .entry(a.name.name.clone())
                            .or_insert((a.params.iter().map(|p| p.name.clone()).collect(), a.ty.clone()));
                    }
                    _ => {}
                }
            }
        }
    }

    // ── Substitution ──────────────────────────────────────────

    /// Follows bound variables and flattens row extensions.
    fn prune(&self, t: &Ty) -> Ty {
        match t {
            Ty::Var(v) => match &self.subst[*v as usize] {
                Some(b) => self.prune(b),
                None => t.clone(),
            },
            Ty::Rec(fields, Some(tail)) => match &self.subst[*tail as usize] {
                Some(b) => match self.prune(b) {
                    Ty::Rec(more, t2) => {
                        let mut f = fields.clone();
                        f.extend(more);
                        self.prune(&Ty::Rec(f, t2))
                    }
                    _ => t.clone(),
                },
                None => t.clone(),
            },
            Ty::Tags(tags, Some(tail)) => match &self.subst[*tail as usize] {
                Some(b) => match self.prune(b) {
                    Ty::Tags(more, t2) => {
                        let mut f = tags.clone();
                        f.extend(more);
                        self.prune(&Ty::Tags(f, t2))
                    }
                    _ => t.clone(),
                },
                None => t.clone(),
            },
            _ => t.clone(),
        }
    }

    fn occurs(&self, v: Var, t: &Ty) -> bool {
        match self.prune(t) {
            Ty::Var(w) => v == w,
            Ty::Con(_, args) => args.iter().any(|a| self.occurs(v, a)),
            Ty::Fn(p, r) => p.iter().any(|a| self.occurs(v, a)) || self.occurs(v, &r),
            Ty::Rec(f, tail) => tail == Some(v) || f.values().any(|a| self.occurs(v, a)),
            Ty::Tags(f, tail) => tail == Some(v) || f.values().flatten().any(|a| self.occurs(v, a)),
        }
    }

    fn bind(&mut self, v: Var, t: Ty) -> Result<(), String> {
        if let Ty::Var(w) = t {
            if w == v {
                return Ok(());
            }
        }
        if self.occurs(v, &t) {
            return Err("a value can't contain itself".into());
        }
        if self.numeric.contains(&v) {
            match &t {
                Ty::Var(w) => {
                    self.numeric.insert(*w);
                }
                Ty::Con(n, a) if a.is_empty() && (n == "Int" || n == "Dec" || n == "Float") => {}
                other => return Err(format!("a number can't be {}", self.show(other))),
            }
        }
        self.subst[v as usize] = Some(t);
        Ok(())
    }

    fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), String> {
        let a = self.prune(a);
        let b = self.prune(b);
        match (&a, &b) {
            (Ty::Var(x), _) => self.bind(*x, b.clone()),
            (_, Ty::Var(y)) => self.bind(*y, a.clone()),
            (Ty::Con(n1, a1), Ty::Con(n2, a2)) => {
                if n1 != n2 || a1.len() != a2.len() {
                    return Err(self.mismatch(&a, &b));
                }
                for (x, y) in a1.iter().zip(a2) {
                    self.unify(x, y)?;
                }
                Ok(())
            }
            (Ty::Con(n, args), Ty::Rec(fields, tail)) | (Ty::Rec(fields, tail), Ty::Con(n, args))
                if args.is_empty() && self.records.contains_key(n) =>
            {
                let n = n.clone();
                for (f, t) in fields.clone() {
                    match self.record_field(&n, &f) {
                        Some(ft) => self.unify(&ft, &t)?,
                        None => return Err(format!("`{n}` has no field `{f}`")),
                    }
                }
                if tail.is_none() {
                    let missing: Vec<String> = self.records[&n]
                        .fields
                        .iter()
                        .filter(|(f, _, _)| !fields.contains_key(f))
                        .map(|(f, _, _)| format!("`{f}`"))
                        .collect();
                    if !missing.is_empty() {
                        return Err(format!("a record with only these fields isn't a `{n}`: it needs {}", missing.join(", ")));
                    }
                }
                Ok(())
            }
            (Ty::Con(n, args), Ty::Rec(..)) | (Ty::Rec(..), Ty::Con(n, args))
                if args.is_empty() && !self.records.contains_key(n) && !crate::builtins::TYPES.contains(&n.as_str()) =>
            {
                // A record type this program doesn't declare (a doc snippet's
                // `Event`): nothing to check against.
                Ok(())
            }
            (Ty::Fn(p1, r1), Ty::Fn(p2, r2)) => {
                if p1.len() != p2.len() {
                    return Err(format!(
                        "a function taking {} argument{} where one taking {} is expected",
                        p1.len(),
                        if p1.len() == 1 { "" } else { "s" },
                        p2.len()
                    ));
                }
                for (x, y) in p1.iter().zip(p2) {
                    self.unify(x, y)?;
                }
                self.unify(r1, r2)
            }
            (Ty::Rec(f1, t1), Ty::Rec(f2, t2)) => {
                let (f1, t1, f2, t2) = (f1.clone(), *t1, f2.clone(), *t2);
                let mut only1 = BTreeMap::new();
                for (k, v) in &f1 {
                    match f2.get(k) {
                        Some(w) => self.unify(v, w)?,
                        None => {
                            only1.insert(k.clone(), v.clone());
                        }
                    }
                }
                let only2: BTreeMap<String, Ty> =
                    f2.iter().filter(|(k, _)| !f1.contains_key(*k)).map(|(k, v)| (k.clone(), v.clone())).collect();
                self.unify_rows(only1, t1, only2, t2, Ty::Rec, "field")
            }
            (Ty::Tags(f1, t1), Ty::Tags(f2, t2)) => {
                let (f1, t1, f2, t2) = (f1.clone(), *t1, f2.clone(), *t2);
                let mut only1 = BTreeMap::new();
                for (k, v) in &f1 {
                    match f2.get(k) {
                        Some(w) => {
                            if v.len() != w.len() {
                                return Err(format!("`{k}` is used with different numbers of values"));
                            }
                            for (x, y) in v.iter().zip(w) {
                                self.unify(x, y)?;
                            }
                        }
                        None => {
                            only1.insert(k.clone(), v.clone());
                        }
                    }
                }
                let only2: BTreeMap<String, Vec<Ty>> =
                    f2.iter().filter(|(k, _)| !f1.contains_key(*k)).map(|(k, v)| (k.clone(), v.clone())).collect();
                self.unify_rows(only1, t1, only2, t2, Ty::Tags, "tag")
            }
            _ => Err(self.mismatch(&a, &b)),
        }
    }

    fn unify_rows<V: Clone>(
        &mut self,
        only1: BTreeMap<String, V>,
        t1: Option<Var>,
        only2: BTreeMap<String, V>,
        t2: Option<Var>,
        make: fn(BTreeMap<String, V>, Option<Var>) -> Ty,
        what: &str,
    ) -> Result<(), String> {
        let extra = |m: &BTreeMap<String, V>| m.keys().map(|k| format!("`{k}`")).collect::<Vec<_>>().join(", ");
        match (t1, t2) {
            (None, None) => {
                if !only1.is_empty() || !only2.is_empty() {
                    let side = if only1.is_empty() { &only2 } else { &only1 };
                    return Err(format!("the {what}s don't match: {} isn't in both", extra(side)));
                }
                Ok(())
            }
            (Some(r1), None) => {
                if !only1.is_empty() {
                    return Err(format!("{} isn't one of the {what}s allowed here", extra(&only1)));
                }
                self.bind(r1, make(only2, None))
            }
            (None, Some(r2)) => {
                if !only2.is_empty() {
                    return Err(format!("{} isn't one of the {what}s allowed here", extra(&only2)));
                }
                self.bind(r2, make(only1, None))
            }
            (Some(r1), Some(r2)) => {
                if r1 == r2 {
                    if only1.is_empty() && only2.is_empty() {
                        return Ok(());
                    }
                    return Err(format!("the {what}s don't match"));
                }
                let r3 = self.fresh_var();
                self.bind(r1, make(only2, Some(r3)))?;
                self.bind(r2, make(only1, Some(r3)))
            }
        }
    }

    fn mismatch(&self, a: &Ty, b: &Ty) -> String {
        format!("expected {}, found {}", self.show(b), self.show(a))
    }

    fn show(&self, t: &Ty) -> String {
        match self.prune(t) {
            Ty::Var(_) => "_".into(),
            Ty::Con(n, args) if args.is_empty() => n,
            Ty::Con(n, args) => format!("{n}({})", args.iter().map(|a| self.show(a)).collect::<Vec<_>>().join(", ")),
            Ty::Fn(p, r) => format!(
                "|{}| -> {}",
                p.iter().map(|a| self.show(a)).collect::<Vec<_>>().join(", "),
                self.show(&r)
            ),
            Ty::Rec(f, tail) => {
                let mut parts: Vec<String> = f.iter().map(|(k, v)| format!("{k}: {}", self.show(v))).collect();
                if tail.is_some() {
                    parts.push("..".into());
                }
                if parts.is_empty() {
                    "{}".into()
                } else {
                    format!("{{ {} }}", parts.join(", "))
                }
            }
            Ty::Tags(f, tail) => {
                let keys: Vec<&String> = f.keys().collect();
                if tail.is_none() && keys == ["False", "True"] {
                    return "Bool".into();
                }
                if tail.is_none() && keys == ["None", "Some"] {
                    return format!("Option({})", self.show(&f["Some"][0]));
                }
                if tail.is_none() && keys == ["Err", "Ok"] {
                    return format!("Result({}, {})", self.show(&f["Ok"][0]), self.show(&f["Err"][0]));
                }
                let mut parts: Vec<String> = f
                    .iter()
                    .map(|(k, v)| {
                        if v.is_empty() {
                            k.clone()
                        } else {
                            format!("{k}({})", v.iter().map(|a| self.show(a)).collect::<Vec<_>>().join(", "))
                        }
                    })
                    .collect();
                if tail.is_some() {
                    parts.push("..".into());
                }
                format!("[{}]", parts.join(", "))
            }
        }
    }

    // ── Annotations to types ──────────────────────────────────

    fn from_ast(&mut self, t: &Type, vars: &mut HashMap<String, Ty>) -> Ty {
        match t {
            Type::Name { name, args } => {
                let args: Vec<Ty> = args.iter().map(|a| self.from_ast(a, vars)).collect();
                match (name.name.as_str(), args.as_slice()) {
                    ("Bool", []) => bool_ty(),
                    // A handler: what it returns doesn't matter (it may wait).
                    ("Action", []) => {
                        let r = self.fresh();
                        func(vec![], r)
                    }
                    ("Action", [a]) => {
                        let r = self.fresh();
                        func(vec![a.clone()], r)
                    }
                    ("Option", [a]) => option(a.clone()),
                    ("Result", [o, e]) => result(o.clone(), e.clone()),
                    (n, _) if self.aliases.contains_key(n) => {
                        let (params, body) = self.aliases[n].clone();
                        let mut inner: HashMap<String, Ty> =
                            params.iter().cloned().zip(args.iter().cloned()).collect();
                        self.from_ast(&body, &mut inner)
                    }
                    (n, _) => Ty::Con(n.to_string(), args),
                }
            }
            Type::Var(v) => {
                if let Some(t) = vars.get(&v.name) {
                    return t.clone();
                }
                let f = self.fresh();
                vars.insert(v.name.clone(), f.clone());
                f
            }
            Type::Fn { params, result, .. } => {
                // `() -> T` takes no arguments.
                let params: &[Type] = if matches!(params.as_slice(), [Type::Unit(_)]) { &[] } else { params };
                let p = params.iter().map(|x| self.from_ast(x, vars)).collect();
                let r = self.from_ast(result, vars);
                func(p, r)
            }
            Type::Record { fields, .. } => {
                Ty::Rec(fields.iter().map(|(n, t)| (n.name.clone(), self.from_ast(t, vars))).collect(), None)
            }
            Type::Tags { tags, open, .. } => {
                let tail = if *open { Some(self.fresh_var()) } else { None };
                Ty::Tags(
                    tags.iter()
                        .map(|(n, args)| (n.name.clone(), args.iter().map(|a| self.from_ast(a, vars)).collect()))
                        .collect(),
                    tail,
                )
            }
            Type::Unit(_) => unit(),
            Type::Error(_) => self.fresh(),
        }
    }

    /// A closed tag union produced by a function can be used as an open one:
    /// opens the unions in covariant positions of `t`.
    fn open_outputs(&mut self, t: &Ty) -> Ty {
        match self.prune(t) {
            Ty::Tags(f, None) => {
                let tail = self.fresh_var();
                let f = f.into_iter().map(|(k, v)| (k, v.iter().map(|x| self.open_outputs(x)).collect())).collect();
                Ty::Tags(f, Some(tail))
            }
            Ty::Tags(f, tail) => {
                Ty::Tags(f.into_iter().map(|(k, v)| (k, v.iter().map(|x| self.open_outputs(x)).collect())).collect(), tail)
            }
            Ty::Con(n, args) => Ty::Con(n, args.iter().map(|a| self.open_outputs(a)).collect()),
            Ty::Rec(f, tail) => Ty::Rec(f.into_iter().map(|(k, v)| (k, self.open_outputs(&v))).collect(), tail),
            other => other,
        }
    }

    // ── Schemes ───────────────────────────────────────────────

    fn free_vars(&self, t: &Ty, out: &mut Vec<Var>) {
        match self.prune(t) {
            Ty::Var(v) => {
                if !out.contains(&v) {
                    out.push(v)
                }
            }
            Ty::Con(_, args) => args.iter().for_each(|a| self.free_vars(a, out)),
            Ty::Fn(p, r) => {
                p.iter().for_each(|a| self.free_vars(a, out));
                self.free_vars(&r, out);
            }
            Ty::Rec(f, tail) => {
                f.values().for_each(|a| self.free_vars(a, out));
                if let Some(v) = tail {
                    if !out.contains(&v) {
                        out.push(v)
                    }
                }
            }
            Ty::Tags(f, tail) => {
                f.values().flatten().for_each(|a| self.free_vars(a, out));
                if let Some(v) = tail {
                    if !out.contains(&v) {
                        out.push(v)
                    }
                }
            }
        }
    }

    fn generalize(&self, t: &Ty) -> Scheme {
        let mut vars = Vec::new();
        self.free_vars(t, &mut vars);
        Scheme { vars, ty: self.resolve_fully(t) }
    }

    fn resolve_fully(&self, t: &Ty) -> Ty {
        match self.prune(t) {
            Ty::Con(n, args) => Ty::Con(n, args.iter().map(|a| self.resolve_fully(a)).collect()),
            Ty::Fn(p, r) => Ty::Fn(p.iter().map(|a| self.resolve_fully(a)).collect(), Box::new(self.resolve_fully(&r))),
            Ty::Rec(f, tail) => Ty::Rec(f.iter().map(|(k, v)| (k.clone(), self.resolve_fully(v))).collect(), tail),
            Ty::Tags(f, tail) => Ty::Tags(
                f.iter().map(|(k, v)| (k.clone(), v.iter().map(|x| self.resolve_fully(x)).collect())).collect(),
                tail,
            ),
            v => v,
        }
    }

    fn instantiate(&mut self, s: &Scheme) -> Ty {
        let map: HashMap<Var, Var> = s.vars.iter().map(|v| (*v, self.fresh_var())).collect();
        fn go(t: &Ty, m: &HashMap<Var, Var>) -> Ty {
            match t {
                Ty::Var(v) => Ty::Var(*m.get(v).unwrap_or(v)),
                Ty::Con(n, a) => Ty::Con(n.clone(), a.iter().map(|x| go(x, m)).collect()),
                Ty::Fn(p, r) => Ty::Fn(p.iter().map(|x| go(x, m)).collect(), Box::new(go(r, m))),
                Ty::Rec(f, tail) => Ty::Rec(f.iter().map(|(k, v)| (k.clone(), go(v, m))).collect(), tail.map(|v| *m.get(&v).unwrap_or(&v))),
                Ty::Tags(f, tail) => Ty::Tags(
                    f.iter().map(|(k, v)| (k.clone(), v.iter().map(|x| go(x, m)).collect())).collect(),
                    tail.map(|v| *m.get(&v).unwrap_or(&v)),
                ),
            }
        }
        go(&s.ty, &map)
    }

    /// The type of a top-level name, inferring it on first use.
    fn global(&mut self, name: &str) -> Option<Ty> {
        if let Some(s) = self.schemes.get(name) {
            let s = Scheme { vars: s.vars.clone(), ty: s.ty.clone() };
            return Some(self.instantiate(&s));
        }
        let def: Def = self.g.values.get(name)?.clone();
        if self.in_progress.contains(name) {
            return Some(self.fresh());
        }
        self.in_progress.insert(name.to_string());
        let saved = (self.file, std::mem::take(&mut self.scopes), std::mem::take(&mut self.ret), self.client);
        self.file = def.file;
        self.client = false;
        let ty = match &def.kind {
            DefKind::Table(_) => {
                let rec = self.table_record(name);
                Some(con1("Table", rec))
            }
            DefKind::Check => Some(con("Check")),
            DefKind::Config(n) => Some(con(n)),
            DefKind::Handler => Some(con("Handler")),
            DefKind::Component => Some(html()),
            _ => self.binding_value(name).map(|v| {
                let t = self.top_value(name, v);
                t
            }),
        };
        (self.file, self.scopes, self.ret, self.client) = saved;
        self.in_progress.remove(name);
        let ty = ty?;
        let scheme = self.generalize(&ty);
        let inst = self.instantiate(&scheme);
        self.schemes.insert(name.to_string(), scheme);
        Some(inst)
    }

    fn binding_value(&self, name: &str) -> Option<&'a Expr> {
        let def = self.g.values.get(name)?;
        for item in &self.sources[def.file].file.items {
            if let ItemKind::Binding(b) = &item.kind {
                if matches!(&b.left, Pattern::Name(n) if n.name == name) {
                    return Some(&b.value);
                }
            }
        }
        None
    }

    fn table_record(&mut self, name: &str) -> Ty {
        if let Some(Expr { kind: ExprKind::Call { args, .. }, .. }) = self.binding_value(name) {
            if let Some(Arg { value: Expr { kind: ExprKind::Tag(r), .. }, .. }) = args.first() {
                return con(r);
            }
        }
        self.fresh()
    }

    /// Infers a top-level value, using its signature if it has one.
    fn top_value(&mut self, name: &str, value: &'a Expr) -> Ty {
        let sig = self.g.signatures.get(name).map(|s| s.ty.clone());
        match &value.kind {
            ExprKind::Lambda(l) => {
                let declared = sig.map(|s| self.from_ast(&s, &mut HashMap::new()));
                let t = self.lambda(l, declared.clone());
                if let Some(d) = declared {
                    if let Err(e) = self.unify(&t, &d) {
                        self.report(value.span, format!("`{name}` doesn't match its signature: {e}"));
                    }
                }
                t
            }
            ExprKind::Call { function, args } if matches!(&function.kind, ExprKind::Tag(t) if t == "Query" || t == "Command") => {
                let saved = self.server;
                self.server = true;
                let t = match args.first().map(|a| &a.value.kind) {
                    Some(ExprKind::Lambda(l)) => self.lambda(l, None),
                    _ => self.fresh(),
                };
                self.server = saved;
                t
            }
            ExprKind::TypedRecord { ty, .. } if ty.name == "Extern" => match sig {
                Some(s) => self.from_ast(&s, &mut HashMap::new()),
                None => self.fresh(),
            },
            _ => {
                let t = self.expr(value, false);
                if let Some(s) = sig {
                    let d = self.from_ast(&s, &mut HashMap::new());
                    if let Err(e) = self.unify(&t, &d) {
                        self.report(value.span, format!("`{name}` doesn't match its signature: {e}"));
                    }
                }
                t
            }
        }
    }

    // ── Items ─────────────────────────────────────────────────

    fn item(&mut self, item: &'a Item) {
        self.client = false;
        self.scopes.clear();
        self.ret.clear();
        match &item.kind {
            ItemKind::Binding(b) => {
                if let Pattern::Name(n) = &b.left {
                    // Only infer each top-level name once, in its own file.
                    if self.g.values.get(&n.name).is_some_and(|d| d.file == self.file && d.span == n.span) {
                        if matches!(self.g.values[&n.name].kind, DefKind::Function { stateful: true }) {
                            self.client = true;
                        }
                        let _ = self.global(&n.name);
                    }
                }
            }
            ItemKind::Expect(e) => {
                let t = self.expr(e, false);
                if let Err(err) = self.unify(&t, &bool_ty()) {
                    self.report(e.span, format!("`expect` needs a Bool: {err}"));
                }
            }
            ItemKind::Component(c) => self.component(c),
            ItemKind::Record(r) => {
                for f in &r.fields {
                    if let Some(d) = &f.default {
                        let t = self.expr(d, false);
                        let want = self.from_ast(&f.ty, &mut HashMap::new());
                        let want = self.id_context(&want, &r.name.name);
                        if let Err(e) = self.unify(&t, &want) {
                            self.report(d.span, format!("the default for `{}` is the wrong type: {e}", f.name.name));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// `new_id()` in a field `key id: Id(Item)` gets its type from context.
    fn id_context(&mut self, t: &Ty, _record: &str) -> Ty {
        t.clone()
    }

    fn component(&mut self, c: &'a weft_syntax::ast::Component) {
        self.client = true;
        self.scopes.push(HashMap::new());
        for p in &c.params {
            let t = self.from_ast(&p.ty, &mut HashMap::new());
            if let Some(d) = &p.default {
                let dt = self.expr(d, false);
                if let Err(e) = self.unify(&dt, &t) {
                    self.report(d.span, format!("the default for `{}` is the wrong type: {e}", p.name.name));
                }
            }
            self.declare(&p.name.name, t);
        }
        // Members are visible in any order: declare them all first.
        for s in &c.body.stmts {
            match &s.kind {
                StmtKind::State { name, ty, .. } => {
                    let t = match ty {
                        Some(t) => self.from_ast(t, &mut HashMap::new()),
                        None => self.fresh(),
                    };
                    self.declare(&name.name, t);
                }
                StmtKind::Binding(b) => {
                    for n in b.left.names() {
                        let f = self.fresh();
                        self.declare(&n.name, f);
                    }
                }
                _ => {}
            }
        }
        for s in &c.body.stmts {
            match &s.kind {
                StmtKind::State { name, value, .. } => {
                    let t = self.expr(value, false);
                    let declared = self.lookup(&name.name).unwrap_or_else(|| self.fresh());
                    if let Err(e) = self.unify(&t, &declared) {
                        self.report(value.span, format!("`{}` starts with the wrong type: {e}", name.name));
                    }
                }
                StmtKind::Binding(b) => {
                    let t = self.expr(&b.value, false);
                    if let Pattern::Name(n) = &b.left {
                        let declared = self.lookup(&n.name).unwrap_or_else(|| self.fresh());
                        let _ = self.unify(&t, &declared);
                    }
                }
                _ => {
                    let _ = self.stmt(s);
                }
            }
        }
        self.scopes.pop();
        self.client = false;
    }

    // ── Scopes ────────────────────────────────────────────────

    fn declare(&mut self, name: &str, t: Ty) {
        if let Some(s) = self.scopes.last_mut() {
            s.insert(name.to_string(), t);
        }
    }

    fn lookup(&self, name: &str) -> Option<Ty> {
        for s in self.scopes.iter().rev() {
            if let Some(t) = s.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    fn name(&mut self, n: &str) -> Ty {
        if let Some(t) = self.lookup(n) {
            return t;
        }
        if let Some(t) = self.global(n) {
            return t;
        }
        match n {
            "nothing" | "outlet" => html(),
            _ => self.fresh(),
        }
    }

    // ── Lambdas ───────────────────────────────────────────────

    fn lambda(&mut self, l: &'a Lambda, declared: Option<Ty>) -> Ty {
        let (dparams, dret) = match declared.map(|d| self.prune(&d)) {
            Some(Ty::Fn(p, r)) if p.len() == l.params.len() => (Some(p), Some(*r)),
            _ => (None, None),
        };
        self.scopes.push(HashMap::new());
        let mut params = Vec::new();
        let mut vars = HashMap::new();
        for (i, p) in l.params.iter().enumerate() {
            let t = match (&p.ty, &dparams) {
                (Some(t), _) => self.from_ast(t, &mut vars),
                (None, Some(d)) => d[i].clone(),
                (None, None) => self.fresh(),
            };
            self.bind_pattern(&p.pattern, &t);
            params.push(t);
        }
        let ret = match (&l.result, dret) {
            (Some(r), _) => self.from_ast(r, &mut vars),
            (None, Some(r)) => r,
            (None, None) => self.fresh(),
        };
        let waits = contains_await(&l.body);
        // `return` and `?` refer to what the body produces, before `Pending`.
        self.ret.push(ret.clone());
        let body = self.expr(&l.body, false);
        self.ret.pop();
        let snippet_wrapper = !self.strict && self.in_progress.contains("__snippet") && self.ret.is_empty();
        if snippet_wrapper {
            // A doc snippet's statements aren't a real function body.
        } else if let Err(e) = self.unify(&body, &ret) {
            let span = last_expr_span(&l.body);
            self.report(span, format!("this function should give {}, but this is {}: {e}", self.show(&ret), self.show(&body)));
        }
        self.scopes.pop();
        let ret = if waits { con1("Pending", ret) } else { ret };
        func(params, ret)
    }

    // ── Statements ────────────────────────────────────────────

    fn block(&mut self, b: &'a Block) -> Ty {
        self.scopes.push(HashMap::new());
        let mut last = unit();
        for (i, s) in b.stmts.iter().enumerate() {
            let is_last = i + 1 == b.stmts.len();
            let t = self.stmt(s);
            if is_last {
                last = t;
            }
        }
        self.scopes.pop();
        last
    }

    fn stmt(&mut self, s: &'a Stmt) -> Ty {
        match &s.kind {
            StmtKind::Binding(b) => {
                let t = self.expr(&b.value, false);
                // An assignment to a `let mut` or `state`.
                if let Pattern::Name(n) = &b.left {
                    match self.lookup(&n.name) {
                        Some(cur) => {
                            if let Err(e) = self.unify(&t, &cur) {
                                self.report(b.value.span, format!("`{}` can't take this value: {e}", n.name));
                            }
                        }
                        None => self.declare(&n.name, t),
                    }
                }
                unit()
            }
            StmtKind::Let { pattern, value, otherwise, .. } => {
                let t = self.expr(value, false);
                if let Some(b) = otherwise {
                    self.block(b);
                }
                self.bind_pattern(pattern, &t);
                unit()
            }
            StmtKind::State { name, value, .. } => {
                let t = self.expr(value, false);
                match self.lookup(&name.name) {
                    Some(cur) => {
                        let _ = self.unify(&t, &cur);
                    }
                    None => self.declare(&name.name, t),
                }
                unit()
            }
            StmtKind::Style | StmtKind::Break | StmtKind::Continue => unit(),
            StmtKind::For { pattern, iterable, body } => {
                let it = self.expr(iterable, false);
                let elem = match self.prune(&it) {
                    Ty::Con(n, args) if (n == "List" || n == "Set" || n == "Range") && args.len() == 1 => args[0].clone(),
                    Ty::Var(_) => {
                        let e = self.fresh();
                        let _ = self.unify(&it, &con1("List", e.clone()));
                        e
                    }
                    _ => self.fresh(),
                };
                self.scopes.push(HashMap::new());
                self.bind_pattern(pattern, &elem);
                self.block(body);
                self.scopes.pop();
                unit()
            }
            StmtKind::While { condition, body } => {
                self.condition(condition);
                self.block(body);
                unit()
            }
            StmtKind::Return(e) => {
                let t = match e {
                    Some(e) => self.expr(e, false),
                    None => unit(),
                };
                if let Some(r) = self.ret.last().cloned() {
                    if let Err(err) = self.unify(&t, &r) {
                        let span = e.as_ref().map(|e| e.span).unwrap_or(s.span);
                        self.report(span, format!("this returns {}, but the function gives {}: {err}", self.show(&t), self.show(&r)));
                    }
                }
                self.fresh()
            }
            StmtKind::With { handler, body } => {
                self.expr(handler, false);
                self.block(body)
            }
            StmtKind::Defer(b) => {
                self.block(b);
                unit()
            }
            StmtKind::Expr(e) => self.expr(e, true),
        }
    }

    fn condition(&mut self, c: &'a Expr) {
        let t = self.expr(c, false);
        if let Err(e) = self.unify(&t, &bool_ty()) {
            self.report(c.span, format!("a condition must be a Bool: {e}"));
        }
    }

    fn bind_pattern(&mut self, p: &'a Pattern, t: &Ty) {
        let pt = self.pattern(p);
        if let Err(e) = self.unify(&pt, t) {
            self.report(p.span(), format!("this pattern doesn't fit the value: {e}"));
        }
    }

    /// The type a pattern matches, binding its names.
    fn pattern(&mut self, p: &'a Pattern) -> Ty {
        match p {
            Pattern::Name(n) => {
                let t = self.fresh();
                self.declare(&n.name, t.clone());
                t
            }
            Pattern::Wildcard(_) | Pattern::Error(_) => self.fresh(),
            Pattern::Tag { name, args } => {
                if (name.name == "True" || name.name == "False") && args.is_empty() {
                    return bool_ty();
                }
                let args: Vec<Ty> = args.iter().map(|a| self.pattern(a)).collect();
                let tail = self.fresh_var();
                Ty::Tags([(name.name.clone(), args)].into_iter().collect(), Some(tail))
            }
            Pattern::Record { fields, .. } => {
                let mut f = BTreeMap::new();
                for (name, sub) in fields {
                    let t = match sub {
                        Some(sp) => self.pattern(sp),
                        None => {
                            let t = self.fresh();
                            self.declare(&name.name, t.clone());
                            t
                        }
                    };
                    f.insert(name.name.clone(), t);
                }
                let tail = self.fresh_var();
                Ty::Rec(f, Some(tail))
            }
            Pattern::List { items, rest, .. } => {
                let e = self.fresh();
                for i in items {
                    let t = self.pattern(i);
                    let _ = self.unify(&t, &e);
                }
                let list = con1("List", e);
                if let Some(Some(r)) = rest {
                    self.declare(&r.name, list.clone());
                }
                list
            }
            Pattern::Literal(e) => self.expr(e, false),
            Pattern::Or(alts, _) => {
                // Each side binds the same names; bind them once, from the
                // first side, and match the others against the same type.
                let t = self.pattern(&alts[0]);
                for alt in &alts[1..] {
                    self.scopes.push(HashMap::new());
                    let at = self.pattern(alt);
                    self.scopes.pop();
                    if let Err(x) = self.unify(&at, &t) {
                        self.report(alt.span(), format!("each side of `|` must match the same type: {x}"));
                    }
                }
                t
            }
        }
    }

    // ── Expressions ───────────────────────────────────────────

    /// `stmt`: the expression is a statement, so its value is unused and
    /// branches of `if` and `match` needn't agree.
    fn expr(&mut self, e: &'a Expr, stmt: bool) -> Ty {
        match &e.kind {
            ExprKind::Name(n) => self.name(n),
            ExprKind::Tag(t) => self.tag(t, e.span),
            ExprKind::Number { kind, .. } => match kind {
                NumberKind::Int => {
                    let v = self.fresh_var();
                    self.numeric.insert(v);
                    Ty::Var(v)
                }
                NumberKind::Dec | NumberKind::Percent => con("Dec"),
                NumberKind::Float => con("Float"),
                NumberKind::Duration => con("Duration"),
                NumberKind::Size => con("Size"),
            },
            ExprKind::Text(parts) => {
                for p in parts {
                    if let TextPart::Interpolation(x) = p {
                        self.expr(x, false);
                    }
                }
                con("Text")
            }
            ExprKind::List(items) => {
                let el = self.fresh();
                for i in items {
                    let t = self.expr(i, false);
                    if let Err(err) = self.unify(&t, &el) {
                        self.report(i.span, format!("list items must all be one type: {err}"));
                    }
                }
                con1("List", el)
            }
            ExprKind::Record(fields) => self.record_literal(fields),
            ExprKind::TypedRecord { ty, fields } => self.typed_record(ty, fields, e.span),
            ExprKind::Block(b) => self.block(b),
            ExprKind::Lambda(l) => self.lambda(l, None),
            ExprKind::If { pattern, condition, then, otherwise } => {
                self.scopes.push(HashMap::new());
                match pattern {
                    Some(p) => {
                        let t = self.expr(condition, false);
                        let pt = self.pattern(p);
                        if let Err(x) = self.unify(&pt, &t) {
                            self.report(p.span(), format!("this pattern can't match {}: {x}", self.show(&t)));
                        }
                    }
                    None => self.condition(condition),
                }
                let a = self.block(then);
                self.scopes.pop();
                match otherwise {
                    Some(o) => {
                        let b = self.expr(o, stmt);
                        if !stmt {
                            if let Err(err) = self.unify(&a, &b) {
                                self.report(o.span, format!("both branches of `if` must give the same type: {err}"));
                            }
                        }
                        a
                    }
                    None => unit(),
                }
            }
            ExprKind::Match { value, arms } => self.match_expr(value, arms, stmt, e.span),
            ExprKind::Accessor(f) => {
                let v = self.fresh();
                let tail = self.fresh_var();
                func(vec![Ty::Rec([(f.name.clone(), v.clone())].into_iter().collect(), Some(tail))], v)
            }
            ExprKind::Unit => unit(),
            ExprKind::Call { function, args } => self.call(e, function, args),
            ExprKind::Element { head, children } => {
                if let ExprKind::Call { args, .. } = &head.kind {
                    for a in args {
                        self.expr(&a.value, false);
                    }
                }
                self.scopes.push(HashMap::new());
                for s in &children.stmts {
                    self.stmt(s);
                }
                self.scopes.pop();
                html()
            }
            ExprKind::Field { value, field } => {
                let t = self.expr(value, false);
                self.field(&t, &field.name, field.span)
            }
            ExprKind::Await(inner) => {
                let t = self.expr(inner, false);
                let v = self.fresh();
                if let Err(err) = self.unify(&t, &con1("Pending", v.clone())) {
                    self.report(e.span, format!("`.await` needs something that takes time (a Pending): {err}"));
                }
                v
            }
            ExprKind::Try(inner) => {
                let t = self.expr(inner, false);
                let o = self.fresh();
                let err = self.fresh();
                if let Err(x) = self.unify(&t, &result(o.clone(), err.clone())) {
                    self.report(e.span, format!("`?` needs a Result: {x}"));
                    return o;
                }
                if let Some(r) = self.ret.last().cloned() {
                    let ro = self.fresh();
                    let opened = self.open_outputs(&err);
                    if let Err(x) = self.unify(&r, &result(ro, opened)) {
                        self.report(e.span, format!("`?` passes this error on, but the function can't return it: {x}"));
                    }
                }
                o
            }
            ExprKind::Unary { op, operand } => {
                let t = self.expr(operand, false);
                match op {
                    UnOp::Not => {
                        if let Err(x) = self.unify(&t, &bool_ty()) {
                            self.report(operand.span, format!("`not` needs a Bool: {x}"));
                        }
                        bool_ty()
                    }
                    UnOp::Neg => t,
                }
            }
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, e.span),
            ExprKind::Crash(m) => {
                self.expr(m, false);
                self.fresh()
            }
            ExprKind::Assign { name, value } => {
                let t = self.expr(value, false);
                if let Some(cur) = self.lookup(&name.name) {
                    if let Err(x) = self.unify(&t, &cur) {
                        self.report(value.span, format!("`{}` can't take this value: {x}", name.name));
                    }
                }
                unit()
            }
            ExprKind::Error => self.fresh(),
        }
    }

    fn tag(&mut self, t: &str, span: Span) -> Ty {
        match t {
            "True" | "False" => bool_ty(),
            _ if self.components.contains_key(t) => html(),
            _ if self.records.contains_key(t) => {
                self.report(span, format!("`{t}` is a record type; build one with `{t} {{ … }}`"));
                self.fresh()
            }
            _ => {
                let tail = self.fresh_var();
                Ty::Tags([(t.to_string(), vec![])].into_iter().collect(), Some(tail))
            }
        }
    }

    fn record_literal(&mut self, fields: &'a [Field]) -> Ty {
        let mut base: Option<Ty> = None;
        let mut out = BTreeMap::new();
        for f in fields {
            match f {
                Field::Spread(v) => base = Some(self.expr(v, false)),
                Field::Named { name, value } => {
                    let t = self.expr(value, false);
                    out.insert(name.name.clone(), (t, value.span));
                }
                Field::Shorthand(n) => {
                    let t = self.name(&n.name);
                    out.insert(n.name.clone(), (t, n.span));
                }
            }
        }
        match base {
            // `{ ..r, f: v }` has r's type; each new field must fit it.
            Some(b) => {
                for (name, (t, span)) in out {
                    let ft = self.field(&b, &name, span);
                    if let Err(x) = self.unify(&t, &ft) {
                        self.report(span, format!("`{name}` can't take this value: {x}"));
                    }
                }
                b
            }
            None => Ty::Rec(out.into_iter().map(|(k, (t, _))| (k, t)).collect(), None),
        }
    }

    fn typed_record(&mut self, ty: &Ident, fields: &'a [Field], span: Span) -> Ty {
        let Some(rec) = self.records.get(&ty.name) else {
            for f in fields {
                if let Field::Named { value, .. } | Field::Spread(value) = f {
                    self.expr(value, false);
                }
            }
            return con(&ty.name);
        };
        let declared: Vec<(String, Type, bool)> = rec.fields.clone();
        let mut given = HashSet::new();
        let mut has_spread = false;
        for f in fields {
            let (name, t, fspan) = match f {
                Field::Named { name, value } => (name.name.clone(), self.expr(value, false), value.span),
                Field::Shorthand(n) => (n.name.clone(), self.name(&n.name), n.span),
                Field::Spread(v) => {
                    has_spread = true;
                    let t = self.expr(v, false);
                    if let Err(x) = self.unify(&t, &con(&ty.name)) {
                        self.report(v.span, format!("`..` copies a `{}` here: {x}", ty.name));
                    }
                    continue;
                }
            };
            given.insert(name.clone());
            match declared.iter().find(|(n, _, _)| *n == name) {
                Some((_, fty, _)) => {
                    let want = self.from_ast(fty, &mut HashMap::new());
                    if let Err(x) = self.unify(&t, &want) {
                        self.report(fspan, format!("`{}.{name}` is a {}: {x}", ty.name, self.show(&want)));
                    }
                }
                None => self.report(fspan, format!("`{}` has no field `{name}`", ty.name)),
            }
        }
        if !has_spread {
            let missing: Vec<String> = declared
                .iter()
                .filter(|(n, _, has_default)| !has_default && !given.contains(n))
                .map(|(n, _, _)| format!("`{n}`"))
                .collect();
            if !missing.is_empty() {
                self.report(span, format!("`{}` needs {}", ty.name, missing.join(", ")));
            }
        }
        con(&ty.name)
    }

    fn match_expr(&mut self, value: &'a Expr, arms: &'a [Arm], stmt: bool, span: Span) -> Ty {
        let scrutinee = self.expr(value, false);
        let out = self.fresh();
        let mut covered: Vec<String> = Vec::new();
        let mut catch_all = false;
        for a in arms {
            self.scopes.push(HashMap::new());
            let pt = self.pattern(&a.pattern);
            if let Err(x) = self.unify(&pt, &scrutinee) {
                self.report(a.pattern.span(), format!("this pattern can't match {}: {x}", self.show(&scrutinee)));
            }
            match &a.pattern {
                Pattern::Tag { name, .. } if a.guard.is_none() => covered.push(name.name.clone()),
                Pattern::Or(alts, _) if a.guard.is_none() => {
                    for alt in alts {
                        match alt {
                            Pattern::Tag { name, .. } => covered.push(name.name.clone()),
                            Pattern::Name(_) | Pattern::Wildcard(_) => catch_all = true,
                            _ => {}
                        }
                    }
                }
                Pattern::Name(_) | Pattern::Wildcard(_) if a.guard.is_none() => catch_all = true,
                _ => {}
            }
            if let Some(g) = &a.guard {
                self.condition(g);
            }
            let t = self.expr(&a.value, stmt);
            if !stmt {
                if let Err(x) = self.unify(&t, &out) {
                    self.report(a.value.span, format!("every arm of `match` must give the same type: {x}"));
                }
            }
            self.scopes.pop();
        }
        // Without a catch-all arm, the arms close the union; a tag the
        // scrutinee can have but no arm handles is an error.
        if !catch_all && !covered.is_empty() {
            if let Ty::Tags(tags, tail) = self.prune(&scrutinee) {
                if let Some(t) = tail {
                    let _ = self.bind(t, Ty::Tags(BTreeMap::new(), None));
                }
                let missing: Vec<String> =
                    tags.keys().filter(|k| !covered.contains(k)).map(|k| format!("`{k}`")).collect();
                if !missing.is_empty() {
                    self.report(span, format!("this `match` doesn't handle {}", missing.join(", ")));
                }
            }
        }
        if stmt {
            unit()
        } else {
            out
        }
    }

    fn binary(&mut self, op: BinOp, left: &'a Expr, right: &'a Expr, span: Span) -> Ty {
        let l = self.expr(left, false);
        let r = self.expr(right, false);
        match op {
            BinOp::And | BinOp::Or => {
                for (t, e) in [(&l, left), (&r, right)] {
                    if let Err(x) = self.unify(t, &bool_ty()) {
                        self.report(e.span, format!("`and` and `or` need Bools: {x}"));
                    }
                }
                bool_ty()
            }
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                if let Err(x) = self.unify(&l, &r) {
                    self.report(span, format!("can't compare {} with {}: {x}", self.show(&l), self.show(&r)));
                }
                bool_ty()
            }
            BinOp::RangeExclusive | BinOp::RangeInclusive => {
                if let Err(x) = self.unify(&l, &r) {
                    self.report(span, format!("a range's ends must be one type: {x}"));
                }
                con1("Range", l)
            }
            BinOp::Add | BinOp::Sub => {
                let (pl, pr) = (self.prune(&l), self.prune(&r));
                let is = |t: &Ty, n: &str| matches!(t, Ty::Con(c, a) if c == n && a.is_empty());
                if (is(&pl, "Time") || is(&pl, "Date")) && is(&pr, "Duration") {
                    return pl;
                }
                if op == BinOp::Sub && is(&pl, "Time") && is(&pr, "Time") {
                    return con("Duration");
                }
                if let Err(x) = self.unify(&l, &r) {
                    self.report(span, format!("can't combine {} and {}: {x}", self.show(&l), self.show(&r)));
                }
                l
            }
            BinOp::Mul | BinOp::Div | BinOp::Rem => {
                if matches!(self.prune(&l), Ty::Con(c, _) if c == "Duration") {
                    return l;
                }
                if let Err(x) = self.unify(&l, &r) {
                    self.report(span, format!("can't combine {} and {}: {x}", self.show(&l), self.show(&r)));
                }
                l
            }
        }
    }

    fn call(&mut self, e: &'a Expr, function: &'a Expr, args: &'a [Arg]) -> Ty {
        match &function.kind {
            ExprKind::Tag(t) => return self.tag_call(t, args, e.span),
            ExprKind::Field { value, field } => return self.method_call(e, value, &field.name, field.span, args),
            ExprKind::Name(n) if self.lookup(n).is_none() => {
                if let Some(t) = self.builtin_call(n, args) {
                    return t;
                }
                if crate::builtins::ELEMENTS.contains(&n.as_str()) && self.g.values.get(n).is_none() {
                    for a in args {
                        self.expr(&a.value, false);
                    }
                    return html();
                }
            }
            _ => {}
        }
        let ft = self.expr(function, false);
        let arg_tys: Vec<Ty> = args.iter().map(|a| self.expr(&a.value, false)).collect();
        let ret = self.fresh();
        // A command called from client code is sent: you get its result later.
        let is_command = matches!(&function.kind, ExprKind::Name(n) if self.lookup(n).is_none()
            && matches!(self.g.values.get(n), Some(Def { kind: DefKind::Command { .. }, .. })));
        let ft = match self.prune(&ft) {
            Ty::Fn(p, r) => {
                let r2 = self.open_outputs(&r);
                Ty::Fn(p, Box::new(r2))
            }
            other => other,
        };
        if let Ty::Fn(p, _) = self.prune(&ft) {
            if p.len() != arg_tys.len() {
                self.report(e.span, format!("this takes {} argument{}, but {} {} given", p.len(), if p.len() == 1 { "" } else { "s" }, arg_tys.len(), if arg_tys.len() == 1 { "is" } else { "are" }));
                return ret;
            }
            for (i, (pt, at)) in p.iter().zip(&arg_tys).enumerate() {
                if let Err(x) = self.unify(at, pt) {
                    let what = match &function.kind {
                        ExprKind::Name(n) => format!("`{n}`"),
                        _ => "this function".into(),
                    };
                    self.report(args[i].value.span, format!("argument {} of {what} should be {}, but it's {}: {x}", i + 1, self.show(pt), self.show(at)));
                }
            }
        }
        if let Err(x) = self.unify(&ft, &func(arg_tys, ret.clone())) {
            self.report(function.span, format!("this can't be called like that: {x}"));
        }
        if is_command && (self.client || (!self.strict && !self.server)) {
            con1("Pending", ret)
        } else {
            ret
        }
    }

    /// `Some(x)`, a component, a route, or a configuration constructor.
    fn tag_call(&mut self, t: &str, args: &'a [Arg], span: Span) -> Ty {
        if let Some(c) = self.components.get(t) {
            let params = c.params.clone();
            for a in args {
                let at = self.expr(&a.value, false);
                match &a.name {
                    Some(n) => match params.iter().find(|(p, _, _)| *p == n.name) {
                        Some((_, pty, _)) => {
                            let want = self.from_ast(pty, &mut HashMap::new());
                            if let Err(x) = self.unify(&at, &want) {
                                self.report(a.value.span, format!("`{t}`'s `{}` is a {}: {x}", n.name, self.show(&want)));
                            }
                        }
                        None => self.report(n.span, format!("`{t}` has no parameter `{}`", n.name)),
                    },
                    None => self.report(a.value.span, format!("`{t}`'s parameters are named: write `{t}(name: value)`")),
                }
            }
            let missing: Vec<String> = params
                .iter()
                .filter(|(p, _, d)| !d && !args.iter().any(|a| a.name.as_ref().is_some_and(|n| n.name == *p)))
                .map(|(p, _, _)| format!("`{p}`"))
                .collect();
            if !missing.is_empty() {
                self.report(span, format!("`{t}` needs {}", missing.join(", ")));
            }
            return html();
        }
        match t {
            "Query" | "Command" | "Check" | "Table" | "Job" | "Endpoint" | "Machine" | "Deployment" | "Simulation"
            | "Network" | "Crashes" | "Outside" | "Fallible" | "Constant" | "Script" | "Fixed" => {
                for a in args {
                    match &a.value.kind {
                        ExprKind::Lambda(l) => {
                            self.lambda(l, None);
                        }
                        ExprKind::Tag(_) => {}
                        _ => {
                            self.expr(&a.value, false);
                        }
                    }
                }
                return con(t);
            }
            _ => {}
        }
        let payload: Vec<Ty> = args.iter().map(|a| self.expr(&a.value, false)).collect();
        let tail = self.fresh_var();
        Ty::Tags([(t.to_string(), payload)].into_iter().collect(), Some(tail))
    }

    fn builtin_call(&mut self, n: &str, args: &'a [Arg]) -> Option<Ty> {
        if self.g.values.contains_key(n) {
            return None;
        }
        let arg_tys: Vec<Ty> = match n {
            "now" | "today" | "new_id" | "random" | "sleep" | "go" | "status" | "all" | "race" | "daily" | "weekly"
            | "cron" => args.iter().map(|a| self.expr(&a.value, false)).collect(),
            _ => return None,
        };
        Some(match n {
            "now" => con("Time"),
            "today" => con("Date"),
            "new_id" => {
                let t = self.fresh();
                con1("Id", t)
            }
            "random" => self.fresh(),
            "sleep" => con1("Pending", unit()),
            "go" => unit(),
            "status" => {
                let a = arg_tys.first().cloned().unwrap_or_else(|| self.fresh());
                tags_closed(&[("Failed", vec![con("Text")]), ("Loading", vec![]), ("Ready", vec![a.clone()]), ("Refreshing", vec![a])])
            }
            "all" => {
                let a = self.fresh();
                if let Some(t) = arg_tys.first() {
                    let _ = self.unify(t, &con1("List", con1("Pending", a.clone())));
                }
                con1("Pending", con1("List", a))
            }
            "race" => {
                let a = self.fresh();
                if let Some(t) = arg_tys.first() {
                    let _ = self.unify(t, &con1("List", con1("Pending", a.clone())));
                }
                con1("Pending", a)
            }
            _ => con("Schedule"),
        })
    }

    fn field(&mut self, t: &Ty, name: &str, span: Span) -> Ty {
        match self.prune(t) {
            Ty::Con(n, args) if args.is_empty() && self.records.contains_key(&n) => match self.record_field(&n, name) {
                Some(ft) => ft,
                None => {
                    self.report(span, format!("`{n}` has no field `{name}`"));
                    self.fresh()
                }
            },
            Ty::Con(n, _) if n == "Response" && name == "status" => self.fresh(),
            Ty::Rec(fields, tail) => match fields.get(name) {
                Some(ft) => ft.clone(),
                None if tail.is_some() => {
                    let v = self.fresh();
                    let tail2 = self.fresh_var();
                    let want = Ty::Rec([(name.to_string(), v.clone())].into_iter().collect(), Some(tail2));
                    let cur = Ty::Rec(fields, tail);
                    let _ = self.unify(&cur, &want);
                    v
                }
                None => {
                    self.report(span, format!("this record has no field `{name}`"));
                    self.fresh()
                }
            },
            Ty::Var(_) => {
                let v = self.fresh();
                let tail = self.fresh_var();
                let _ = self.unify(t, &Ty::Rec([(name.to_string(), v.clone())].into_iter().collect(), Some(tail)));
                v
            }
            Ty::Tags(tags, _) if self.strict || tags.contains_key("Ok") || tags.contains_key("Some") => {
                self.report(span, format!("{} has no field `{name}`: match on it first", self.show(t)));
                self.fresh()
            }
            _ => self.fresh(),
        }
    }

    fn record_field(&mut self, rec: &str, field: &str) -> Option<Ty> {
        let fty = self.records.get(rec)?.fields.iter().find(|(n, _, _)| n == field)?.1.clone();
        Some(self.from_ast(&fty, &mut HashMap::new()))
    }

    // ── Methods ───────────────────────────────────────────────

    fn method_call(&mut self, e: &'a Expr, receiver: &'a Expr, m: &str, mspan: Span, args: &'a [Arg]) -> Ty {
        // Platform modules.
        if let ExprKind::Name(r) = &receiver.kind {
            if crate::builtins::MODULES.contains(&r.as_str()) && self.lookup(r).is_none() {
                for a in args {
                    self.expr(&a.value, false);
                }
                return self.module_call(r, m);
            }
        }
        if let ExprKind::Tag(_) = &receiver.kind {
            // `Dict.empty()`, `Money.zero(Usd)`: static calls.
            for a in args {
                self.expr(&a.value, false);
            }
            return self.fresh();
        }
        let rt = self.expr(receiver, false);
        let sig = self.method_sig(&rt, m, args).map(|t| match t {
            Ty::Fn(p, r) => {
                let r = self.open_outputs(&r);
                Ty::Fn(p, Box::new(r))
            }
            other => other,
        });
        match sig {
            Some(Ty::Fn(params, ret)) => {
                if params.len() != args.len() {
                    self.report(e.span, format!("`{m}` takes {} argument{}, but {} {} given", params.len(), if params.len() == 1 { "" } else { "s" }, args.len(), if args.len() == 1 { "is" } else { "are" }));
                    return *ret;
                }
                for (a, p) in args.iter().zip(&params) {
                    let at = match (&a.value.kind, self.prune(p)) {
                        (ExprKind::Lambda(l), Ty::Fn(..)) => self.lambda(l, Some(p.clone())),
                        _ => self.expr(&a.value, false),
                    };
                    if let Err(x) = self.unify(&at, p) {
                        self.report(a.value.span, format!("`{m}` needs {} here, but this is {}: {x}", self.show(p), self.show(&at)));
                    }
                }
                *ret
            }
            _ => {
                if self.strict {
                    if let Some(tn) = self.known_head(&rt) {
                        self.report(mspan, format!("`{tn}` has no method `{m}`"));
                    }
                }
                for a in args {
                    self.expr(&a.value, false);
                }
                self.fresh()
            }
        }
    }

    /// The type's name, when it's one the checker knows all methods of.
    fn known_head(&self, t: &Ty) -> Option<String> {
        match self.prune(t) {
            Ty::Con(n, _) if ["Text", "List", "Table", "Id", "Time", "Date", "Duration", "Set", "Dict"].contains(&n.as_str()) => Some(n),
            _ => None,
        }
    }

    fn module_call(&mut self, module: &str, f: &str) -> Ty {
        let http_err = tags_closed(&[("HttpError", vec![con("Text")])]);
        let any = self.fresh();
        let t = match (module, f) {
            ("http", _) => con1("Pending", result(con("Response"), http_err)),
            ("ws", "connect") => con1("Pending", con("Socket")),
            ("browser", "online") | ("browser", "focused") => bool_ty(),
            ("browser", "location") => con("Text"),
            ("secrets", "get") => con("Secret"),
            _ => return self.fresh(),
        };
        let _ = any;
        // The module function takes whatever arguments it takes; its result
        // is what matters here.
        let p = self.fresh();
        let _ = p;
        t
    }

    /// A method's type, without the receiver.
    fn method_sig(&mut self, recv: &Ty, m: &str, args: &[Arg]) -> Option<Ty> {
        let r = self.prune(recv);
        let a = |cx: &mut Self| cx.fresh();
        let text = con("Text");
        let int = || con("Int");
        // Methods every value has.
        if m == "to_text" {
            return Some(func(vec![], text));
        }
        match &r {
            Ty::Con(n, args0) if n == "Text" => {
                let _ = args0;
                Some(match m {
                    "trim" | "trim_start" | "trim_end" | "lower" | "upper" => func(vec![], text),
                    "is_empty" => func(vec![], bool_ty()),
                    "length" => func(vec![], int()),
                    "contains" | "starts_with" | "ends_with" => func(vec![con("Text")], bool_ty()),
                    "split" => func(vec![con("Text")], con1("List", con("Text"))),
                    "replace" => func(vec![con("Text"), con("Text")], con("Text")),
                    "concat" => func(vec![con("Text")], con("Text")),
                    "slice" => func(vec![con1("Range", int())], con("Text")),
                    "to_int" => func(vec![], result(int(), tags_closed(&[("NotANumber", vec![])]))),
                    "to_dec" => func(vec![], result(con("Dec"), tags_closed(&[("NotANumber", vec![])]))),
                    "to_bytes" => func(vec![], con("Bytes")),
                    _ => return None,
                })
            }
            Ty::Con(n, targs) if (n == "List" || n == "Table" || n == "Set") && targs.len() == 1 => {
                let el = targs[0].clone();
                let list = |t: Ty| con1("List", t);
                let pred = func(vec![el.clone()], bool_ty());
                if n == "Table" {
                    match m {
                        "get" => return Some(func(vec![con1("Id", el.clone())], option(el))),
                        "has" => return Some(func(vec![con1("Id", el.clone())], bool_ty())),
                        "rows" => return Some(func(vec![], list(el))),
                        "insert" => return Some(func(vec![el.clone()], con1("Id", el))),
                        "update" => return Some(func(vec![con1("Id", el.clone()), func(vec![el.clone()], el)], unit())),
                        "delete" => return Some(func(vec![con1("Id", el)], unit())),
                        "update_where" => return Some(func(vec![pred, func(vec![el.clone()], el)], unit())),
                        "delete_where" => return Some(func(vec![pred], unit())),
                        _ => {}
                    }
                }
                Some(match m {
                    "count" => func(vec![], int()),
                    "count_if" => func(vec![pred], int()),
                    "is_empty" => func(vec![], bool_ty()),
                    "get" => func(vec![int()], option(el)),
                    "first" | "last" => func(vec![], option(el)),
                    "find" => func(vec![pred], option(el)),
                    "contains" | "has" => func(vec![el], bool_ty()),
                    "any" | "all" => func(vec![pred], bool_ty()),
                    "filter" => func(vec![pred], list(el)),
                    "map" => {
                        let b = a(self);
                        func(vec![func(vec![el], b.clone())], list(b))
                    }
                    "flat_map" => {
                        let b = a(self);
                        func(vec![func(vec![el], list(b.clone()))], list(b))
                    }
                    "fold" => {
                        let b = a(self);
                        func(vec![b.clone(), func(vec![b.clone(), el], b.clone())], b)
                    }
                    "sum" => func(vec![], el),
                    "min" | "max" => func(vec![], option(el)),
                    "sort" | "reverse" | "distinct" => func(vec![], list(el)),
                    "sort_by" | "distinct_by" => {
                        let k = a(self);
                        func(vec![func(vec![el.clone()], k)], list(el))
                    }
                    "group_by" => {
                        let k = a(self);
                        let group = Ty::Rec([("key".to_string(), k.clone()), ("items".to_string(), list(el.clone()))].into_iter().collect(), None);
                        func(vec![func(vec![el], k)], list(group))
                    }
                    "take" | "skip" => func(vec![int()], list(el)),
                    "append" | "prepend" => func(vec![el.clone()], list(el)),
                    "concat" => func(vec![list(el.clone())], list(el)),
                    "insert" | "remove" if n == "Set" => func(vec![el.clone()], con1("Set", el)),
                    "to_list" => func(vec![], list(el)),
                    _ => return None,
                })
            }
            Ty::Tags(tags, None) if tags.contains_key("None") && tags.contains_key("Some") && tags.len() == 2 => {
                let el = tags["Some"][0].clone();
                Some(match m {
                    "with_default" => func(vec![el.clone()], el),
                    "map" => {
                        let b = a(self);
                        func(vec![func(vec![el], b.clone())], option(b))
                    }
                    "ok_or" => {
                        let e = a(self);
                        func(vec![e.clone()], result(el, e))
                    }
                    "is_some" | "is_none" => func(vec![], bool_ty()),
                    _ => return None,
                })
            }
            Ty::Tags(tags, None) if tags.contains_key("Ok") && tags.contains_key("Err") && tags.len() == 2 => {
                let o = tags["Ok"][0].clone();
                let e = tags["Err"][0].clone();
                Some(match m {
                    "with_default" => func(vec![o.clone()], o),
                    "map" => {
                        let b = a(self);
                        func(vec![func(vec![o], b.clone())], result(b, e))
                    }
                    "map_err" => {
                        let f = a(self);
                        func(vec![func(vec![e], f.clone())], result(o, f))
                    }
                    "on_ok" => {
                        let x = a(self);
                        func(vec![func(vec![o], x)], unit())
                    }
                    "on_err" => {
                        let x = a(self);
                        func(vec![func(vec![e], x)], unit())
                    }
                    "is_ok" | "is_err" => func(vec![], bool_ty()),
                    _ => return None,
                })
            }
            Ty::Con(n, _) if n == "Id" => match m {
                _ => None,
            },
            Ty::Con(n, _) if n == "Time" => match m {
                "date" => Some(func(vec![], con("Date"))),
                _ => None,
            },
            Ty::Con(n, _) if n == "Date" => match m {
                "day" | "month" | "year" => Some(func(vec![], int())),
                _ => None,
            },
            Ty::Con(n, _) if n == "Socket" => Some(match m {
                "send" => {
                    let x = a(self);
                    func(vec![x], unit())
                }
                "receive" => con1("Pending", result(con("Message"), tags_closed(&[("Closed", vec![])]))).pipe(|t| func(vec![], t)),
                "close" => func(vec![], unit()),
                _ => return None,
            }),
            Ty::Con(n, _) if n == "Response" || n == "Message" => Some(match m {
                "decode" => {
                    // `.decode(Quote)`: the argument names the type.
                    let target = match args.first().map(|a| &a.value.kind) {
                        Some(ExprKind::Tag(t)) if self.records.contains_key(t) => con(t),
                        _ => a(self),
                    };
                    let p = a(self);
                    func(vec![p], result(target, tags_closed(&[("BadData", vec![con("Text")])])))
                }
                "text" => func(vec![], con("Text")),
                _ => return None,
            }),
            Ty::Con(n, _) if n == "Element" => Some(match m {
                "focus" | "blur" | "scroll_into_view" => func(vec![], unit()),
                _ => return None,
            }),
            Ty::Con(n, _) if self.records.contains_key(n) => self.record_method(n, m),
            _ => None,
        }
    }

    /// A method defined in a record: its type without the receiver.
    fn record_method(&mut self, rec: &str, m: &str) -> Option<Ty> {
        let key = format!("{rec}.{m}");
        if !self.schemes.contains_key(&key) {
            let lambda = self.sources.iter().enumerate().find_map(|(fi, s)| {
                s.file.items.iter().find_map(|i| match &i.kind {
                    ItemKind::Record(r) if r.name.name == rec => r.methods.iter().find_map(|b| match (&b.left, &b.value.kind) {
                        (Pattern::Name(n), ExprKind::Lambda(l)) if n.name == m => Some((fi, l)),
                        _ => None,
                    }),
                    _ => None,
                })
            })?;
            let saved = (self.file, std::mem::take(&mut self.scopes), std::mem::take(&mut self.ret));
            self.file = lambda.0;
            let t = self.lambda(lambda.1, None);
            (self.file, self.scopes, self.ret) = saved;
            let s = self.generalize(&t);
            self.schemes.insert(key.clone(), s);
        }
        let s = self.schemes.get(&key)?;
        let s = Scheme { vars: s.vars.clone(), ty: s.ty.clone() };
        match self.instantiate(&s) {
            Ty::Fn(mut p, r) if !p.is_empty() => {
                let first = p.remove(0);
                let _ = self.unify(&first, &con(rec));
                Some(Ty::Fn(p, r))
            }
            _ => None,
        }
    }
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl Pipe for Ty {}

/// Whether an expression waits with `.await` itself (not inside a lambda).
fn contains_await(e: &Expr) -> bool {
    fn stmt(s: &Stmt) -> bool {
        match &s.kind {
            StmtKind::Binding(b) => contains_await(&b.value),
            StmtKind::Let { value, .. } | StmtKind::State { value, .. } => contains_await(value),
            StmtKind::For { iterable, body, .. } => contains_await(iterable) || block(body),
            StmtKind::While { condition, body } => contains_await(condition) || block(body),
            StmtKind::Return(Some(e)) | StmtKind::Expr(e) => contains_await(e),
            StmtKind::With { handler, body } => contains_await(handler) || block(body),
            StmtKind::Defer(b) => block(b),
            _ => false,
        }
    }
    fn block(b: &Block) -> bool {
        b.stmts.iter().any(stmt)
    }
    match &e.kind {
        ExprKind::Await(_) => true,
        ExprKind::Lambda(_) => false,
        ExprKind::Block(b) => block(b),
        ExprKind::If { condition, then, otherwise, .. } => {
            contains_await(condition) || block(then) || otherwise.as_ref().is_some_and(|o| contains_await(o))
        }
        ExprKind::Match { value, arms } => contains_await(value) || arms.iter().any(|a| contains_await(&a.value)),
        ExprKind::Call { function, args } => contains_await(function) || args.iter().any(|a| contains_await(&a.value)),
        ExprKind::Field { value, .. } | ExprKind::Try(value) | ExprKind::Crash(value) => contains_await(value),
        ExprKind::Unary { operand, .. } => contains_await(operand),
        ExprKind::Binary { left, right, .. } => contains_await(left) || contains_await(right),
        ExprKind::Assign { value, .. } => contains_await(value),
        ExprKind::Element { .. } => false,
        _ => false,
    }
}

fn last_expr_span(e: &Expr) -> Span {
    match &e.kind {
        ExprKind::Block(b) => b.stmts.last().map(|s| s.span).unwrap_or(e.span),
        _ => e.span,
    }
}
