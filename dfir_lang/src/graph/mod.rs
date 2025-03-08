//! Graph representation stages for DFIR graphs.

use std::borrow::Cow;
use std::hash::Hash;

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use slotmap::new_key_type;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ExprPath, GenericArgument, Token, Type};

use self::ops::{OperatorConstraints, Persistence};
use crate::diagnostic::{Diagnostic, Level};
use crate::parse::{DfirCode, IndexInt, Operator, PortIndex, Ported};
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
pub use flat_graph_builder::FlatGraphBuilder;
pub use flat_to_partitioned::partition_graph;
pub use meta_graph::{DfirGraph, WriteConfig, WriteGraphType};

pub mod graph_algorithms;
pub mod ops;

new_key_type! {
    /// ID to identify a node (operator or handoff) in [`DfirGraph`].
    pub struct GraphNodeId;

    /// ID to identify an edge.
    pub struct GraphEdgeId;

    /// ID to identify a subgraph in [`DfirGraph`].
    pub struct GraphSubgraphId;

    /// ID to identify a loop block in [`DfirGraph`].
    pub struct GraphLoopId;
}

/// Context identifier as a string.
const CONTEXT: &str = "context";
/// Runnable DFIR graph object identifier as a string.
const GRAPH: &str = "df";

const HANDOFF_NODE_STR: &str = "handoff";
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
struct Varname(#[serde(with = "serde_syn")] pub Ident);

/// A node, corresponding to an operator or a handoff.
#[derive(Clone, Serialize, Deserialize)]
pub enum GraphNode {
    /// An operator.
    Operator(#[serde(with = "serde_syn")] Operator),
    /// A handoff point, used between subgraphs (or within a subgraph to break a cycle).
    Handoff {
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
        /// The value of this span when the ModuleBoundary node is still inside the module is Span::call_site()
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
            GraphNode::Handoff { .. } => HANDOFF_NODE_STR.into(),
            GraphNode::ModuleBoundary { .. } => MODULE_BOUNDARY_NODE_STR.into(),
        }
    }

    /// Return the name of the node as a string, excluding parenthesis and op source code.
    pub fn to_name_string(&self) -> Cow<'static, str> {
        match self {
            GraphNode::Operator(op) => op.name_string().into(),
            GraphNode::Handoff { .. } => HANDOFF_NODE_STR.into(),
            GraphNode::ModuleBoundary { .. } => MODULE_BOUNDARY_NODE_STR.into(),
        }
    }

    /// Return the source code span of the node (for operators) or input/otput spans for handoffs.
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
            Self::Handoff { .. } => write!(f, "Node::Handoff"),
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
    pub singletons_referenced: Vec<Ident>,

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

/// Gets the generic arguments for the operator.
///
/// This helper method is useful due to the special handling of persistence lifetimes (`'static`,
/// `'tick`, `'mutable`) which must come before other generic parameters.
pub fn get_operator_generics(
    diagnostics: &mut Vec<Diagnostic>,
    operator: &Operator,
) -> OpInstGenerics {
    // Generic arguments.
    let generic_args = operator.type_arguments().cloned();
    let persistence_args = generic_args.iter().flatten().map_while(|generic_arg| match generic_arg {
            GenericArgument::Lifetime(lifetime) => {
                match &*lifetime.ident.to_string() {
                    "none" => Some(Persistence::None),
                    "tick" => Some(Persistence::Tick),
                    "static" => Some(Persistence::Static),
                    "mutable" => Some(Persistence::Mutable),
                    _ => {
                        diagnostics.push(Diagnostic::spanned(
                            generic_arg.span(),
                            Level::Error,
                            format!("Unknown lifetime generic argument `'{}`, expected `'tick`, `'static`, or `'mutable`.", lifetime.ident),
                        ));
                        // TODO(mingwei): should really keep going and not short circuit?
                        None
                    }
                }
            },
            _ => None,
        }).collect::<Vec<_>>();
    let type_args = generic_args
        .iter()
        .flatten()
        .skip(persistence_args.len())
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

    /// Return `Err(self)` if there is a conflict.
    pub fn combine(self, other: Self) -> Result<Self, Self> {
        if self.is_specified() {
            if other.is_specified() {
                Err(self)
            } else {
                Ok(self)
            }
        } else {
            Ok(other)
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

/// The main function of this module. Compiles a [`DfirCode`] AST into a [`DfirGraph`] and
/// source code, or [`Diagnostic`] errors.
pub fn build_hfcode(
    hf_code: DfirCode,
    root: &TokenStream,
) -> (Option<(DfirGraph, TokenStream)>, Vec<Diagnostic>) {
    let flat_graph_builder = FlatGraphBuilder::from_dfir(hf_code);
    let (mut flat_graph, uses, mut diagnostics) = flat_graph_builder.build();
    if !diagnostics.iter().any(Diagnostic::is_error) {
        if let Err(diagnostic) = flat_graph.merge_modules() {
            diagnostics.push(diagnostic);
            return (None, diagnostics);
        }

        eliminate_extra_unions_tees(&mut flat_graph);
        match partition_graph(flat_graph) {
            Ok(partitioned_graph) => {
                let code = partitioned_graph.as_code(
                    root,
                    true,
                    quote::quote! { #( #uses )* },
                    &mut diagnostics,
                );
                if !diagnostics.iter().any(Diagnostic::is_error) {
                    // Success.
                    return (Some((partitioned_graph, code)), diagnostics);
                }
            }
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    (None, diagnostics)
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
