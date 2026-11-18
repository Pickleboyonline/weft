//! The typed syntax tree. Every node has a [`Span`] pointing back at source.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    /// 0-based line of `start`.
    pub line: usize,
    /// 0-based column (bytes) of `start`.
    pub col: usize,
}

#[derive(Clone, Debug)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct File {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ItemKind {
    Record(Record),
    Component(Component),
    Effect(EffectDecl),
    Handler(HandlerDecl),
    Signature(Signature),
    Alias(Alias),
    Binding(Binding),
    Expect(Expr),
}

#[derive(Clone, Debug)]
pub struct Record {
    pub name: Ident,
    pub fields: Vec<RecordField>,
    pub methods: Vec<Binding>,
}

#[derive(Clone, Debug)]
pub struct RecordField {
    pub key: bool,
    pub name: Ident,
    pub ty: Type,
    pub default: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub name: Ident,
    pub params: Vec<ComponentParam>,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub struct ComponentParam {
    pub name: Ident,
    pub ty: Type,
    pub default: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    Resume,
    Final,
    Control,
}

#[derive(Clone, Debug)]
pub struct EffectDecl {
    pub name: Ident,
    pub ops: Vec<Operation>,
}

#[derive(Clone, Debug)]
pub struct Operation {
    pub kind: OpKind,
    pub name: Ident,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct HandlerDecl {
    pub name: Ident,
    pub effect: Ident,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub struct Signature {
    pub name: Ident,
    pub ty: Type,
    pub constraints: Vec<Constraint>,
}

#[derive(Clone, Debug)]
pub struct Constraint {
    pub var: Ident,
    pub method: Ident,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct Alias {
    pub name: Ident,
    pub params: Vec<Ident>,
    pub ty: Type,
}

/// `name = value`: a definition, a member, or (in a block) an assignment.
#[derive(Clone, Debug)]
pub struct Binding {
    pub left: Pattern,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    /// `name = value` in a block: an assignment to a `let mut` or `state`,
    /// or a component member.
    Binding(Binding),
    /// `let pattern = value`, `let mut name = value`, or
    /// `let pattern = value else { … }`.
    Let { pattern: Pattern, mutable: bool, value: Expr, otherwise: Option<Block> },
    State { name: Ident, ty: Option<Type>, value: Expr },
    Style,
    For { pattern: Pattern, iterable: Expr, body: Block },
    While { condition: Expr, body: Block },
    Break,
    Continue,
    Return(Option<Expr>),
    With { handler: Expr, body: Block },
    Defer(Block),
    Expr(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberKind {
    Int,
    Dec,
    Float,
    Duration,
    Size,
    Percent,
}

#[derive(Clone, Debug)]
pub enum TextPart {
    Literal(String),
    Interpolation(Expr),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    RangeExclusive,
    RangeInclusive,
}

impl BinOp {
    pub fn is_comparison(self) -> bool {
        matches!(self, BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Name(String),
    Tag(String),
    Number { text: String, kind: NumberKind },
    Text(Vec<TextPart>),
    List(Vec<Expr>),
    Record(Vec<Field>),
    TypedRecord { ty: Ident, fields: Vec<Field> },
    Block(Block),
    Lambda(Lambda),
    /// `if condition { … }`, or `if let pattern = condition { … }` when
    /// `pattern` is set.
    If { pattern: Option<Box<Pattern>>, condition: Box<Expr>, then: Block, otherwise: Option<Box<Expr>> },
    Match { value: Box<Expr>, arms: Vec<Arm> },
    /// `.field`: a function from a record to its field.
    Accessor(Ident),
    Unit,
    Call { function: Box<Expr>, args: Vec<Arg> },
    /// `name(args) { children }`: an element, or a `task`.
    Element { head: Box<Expr>, children: Block },
    Field { value: Box<Expr>, field: Ident },
    Await(Box<Expr>),
    Try(Box<Expr>),
    Unary { op: UnOp, operand: Box<Expr> },
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Crash(Box<Expr>),
    /// An assignment in expression position: a lambda body, argument or arm.
    Assign { name: Ident, value: Box<Expr> },
    Error,
}

#[derive(Clone, Debug)]
pub enum Field {
    Named { name: Ident, value: Expr },
    Shorthand(Ident),
    Spread(Expr),
}

#[derive(Clone, Debug)]
pub struct Arg {
    pub name: Option<Ident>,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct Lambda {
    pub params: Vec<LambdaParam>,
    pub result: Option<Type>,
    pub body: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct LambdaParam {
    pub pattern: Pattern,
    pub ty: Option<Type>,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Name(Ident),
    Wildcard(Span),
    Tag { name: Ident, args: Vec<Pattern> },
    Record { fields: Vec<(Ident, Option<Pattern>)>, span: Span },
    List { items: Vec<Pattern>, rest: Option<Option<Ident>>, span: Span },
    /// `Active | Done`, in a `match` arm.
    Or(Vec<Pattern>, Span),
    Literal(Expr),
    Error(Span),
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Name(i) => i.span,
            Pattern::Wildcard(s) | Pattern::Error(s) => *s,
            Pattern::Tag { name, .. } => name.span,
            Pattern::Record { span, .. } | Pattern::List { span, .. } | Pattern::Or(_, span) => *span,
            Pattern::Literal(e) => e.span,
        }
    }

    /// Every name the pattern binds.
    pub fn names(&self) -> Vec<&Ident> {
        let mut out = Vec::new();
        self.collect_names(&mut out);
        out
    }

    fn collect_names<'a>(&'a self, out: &mut Vec<&'a Ident>) {
        match self {
            Pattern::Name(i) => out.push(i),
            Pattern::Tag { args, .. } => args.iter().for_each(|p| p.collect_names(out)),
            Pattern::Or(alts, _) => {
                if let Some(first) = alts.first() {
                    first.collect_names(out);
                }
            }
            Pattern::Record { fields, .. } => {
                for (name, sub) in fields {
                    match sub {
                        Some(p) => p.collect_names(out),
                        None => out.push(name),
                    }
                }
            }
            Pattern::List { items, rest, .. } => {
                items.iter().for_each(|p| p.collect_names(out));
                if let Some(Some(r)) = rest {
                    out.push(r);
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub enum Type {
    /// `Int`, `List(Item)`, `Id(TodoList)`.
    Name { name: Ident, args: Vec<Type> },
    /// A lowercase type variable: `a`.
    Var(Ident),
    Fn { params: Vec<Type>, result: Box<Type>, span: Span },
    Record { fields: Vec<(Ident, Type)>, span: Span },
    Tags { tags: Vec<(Ident, Vec<Type>)>, open: bool, span: Span },
    Unit(Span),
    Error(Span),
}

impl Type {
    pub fn span(&self) -> Span {
        match self {
            Type::Name { name, .. } => name.span,
            Type::Var(i) => i.span,
            Type::Fn { span, .. } | Type::Record { span, .. } | Type::Tags { span, .. } => *span,
            Type::Unit(s) | Type::Error(s) => *s,
        }
    }
}
