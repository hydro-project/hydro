//! Graph representation stages for Hydroflow graphs.

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
use crate::parse::{HfCode, IndexInt, Operator, PortIndex, Ported};
use crate::pretty_span::PrettySpan;

mod di_mul_graph;
mod eliminate_extra_merges_tees;
mod flat_graph_builder;
mod flat_to_partitioned;
mod graph_write;
mod hydroflow_graph;

pub use di_mul_graph::DiMulGraph;
pub use eliminate_extra_merges_tees::eliminate_extra_merges_tees;
pub use flat_graph_builder::FlatGraphBuilder;
pub use flat_to_partitioned::partition_graph;
pub use hydroflow_graph::HydroflowGraph;

pub mod graph_algorithms;
pub mod ops;

new_key_type! {
    /// ID to identify a node (operator or handoff) in [`HydroflowGraph`].
    pub struct GraphNodeId;

    /// ID to identify an edge.
    pub struct GraphEdgeId;

    /// ID to identify a subgraph in [`HydroflowGraph`].
    pub struct GraphSubgraphId;
}

/// Context identifier as a string.
const CONTEXT: &str = "context";
/// Hydroflow identifier as a string.
const HYDROFLOW: &str = "df";

const HANDOFF_NODE_STR: &str = "handoff";

mod serde_syn {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: quote::ToTokens,
    {
        serializer.serialize_str(&*value.to_token_stream().to_string())
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: syn::parse::Parse,
    {
        let s = String::deserialize(deserializer)?;
        syn::parse_str(&*s).map_err(<D::Error as serde::de::Error>::custom)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
struct Varname(#[serde(with = "serde_syn")] pub Ident);

#[derive(Clone, Serialize, Deserialize)]
pub enum Node {
    Operator(#[serde(with = "serde_syn")] Operator),
    Handoff {
        #[serde(skip, default = "Span::call_site")]
        src_span: Span,
        #[serde(skip, default = "Span::call_site")]
        dst_span: Span,
    },
}
impl Node {
    pub fn to_pretty_string(&self) -> Cow<'static, str> {
        match self {
            Node::Operator(op) => op.to_pretty_string().into(),
            Node::Handoff { .. } => HANDOFF_NODE_STR.into(),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Self::Operator(op) => op.span(),
            &Self::Handoff { src_span, dst_span } => src_span.join(dst_span).unwrap_or(src_span),
        }
    }
}
impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operator(operator) => {
                write!(f, "Node::Operator({} span)", PrettySpan(operator.span()))
            }
            Self::Handoff { .. } => write!(f, "Node::Handoff"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct OperatorInstance {
    /// Name of the operator (will match [`OperatorConstraints::name`]).
    pub op_constraints: &'static OperatorConstraints,
    // /// The source span of this operator instance.
    // pub op_span: Span,
    /// Port values used as this operator's input.
    pub input_ports: Vec<PortIndexValue>,
    /// Port values used as this operator's output.
    pub output_ports: Vec<PortIndexValue>,

    /// Generic arguments.
    pub generics: OpInstGenerics,
    /// Arguments provided by the user into the operator as arguments.
    /// I.e. the `a, b, c` in `-> my_op(a, b, c) -> `.
    pub arguments: Punctuated<Expr, Token![,]>,
}

#[derive(Clone, Debug)]
pub struct OpInstGenerics {
    /// Operator generic (type or lifetime) arguments.
    pub generic_args: Option<Punctuated<GenericArgument, Token![,]>>,
    /// Lifetime persistence arguments. Corresponds to a prefix of [`Self::generic_args`].
    pub persistence_args: Vec<Persistence>,
    /// Type persistence arguments. Corersponds to a (suffix) of [`Self::generic_args`].
    pub type_args: Vec<Type>,
}

pub fn get_operator_generics(
    diagnostics: &mut Vec<Diagnostic>,
    operator: &Operator,
) -> OpInstGenerics {
    // Generic arguments.
    let generic_args = operator.type_arguments().cloned();
    let persistence_args = generic_args.iter().flatten().map_while(|generic_arg| match generic_arg {
            GenericArgument::Lifetime(lifetime) => {
                match &*lifetime.ident.to_string() {
                    "static" => Some(Persistence::Static),
                    "tick" => Some(Persistence::Tick),
                    _ => {
                        diagnostics.push(Diagnostic::spanned(
                            generic_arg.span(),
                            Level::Error,
                            format!("Unknown lifetime generic argument `'{}`, expected `'tick` or `'static`.", lifetime.ident),
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

/// Determine op color based on in and out degree. If linear (1 in 1 out), color is None.
///
/// Note that this does NOT consider `DelayType` barriers, which generally imply `Pull`.
pub fn node_color(is_handoff: bool, inn_degree: usize, out_degree: usize) -> Option<Color> {
    if is_handoff {
        Some(Color::Hoff)
    } else {
        match (1 < inn_degree, 1 < out_degree) {
            (true, true) => Some(Color::Comp),
            (true, false) => Some(Color::Pull),
            (false, true) => Some(Color::Push),
            (false, false) => match (inn_degree, out_degree) {
                (0, _) => Some(Color::Pull),
                (_, 0) => Some(Color::Push),
                // (1, 1) =>
                _both_unary => None,
            },
        }
    }
}

/// Helper struct for [`PortIndex`] which keeps span information for elided ports.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PortIndexValue {
    Int(#[serde(with = "serde_syn")] IndexInt),
    Path(#[serde(with = "serde_syn")] ExprPath),
    Elided(#[serde(skip)] Option<Span>),
}
impl PortIndexValue {
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

    pub fn as_error_message_string(&self) -> String {
        match self {
            PortIndexValue::Int(n) => format!("`{}`", n.value),
            PortIndexValue::Path(path) => format!("`{}`", path.to_token_stream()),
            PortIndexValue::Elided(_) => "<elided>".to_owned(),
        }
    }

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
        match (self, other) {
            (Self::Int(s), Self::Int(o)) => s.partial_cmp(o),
            (Self::Path(s), Self::Path(o)) => s
                .to_token_stream()
                .to_string()
                .partial_cmp(&o.to_token_stream().to_string()),
            (Self::Elided(_), Self::Elided(_)) => Some(std::cmp::Ordering::Equal),
            (Self::Int(_), Self::Path(_)) => Some(std::cmp::Ordering::Less),
            (Self::Path(_), Self::Int(_)) => Some(std::cmp::Ordering::Greater),
            (_, Self::Elided(_)) => Some(std::cmp::Ordering::Less),
            (Self::Elided(_), _) => Some(std::cmp::Ordering::Greater),
        }
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

pub fn build_hfcode(
    hf_code: HfCode,
    root: &TokenStream,
) -> (Option<(HydroflowGraph, TokenStream)>, Vec<Diagnostic>) {
    let flat_graph_builder = FlatGraphBuilder::from_hfcode(hf_code);
    let (mut flat_graph, mut diagnostics) = flat_graph_builder.build();
    eliminate_extra_merges_tees(&mut flat_graph);
    if !diagnostics.iter().any(Diagnostic::is_error) {
        match partition_graph(flat_graph) {
            Ok(part_graph) => {
                let code = part_graph.as_code(root, true, &mut diagnostics);
                if !diagnostics.iter().any(Diagnostic::is_error) {
                    // Success.
                    return (Some((part_graph, code)), diagnostics);
                }
            }
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    (None, diagnostics)
}
