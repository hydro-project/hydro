#[rust_sitter::grammar("datalog")]
#[allow(dead_code)]
pub mod datalog {
    use rust_sitter::Spanned;

    #[rust_sitter::language]
    #[derive(Debug)]
    pub struct Program {
        pub rules: Vec<Declaration>,
    }

    #[derive(Debug, Clone)]
    pub enum Declaration {
        Input(
            #[rust_sitter::leaf(text = ".input")] (),
            Spanned<Ident>,
            RustSnippet,
        ),
        Output(
            #[rust_sitter::leaf(text = ".output")] (),
            Spanned<Ident>,
            RustSnippet,
        ),
        Async(
            #[rust_sitter::leaf(text = ".async")] (),
            Spanned<Ident>,
            /// A pipeline for data to be sent to another node, which must consume `(NodeID, Data)` pairs.
            RustSnippet,
            /// A pipeline for data received from another node, which must produce `Data` values.
            RustSnippet,
        ),
        Rule(Spanned<Rule>),
    }

    #[derive(Debug, Clone)]
    pub struct RustSnippet {
        #[rust_sitter::leaf(text = "`")]
        _start: (),
        #[rust_sitter::leaf(pattern = r#"[^`]*"#, transform = |s| s.to_string())]
        pub code: Spanned<String>,
        #[rust_sitter::leaf(text = "`")]
        _end: (),
    }

    #[derive(Debug, Clone)]
    pub struct Rule {
        pub target: TargetRelationExpr,

        pub rule_type: Spanned<RuleType>,

        #[rust_sitter::repeat(non_empty = true)]
        #[rust_sitter::delimited(
            #[rust_sitter::leaf(text = ",")]
            ()
        )]
        pub sources: Vec<Atom>,

        #[rust_sitter::leaf(text = ".")]
        _dot: Option<()>,
    }

    #[derive(Debug, Clone)]
    pub enum RuleType {
        Sync(#[rust_sitter::leaf(text = ":-")] ()),
        NextTick(#[rust_sitter::leaf(text = ":+")] ()),
        Async(#[rust_sitter::leaf(text = ":~")] ()),
    }

    #[derive(Debug, Clone)]
    pub enum Atom {
        Relation(
            #[rust_sitter::leaf(text = "!")] Option<()>,
            Spanned<InputRelationExpr>,
        ),
        Predicate(Spanned<BoolExpr>),
    }

    #[derive(Debug, Clone)]
    pub enum IdentOrUnderscore {
        Ident(Spanned<Ident>),
        Underscore(#[rust_sitter::leaf(text = "_")] Spanned<()>),
    }

    #[derive(Debug, Clone)]
    pub struct InputRelationExpr {
        pub name: Spanned<Ident>,

        #[rust_sitter::leaf(text = "(")]
        _l_paren: (),

        #[rust_sitter::delimited(
            #[rust_sitter::leaf(text = ",")]
            ()
        )]
        pub fields: Vec<Spanned<IdentOrUnderscore>>,

        #[rust_sitter::leaf(text = ")")]
        _r_paren: (),
    }

    #[derive(Debug, Clone)]
    pub struct TargetRelationExpr {
        pub name: Spanned<Ident>,

        pub at_node: Option<AtNode>,

        #[rust_sitter::leaf(text = "(")]
        _l_paren: (),

        #[rust_sitter::delimited(
            #[rust_sitter::leaf(text = ",")]
            ()
        )]
        pub fields: Vec<Spanned<TargetExpr>>,

        #[rust_sitter::leaf(text = ")")]
        _r_paren: (),
    }

    #[derive(Debug, Clone)]
    #[allow(clippy::manual_non_exhaustive)]
    pub struct AtNode {
        #[rust_sitter::leaf(text = "@")]
        _at: (),

        pub node: Spanned<TargetExpr>,
    }

    #[derive(Debug, Clone)]
    pub enum TargetExpr {
        Expr(IntExpr),
        Aggregation(Aggregation),
    }

    impl TargetExpr {
        pub fn idents(&self) -> Vec<&Ident> {
            match self {
                TargetExpr::Expr(e) => e.idents(),
                TargetExpr::Aggregation(a) => vec![&a.ident],
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Aggregation {
        pub tpe: AggregationType,
        #[rust_sitter::leaf(text = "(")]
        _lparen: (),
        pub ident: Spanned<Ident>,
        #[rust_sitter::leaf(text = ")")]
        _rparen: (),
    }

    #[derive(Debug, Clone)]
    pub enum AggregationType {
        Min(#[rust_sitter::leaf(text = "min")] ()),
        Max(#[rust_sitter::leaf(text = "max")] ()),
        Sum(#[rust_sitter::leaf(text = "sum")] ()),
        Count(#[rust_sitter::leaf(text = "count")] ()),
        Choose(#[rust_sitter::leaf(text = "choose")] ()),
    }

    #[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Debug)]
    pub struct Ident {
        #[rust_sitter::leaf(pattern = r"[a-zA-Z][a-zA-Z0-9_]*", transform = |s| s.to_string())]
        pub name: String,
    }

    #[rust_sitter::extra]
    struct Whitespace {
        #[rust_sitter::leaf(pattern = r"\s")]
        _whitespace: (),
    }

    #[rust_sitter::extra]
    struct Comment {
        #[rust_sitter::leaf(pattern = r"(#|\/\/).*")]
        _comment: (),
    }

    #[derive(Debug, Clone)]
    pub enum BoolOp {
        Lt(#[rust_sitter::leaf(text = "<")] ()),
        LtEq(#[rust_sitter::leaf(text = "<=")] ()),
        Gt(#[rust_sitter::leaf(text = ">")] ()),
        GtEq(#[rust_sitter::leaf(text = ">=")] ()),
        Eq(#[rust_sitter::leaf(text = "==")] ()),
        Neq(#[rust_sitter::leaf(text = "!=")] ()),
    }

    #[derive(Debug, Clone)]
    pub struct BoolExpr {
        #[rust_sitter::leaf(text = "(")]
        _l_brace: (),

        pub left: Spanned<IntExpr>,
        pub op: BoolOp,
        pub right: Spanned<IntExpr>,

        #[rust_sitter::leaf(text = ")")]
        _r_brace: (),
    }

    #[derive(Debug, Clone)]
    pub enum IntExpr {
        Ident(Spanned<Ident>),
        Integer(
            #[rust_sitter::leaf(pattern = r"[0-9]+", transform = |s| s.parse().unwrap())]
            Spanned<i64>,
        ),
        #[rust_sitter::prec_left(1)]
        Add(
            Box<IntExpr>,
            #[rust_sitter::leaf(text = "+")] (),
            Box<IntExpr>,
        ),
        #[rust_sitter::prec_left(1)]
        Sub(
            Box<IntExpr>,
            #[rust_sitter::leaf(text = "-")] (),
            Box<IntExpr>,
        ),
    }

    impl IntExpr {
        pub fn idents(&self) -> Vec<&Ident> {
            match self {
                IntExpr::Ident(i) => vec![i],
                IntExpr::Integer(_) => vec![],
                IntExpr::Add(l, _, r) => {
                    let mut idents = l.idents();
                    idents.extend(r.idents());
                    idents
                }
                IntExpr::Sub(l, _, r) => {
                    let mut idents = l.idents();
                    idents.extend(r.idents());
                    idents
                }
            }
        }
    }
}
