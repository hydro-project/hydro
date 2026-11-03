//! Graph representation stages for DFIR graphs.

use std::borrow::Cow;
use std::hash::Hash;

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ExprPath, GenericArgument, Token, Type};

use self::ops::{OperatorConstraints, Persistence};
use crate::diagnostic::{Diagnostic, Diagnostics, Level};
use crate::parse::{DfirCode, IndexInt, Operator, PortIndex, Ported, SingletonRef};
use crate::pretty_span::PrettySpan;

mod di_mul_graph;
mod eliminate_extra_unions_tees;
mod flat_graph_builder;
mod flat_to_partitioned;
mod graph_write;
mod meta_graph;
mod meta_graph_debugging;

use std::fmt::Display;

pub use di_mul_graph::DiMulGraph;
pub use eliminate_extra_unions_tees::eliminate_extra_unions_tees;
pub use flat_graph_builder::{FlatGraphBuilder, FlatGraphBuilderOutput};
pub use flat_to_partitioned::{PartitionError, partition_graph};
pub use meta_graph::{AsCodeOptions, DfirGraph, WriteConfig, WriteGraphType};

pub use crate::graph_ids::{GraphEdgeId, GraphLoopId, GraphNodeId, GraphSubgraphId};

pub mod graph_algorithms;
pub mod ops;

impl GraphSubgraphId {
    /// Generate a deterministic `Ident` for the given subgraph ID.
    pub fn as_ident(self, span: Span) -> Ident {
        use slotmap::Key;
        Ident::new(&format!("sgid_{:?}", self.data()), span)
    }
}

impl GraphLoopId {
    /// Generate a deterministic `Ident` for the given loop ID.
    pub fn as_ident(self, span: Span) -> Ident {
        use slotmap::Key;
        Ident::new(&format!("loop_{:?}", self.data()), span)
    }
}

/// Context identifier as a string.
const CONTEXT: &str = "context";
/// Runnable DFIR graph object identifier as a string.
const GRAPH: &str = "df";

const HANDOFF_NODE_STR: &str = "handoff";
const SINGLETON_SLOT_NODE_STR: &str = "singleton";
const MODULE_BOUNDARY_NODE_STR: &str = "module_boundary";

mod serde_syn {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: quote::ToTokens,
    {
        serializer.serialize_str(&value.to_token_stream().to_string())
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: syn::parse::Parse,
    {
        let s = String::deserialize(deserializer)?;
        syn::parse_str(&s).map_err(<D::Error as serde::de::Error>::custom)
    }
}

/// A variable name assigned to a pipeline in DFIR syntax.
///
/// Fundamentally a serializable/deserializable wrapper around [`syn::Ident`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Varname(#[serde(with = "serde_syn")] pub Ident);

/// The kind of inter-subgraph handoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandoffKind {
    /// A `Vec<T>` buffer for streams (zero or more items).
    Vec,
    /// An `Option<T>` slot for singletons (exactly one item expected).
    /// `#varname` gives `&T` (panics if empty).
    Singleton,
    /// An `Option<T>` slot for optionals (zero or one item).
    /// `#varname` gives `&Option<T>`.
    Optional,
}

/// A node, corresponding to an operator or a handoff.
#[derive(Clone, Serialize, Deserialize)]
pub enum GraphNode {
    /// An operator.
    Operator(#[serde(with = "serde_syn")] Operator),
    /// An inter-subgraph handoff point for buffering data between subgraphs.
    Handoff {
        /// What kind of storage this handoff uses.
        kind: HandoffKind,
        /// The span of the input into the handoff.
        #[serde(skip, default = "Span::call_site")]
        src_span: Span,
        /// The span of the output out of the handoff.
        #[serde(skip, default = "Span::call_site")]
        dst_span: Span,
    },

    /// Module Boundary, used for importing modules. Only exists prior to partitioning.
    ModuleBoundary {
        /// If this module is an input or output boundary.
        input: bool,

        /// The span of the import!() expression that imported this module.
        /// The value of this span when the `ModuleBoundary` node is still inside the module is `Span::call_site()`
        /// TODO: This could one day reference into the module file itself?
        #[serde(skip, default = "Span::call_site")]
        import_expr: Span,
    },
}
impl GraphNode {
    /// Return the node as a human-readable string.
    pub fn to_pretty_string(&self) -> Cow<'static, str> {
        match self {
            GraphNode::Operator(op) => op.to_pretty_string().into(),
            GraphNode::Handoff {
                kind: HandoffKind::Vec,
                ..
            } => HANDOFF_NODE_STR.into(),
            GraphNode::Handoff {
                kind: HandoffKind::Singleton | HandoffKind::Optional,
                ..
            } => SINGLETON_SLOT_NODE_STR.into(),
            GraphNode::ModuleBoundary { .. } => MODULE_BOUNDARY_NODE_STR.into(),
        }
    }

    /// Return the name of the node as a string, excluding parenthesis and op source code.
    pub fn to_name_string(&self) -> Cow<'static, str> {
        match self {
            GraphNode::Operator(op) => op.name_string().into(),
            GraphNode::Handoff {
                kind: HandoffKind::Vec,
                ..
            } => HANDOFF_NODE_STR.into(),
            GraphNode::Handoff {
                kind: HandoffKind::Singleton | HandoffKind::Optional,
                ..
            } => SINGLETON_SLOT_NODE_STR.into(),
            GraphNode::ModuleBoundary { .. } => MODULE_BOUNDARY_NODE_STR.into(),
        }
    }

    /// Return the source code span of the node.
    pub fn span(&self) -> Span {
        match self {
            Self::Operator(op) => op.span(),
            &Self::Handoff {
                src_span, dst_span, ..
            } => src_span.join(dst_span).unwrap_or(src_span),
            Self::ModuleBoundary { import_expr, .. } => *import_expr,
        }
    }
}
impl std::fmt::Debug for GraphNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operator(operator) => {
                write!(f, "Node::Operator({} span)", PrettySpan(operator.span()))
            }
            Self::Handoff { kind, .. } => write!(f, "Node::Handoff({kind:?})"),
            Self::ModuleBoundary { input, .. } => {
                write!(f, "Node::ModuleBoundary{{input: {}}}", input)
            }
        }
    }
}

/// Meta-data relating to operators which may be useful throughout the compilation process.
///
/// This data can be generated from the graph, but it is useful to have it readily available
/// pre-computed as many algorithms use the same info. Stuff like port names, arguments, and the
/// [`OperatorConstraints`] for the operator.
///
/// Because it is derived from the graph itself, there can be "cache invalidation"-esque issues
/// if this data is not kept in sync with the graph.
#[derive(Clone, Debug)]
pub struct OperatorInstance {
    /// Name of the operator (will match [`OperatorConstraints::name`]).
    pub op_constraints: &'static OperatorConstraints,
    /// Port values used as this operator's input.
    pub input_ports: Vec<PortIndexValue>,
    /// Port values used as this operator's output.
    pub output_ports: Vec<PortIndexValue>,
    /// Singleton references within the operator arguments.
    pub singletons_referenced: Vec<SingletonRef>,

    /// Generic arguments.
    pub generics: OpInstGenerics,
    /// Arguments provided by the user into the operator as arguments.
    /// I.e. the `a, b, c` in `-> my_op(a, b, c) -> `.
    ///
    /// These arguments do not include singleton postprocessing codegen. Instead use
    /// [`ops::WriteContextArgs::arguments`].
    pub arguments_pre: Punctuated<Expr, Token![,]>,
    /// Unparsed arguments, for singleton parsing.
    pub arguments_raw: TokenStream,
}

/// Operator generic arguments, split into specific categories.
#[derive(Clone, Debug)]
pub struct OpInstGenerics {
    /// Operator generic (type or lifetime) arguments.
    pub generic_args: Option<Punctuated<GenericArgument, Token![,]>>,
    /// Lifetime persistence arguments. Corresponds to a prefix of [`Self::generic_args`].
    pub persistence_args: Vec<Persistence>,
    /// Type persistence arguments. Corersponds to a (suffix) of [`Self::generic_args`].
    pub type_args: Vec<Type>,
}

impl OpInstGenerics {
    /// Helper to join a sequence of spans into a single span, if possible.
    ///
    /// Returns `None` if there are no spans or if any `Span::join` call fails
    /// (for example, when spans are not contiguous).
    fn join_spans<I>(mut spans: I) -> Option<Span>
    where
        I: Iterator<Item = Span>,
    {
        let mut span = spans.next()?;
        for s in spans {
            span = span.join(s)?;
        }
        Some(span)
    }

    /// Returns a [`Span`] containing all persistence (lifetime) args if possible.
    pub fn persistence_args_span(&self) -> Option<Span> {
        self.generic_args.as_ref().and_then(|args| {
            Self::join_spans(
                args.iter()
                    .filter(|a| matches!(a, GenericArgument::Lifetime(_)))
                    .map(|a| a.span()),
            )
        })
    }

    /// Returns a [`Span`] containing all type args if possible.
    pub fn type_args_span(&self) -> Option<Span> {
        self.generic_args.as_ref().and_then(|args| {
            Self::join_spans(
                args.iter()
                    .filter(|a| matches!(a, GenericArgument::Type(_)))
                    .map(|a| a.span()),
            )
        })
    }
}

/// Gets the generic arguments for the operator.
///
/// This helper method is useful due to the special handling of persistence lifetimes (`'static`,
/// `'tick`) which must come before other generic parameters.
pub fn get_operator_generics(diagnostics: &mut Diagnostics, operator: &Operator) -> OpInstGenerics {
    // Generic arguments.
    let generic_args = operator.type_arguments().cloned();
    // Leading lifetimes are the persistence prefix. A non-lifetime ends that prefix.
    // An unknown lifetime is reported and not stored, but does not end the prefix, so
    // following type arguments are still collected and checked.
    let mut persistence_args = Vec::new();
    let mut persistence_prefix_len = 0usize;
    for generic_arg in generic_args.iter().flatten() {
        let GenericArgument::Lifetime(lifetime) = generic_arg else {
            break;
        };
        persistence_prefix_len += 1;
        match &*lifetime.ident.to_string() {
            "tick" => persistence_args.push(Persistence::Tick),
            "static" => persistence_args.push(Persistence::Static),
            _ => {
                diagnostics.push(Diagnostic::spanned(
                    generic_arg.span(),
                    Level::Error,
                    format!(
                        "Unknown lifetime generic argument `'{}`, expected `'tick` or `'static`.",
                        lifetime.ident
                    ),
                ));
            }
        }
    }
    let type_args = generic_args
        .iter()
        .flatten()
        .skip(persistence_prefix_len)
        .map_while(|generic_arg| match generic_arg {
            GenericArgument::Type(typ) => Some(typ),
            _ => None,
        })
        .cloned()
        .collect::<Vec<_>>();

    OpInstGenerics {
        generic_args,
        persistence_args,
        type_args,
    }
}

/// Push, Pull, Comp, or Hoff polarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Color {
    /// Pull (green)
    Pull,
    /// Push (blue)
    Push,
    /// Computation (yellow)
    Comp,
    /// Handoff (grey) -- not a color for operators, inserted between subgraphs.
    Hoff,
}

/// Helper struct for [`PortIndex`] which keeps span information for elided ports.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PortIndexValue {
    /// An integer value: `[0]`, `[1]`, etc. Can be negative although we don't use that (2023-08-16).
    Int(#[serde(with = "serde_syn")] IndexInt),
    /// A name or path. `[pos]`, `[neg]`, etc. Can use `::` separators but we don't use that (2023-08-16).
    Path(#[serde(with = "serde_syn")] ExprPath),
    /// Elided, unspecified port. We have this variant, rather than wrapping in `Option`, in order
    /// to preserve the `Span` information.
    Elided(#[serde(skip)] Option<Span>),
}
impl PortIndexValue {
    /// For a [`Ported`] value like `[port_in]name[port_out]`, get the `port_in` and `port_out` as
    /// [`PortIndexValue`]s.
    pub fn from_ported<Inner>(ported: Ported<Inner>) -> (Self, Inner, Self)
    where
        Inner: Spanned,
    {
        let ported_span = Some(ported.inner.span());
        let port_inn = ported
            .inn
            .map(|idx| idx.index.into())
            .unwrap_or_else(|| Self::Elided(ported_span));
        let inner = ported.inner;
        let port_out = ported
            .out
            .map(|idx| idx.index.into())
            .unwrap_or_else(|| Self::Elided(ported_span));
        (port_inn, inner, port_out)
    }

    /// Returns `true` if `self` is not [`PortIndexValue::Elided`].
    pub fn is_specified(&self) -> bool {
        !matches!(self, Self::Elided(_))
    }

    /// Returns whichever of the two ports are specified.
    /// If both are [`Self::Elided`], returns [`Self::Elided`].
    /// If both are specified, returns `Err(self)`.
    #[allow(clippy::allow_attributes, reason = "Only triggered on nightly.")]
    #[allow(
        clippy::result_large_err,
        reason = "variants are same size, error isn't to be propagated."
    )]
    pub fn combine(self, other: Self) -> Result<Self, Self> {
        match (self.is_specified(), other.is_specified()) {
            (false, _other) => Ok(other),
            (true, false) => Ok(self),
            (true, true) => Err(self),
        }
    }

    /// Formats self as a human-readable string for error messages.
    pub fn as_error_message_string(&self) -> String {
        match self {
            PortIndexValue::Int(n) => format!("`{}`", n.value),
            PortIndexValue::Path(path) => format!("`{}`", path.to_token_stream()),
            PortIndexValue::Elided(_) => "<elided>".to_owned(),
        }
    }

    /// Returns the span of this port value.
    pub fn span(&self) -> Span {
        match self {
            PortIndexValue::Int(x) => x.span(),
            PortIndexValue::Path(x) => x.span(),
            PortIndexValue::Elided(span) => span.unwrap_or_else(Span::call_site),
        }
    }
}
impl From<PortIndex> for PortIndexValue {
    fn from(value: PortIndex) -> Self {
        match value {
            PortIndex::Int(x) => Self::Int(x),
            PortIndex::Path(x) => Self::Path(x),
        }
    }
}
impl PartialEq for PortIndexValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Path(l0), Self::Path(r0)) => l0 == r0,
            (Self::Elided(_), Self::Elided(_)) => true,
            _else => false,
        }
    }
}
impl Eq for PortIndexValue {}
impl PartialOrd for PortIndexValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PortIndexValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Int(s), Self::Int(o)) => s.cmp(o),
            (Self::Path(s), Self::Path(o)) => s
                .to_token_stream()
                .to_string()
                .cmp(&o.to_token_stream().to_string()),
            (Self::Elided(_), Self::Elided(_)) => std::cmp::Ordering::Equal,
            (Self::Int(_), Self::Path(_)) => std::cmp::Ordering::Less,
            (Self::Path(_), Self::Int(_)) => std::cmp::Ordering::Greater,
            (_, Self::Elided(_)) => std::cmp::Ordering::Less,
            (Self::Elided(_), _) => std::cmp::Ordering::Greater,
        }
    }
}

impl Display for PortIndexValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortIndexValue::Int(x) => write!(f, "{}", x.to_token_stream()),
            PortIndexValue::Path(x) => write!(f, "{}", x.to_token_stream()),
            PortIndexValue::Elided(_) => write!(f, "[]"),
        }
    }
}

/// Output of [`build_dfir_code`].
pub struct BuildDfirCodeOutput {
    /// The now-partitioned graph.
    pub partitioned_graph: DfirGraph,
    /// The Rust source code tokens for the DFIR.
    pub code: TokenStream,
    /// Any (non-error) diagnostics emitted.
    pub diagnostics: Diagnostics,
}

/// Compiles a [`DfirCode`] AST into inline source code that runs the dataflow.
pub fn build_dfir_code(
    dfir_code: DfirCode,
    root: &TokenStream,
) -> Result<BuildDfirCodeOutput, Diagnostics> {
    let flat_graph_builder = FlatGraphBuilder::from_dfir(dfir_code);

    let FlatGraphBuilderOutput {
        mut flat_graph,
        uses,
        mut diagnostics,
    } = flat_graph_builder.build()?;

    let () = match flat_graph.merge_modules() {
        Ok(()) => (),
        Err(d) => {
            diagnostics.push(d);
            return Err(diagnostics);
        }
    };

    eliminate_extra_unions_tees(&mut flat_graph);

    // Detect adjacent handoffs (e.g. `handoff() -> handoff()` or `singleton() -> singleton()`),
    // which can arise after unary tee/union elimination.
    for (edge_id, (src, dst)) in flat_graph.edges() {
        let _ = edge_id;
        if matches!(flat_graph.node(src), GraphNode::Handoff { .. })
            && matches!(flat_graph.node(dst), GraphNode::Handoff { .. })
        {
            let span = flat_graph.node(dst).span();
            diagnostics.push(Diagnostic::spanned(
                span,
                Level::Error,
                "Adjacent handoff/singleton operators are not allowed. \
                 Remove one or insert an operator between them.",
            ));
        }
    }

    if diagnostics.has_error() {
        return Err(diagnostics);
    }

    let partitioned_graph = match partition_graph(flat_graph) {
        Ok(partitioned_graph) => partitioned_graph,
        Err(err) => {
            diagnostics.push(err.diagnostic);
            return Err(diagnostics);
        }
    };

    let code = partitioned_graph.as_code_with_options(
        root,
        &AsCodeOptions {
            // Macro users (`dfir_syntax!`) have no way to configure codegen options, so keep
            // runtime metrics tracking enabled for them (`Dfir::metrics()` should always work).
            include_metrics_tracking: true,
            ..Default::default()
        },
        quote::quote! { #( #uses )* },
        &mut diagnostics,
    )?;

    Ok(BuildDfirCodeOutput {
        partitioned_graph,
        code,
        diagnostics,
    })
}

/// Changes all of token's spans to `span`, recursing into groups.
fn change_spans(tokens: TokenStream, span: Span) -> TokenStream {
    use proc_macro2::{Group, TokenTree};
    tokens
        .into_iter()
        .map(|token| match token {
            TokenTree::Group(mut group) => {
                group.set_span(span);
                TokenTree::Group(Group::new(
                    group.delimiter(),
                    change_spans(group.stream(), span),
                ))
            }
            TokenTree::Ident(mut ident) => {
                ident.set_span(span.resolved_at(ident.span()));
                TokenTree::Ident(ident)
            }
            TokenTree::Punct(mut punct) => {
                punct.set_span(span);
                TokenTree::Punct(punct)
            }
            TokenTree::Literal(mut literal) => {
                literal.set_span(span);
                TokenTree::Literal(literal)
            }
        })
        .collect()
}

#[cfg(test)]
mod test {
    use syn::parse_quote;

    use super::*;

    /// A bad persistence lifetime must be reported without dropping later type arguments.
    #[test]
    fn unknown_lifetime_does_not_drop_following_type_args() {
        let operator: Operator = parse_quote! {
            identity::<'a, usize>()
        };
        let mut diagnostics = Diagnostics::new();
        let generics = get_operator_generics(&mut diagnostics, &operator);

        assert!(
            diagnostics.iter().any(|diagnostic| {
                Level::Error == diagnostic.level
                    && diagnostic
                        .message
                        .contains("Unknown lifetime generic argument `'a`")
            }),
            "expected an unknown-lifetime diagnostic, got: {diagnostics:?}",
        );
        assert!(
            generics.persistence_args.is_empty(),
            "a bad lifetime must not be stored as Tick or Static, got: {:?}",
            generics.persistence_args,
        );
        assert_eq!(
            1,
            generics.type_args.len(),
            "type argument `usize` must still be collected, got: {:?}",
            generics
                .type_args
                .iter()
                .map(|ty| ty.to_token_stream().to_string())
                .collect::<Vec<_>>(),
        );
        assert_eq!("usize", generics.type_args[0].to_token_stream().to_string(),);
    }

    /// A type argument still ends the persistence prefix. Lifetimes after that are not
    /// persistence and are not reported as unknown persistence lifetimes.
    #[test]
    fn non_lifetime_ends_persistence_prefix() {
        let operator: Operator = parse_quote! {
            identity::<usize, 'a>()
        };
        let mut diagnostics = Diagnostics::new();
        let generics = get_operator_generics(&mut diagnostics, &operator);

        assert!(
            diagnostics.is_empty(),
            "a lifetime after a type is outside the persistence prefix, got: {diagnostics:?}",
        );
        assert!(generics.persistence_args.is_empty());
        assert_eq!("usize", generics.type_args[0].to_token_stream().to_string(),);
    }

    /// A bad lifetime between a real persistence lifetime and a type does not hide either.
    #[test]
    fn bad_lifetime_in_prefix_keeps_neighboring_generics() {
        let operator: Operator = parse_quote! {
            identity::<'tick, 'a, usize>()
        };
        let mut diagnostics = Diagnostics::new();
        let generics = get_operator_generics(&mut diagnostics, &operator);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("Unknown lifetime generic argument `'a`")
        }));
        assert_eq!(&[Persistence::Tick], generics.persistence_args.as_slice());
        assert_eq!("usize", generics.type_args[0].to_token_stream().to_string(),);
    }

    /// Operators that reject type arguments still report the extra type after a bad lifetime.
    #[test]
    fn bad_lifetime_still_reports_extra_type_argument() {
        let mut builder = FlatGraphBuilder::new();
        builder.add_dfir(
            parse_quote! {
                source_iter(0..10) -> inspect::<'a, usize>(std::mem::drop) -> for_each(std::mem::drop);
            },
            None,
            None,
        );
        let Err(diagnostics) = builder.build() else {
            panic!("expected diagnostics for an unknown lifetime and an extra type argument");
        };
        let messages = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            2,
            messages.len(),
            "expected only the unknown lifetime and the extra type argument, got: {messages:?}",
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("Unknown lifetime generic argument `'a`")),
            "missing unknown-lifetime diagnostic, got: {messages:?}",
        );
        assert!(
            messages.iter().any(|message| {
                message.contains(
                    "`inspect` should have exactly 0 generic type arguments, actually has 1.",
                )
            }),
            "missing extra-type diagnostic, got: {messages:?}",
        );
    }
}
