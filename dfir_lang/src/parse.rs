//! AST for surface syntax, modelled on [`syn`]'s ASTs.
#![allow(clippy::allow_attributes, missing_docs, reason = "internal use")]

use std::fmt::Debug;
use std::hash::Hash;

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{Brace, Bracket, Paren};
use syn::{
    AngleBracketedGenericArguments, Expr, ExprPath, GenericArgument, Ident, ItemUse, LitInt, Path,
    PathArguments, PathSegment, Token, braced, bracketed, parenthesized,
};

use crate::process_singletons::preprocess_singletons;

pub struct DfirCode {
    pub statements: Vec<DfirStatement>,
}
impl Parse for DfirCode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut statements = Vec::new();
        while !input.is_empty() {
            statements.push(input.parse()?);
        }
        Ok(DfirCode { statements })
    }
}
impl ToTokens for DfirCode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for statement in self.statements.iter() {
            statement.to_tokens(tokens);
        }
    }
}

pub enum DfirStatement {
    Use(ItemUse),
    Named(NamedStatement),
    Pipeline(PipelineStatement),
    Loop(LoopStatement),
}
impl Parse for DfirStatement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead1 = input.lookahead1();
        if lookahead1.peek(Token![use]) {
            Ok(Self::Use(ItemUse::parse(input)?))
        } else if lookahead1.peek(Paren) || lookahead1.peek(Bracket) || lookahead1.peek(Token![mod])
        {
            Ok(Self::Pipeline(PipelineStatement::parse(input)?))
        } else if lookahead1.peek(Token![loop]) {
            Ok(Self::Loop(LoopStatement::parse(input)?))
        } else if lookahead1.peek(Ident) {
            let fork = input.fork();
            let _: Path = fork.parse()?;
            let lookahead2 = fork.lookahead1();
            if lookahead2.peek(Token![=]) {
                Ok(Self::Named(NamedStatement::parse(input)?))
            } else if lookahead2.peek(Token![->])
                || lookahead2.peek(Paren)
                || lookahead2.peek(Bracket)
            {
                Ok(Self::Pipeline(PipelineStatement::parse(input)?))
            } else {
                Err(lookahead2.error())
            }
        } else {
            Err(lookahead1.error())
        }
    }
}
impl ToTokens for DfirStatement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Use(x) => x.to_tokens(tokens),
            Self::Named(x) => x.to_tokens(tokens),
            Self::Pipeline(x) => x.to_tokens(tokens),
            Self::Loop(x) => x.to_tokens(tokens),
        }
    }
}

pub struct NamedStatement {
    pub name: Ident,
    pub equals: Token![=],
    pub pipeline: Pipeline,
    pub semi_token: Token![;],
}
impl Parse for NamedStatement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let equals = input.parse()?;
        let pipeline = input.parse()?;
        let semi_token = input.parse()?;
        Ok(Self {
            name,
            equals,
            pipeline,
            semi_token,
        })
    }
}
impl ToTokens for NamedStatement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.equals.to_tokens(tokens);
        self.pipeline.to_tokens(tokens);
        self.semi_token.to_tokens(tokens);
    }
}

pub struct PipelineStatement {
    pub pipeline: Pipeline,
    pub semi_token: Token![;],
}
impl Parse for PipelineStatement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pipeline = input.parse()?;
        let semi_token = input.parse()?;
        Ok(Self {
            pipeline,
            semi_token,
        })
    }
}
impl ToTokens for PipelineStatement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.pipeline.to_tokens(tokens);
        self.semi_token.to_tokens(tokens);
    }
}

#[derive(Clone, Debug)]
pub enum Pipeline {
    Paren(Ported<PipelineParen>),
    Name(Ported<Ident>),
    Link(PipelineLink),
    Operator(Operator),
    ModuleBoundary(Ported<Token![mod]>),
}
impl Pipeline {
    fn parse_one(input: ParseStream) -> syn::Result<Self> {
        let lookahead1 = input.lookahead1();

        // Leading indexing
        if lookahead1.peek(Bracket) {
            let inn_idx = input.parse()?;
            let lookahead2 = input.lookahead1();
            // Indexed paren
            if lookahead2.peek(Paren) {
                Ok(Self::Paren(Ported::parse_rest(Some(inn_idx), input)?))
            }
            // Indexed name
            else if lookahead2.peek(Ident) {
                Ok(Self::Name(Ported::parse_rest(Some(inn_idx), input)?))
            }
            // Indexed module boundary
            else if lookahead2.peek(Token![mod]) {
                Ok(Self::ModuleBoundary(Ported::parse_rest(
                    Some(inn_idx),
                    input,
                )?))
            }
            // Emit lookahead expected tokens errors.
            else {
                Err(lookahead2.error())
            }
        // module input/output
        } else if lookahead1.peek(Token![mod]) {
            Ok(Self::ModuleBoundary(input.parse()?))
        // Ident or macro-style expression
        } else if lookahead1.peek(Ident) {
            let speculative = input.fork();
            let _ident: Ident = speculative.parse()?;

            // If has paren or generic next, it's an operator
            if speculative.peek(Paren)
                || speculative.peek(Token![<])
                || speculative.peek(Token![::])
            {
                Ok(Self::Operator(input.parse()?))
            }
            // Otherwise it's a variable name
            else {
                Ok(Self::Name(input.parse()?))
            }
        }
        // Paren group
        else if lookahead1.peek(Paren) {
            Ok(Self::Paren(input.parse()?))
        }
        // Emit lookahead expected tokens errors.
        else {
            Err(lookahead1.error())
        }
    }
}
impl Parse for Pipeline {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lhs = Pipeline::parse_one(input)?;
        if input.is_empty() || input.peek(Token![;]) {
            Ok(lhs)
        } else {
            let arrow = input.parse()?;
            let rhs = input.parse()?;
            let lhs = Box::new(lhs);
            Ok(Self::Link(PipelineLink { lhs, arrow, rhs }))
        }
    }
}
impl ToTokens for Pipeline {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Paren(x) => x.to_tokens(tokens),
            Self::Link(x) => x.to_tokens(tokens),
            Self::Name(x) => x.to_tokens(tokens),
            Self::Operator(x) => x.to_tokens(tokens),
            Self::ModuleBoundary(x) => x.to_tokens(tokens),
        }
    }
}

pub struct LoopStatement {
    pub loop_token: Token![loop],
    pub ident: Option<Ident>,
    pub brace_token: Brace,
    pub statements: Vec<DfirStatement>,
    pub semi_token: Token![;],
}
impl Parse for LoopStatement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let loop_token = input.parse()?;
        let ident = input.parse()?;
        let content;
        let brace_token = braced!(content in input);
        let mut statements = Vec::new();
        while !content.is_empty() {
            statements.push(content.parse()?);
        }
        let semi_token = input.parse()?;
        Ok(Self {
            loop_token,
            ident,
            brace_token,
            statements,
            semi_token,
        })
    }
}
impl ToTokens for LoopStatement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.loop_token.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        self.brace_token.surround(tokens, |tokens| {
            for statement in self.statements.iter() {
                statement.to_tokens(tokens);
            }
        });
        self.semi_token.to_tokens(tokens);
    }
}

#[derive(Clone, Debug)]
pub struct Ported<Inner> {
    pub inn: Option<Indexing>,
    pub inner: Inner,
    pub out: Option<Indexing>,
}
impl<Inner> Ported<Inner>
where
    Inner: Parse,
{
    /// The caller will often parse the first port (`inn`) as part of determining what to parse
    /// next, so this will do the rest after that.
    fn parse_rest(inn: Option<Indexing>, input: ParseStream) -> syn::Result<Self> {
        let inner = input.parse()?;
        let out = input.call(Indexing::parse_opt)?;
        Ok(Self { inn, inner, out })
    }
}
impl<Inner> Parse for Ported<Inner>
where
    Inner: Parse,
{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let inn = input.call(Indexing::parse_opt)?;
        Self::parse_rest(inn, input)
    }
}
impl<Inner> ToTokens for Ported<Inner>
where
    Inner: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.inn.to_tokens(tokens);
        self.inner.to_tokens(tokens);
        self.out.to_tokens(tokens);
    }
}

#[derive(Clone, Debug)]
pub struct PipelineParen {
    pub paren_token: Paren,
    pub pipeline: Box<Pipeline>,
}
impl Parse for PipelineParen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let paren_token = parenthesized!(content in input);
        let pipeline = content.parse()?;
        Ok(Self {
            paren_token,
            pipeline,
        })
    }
}
impl ToTokens for PipelineParen {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.paren_token.surround(tokens, |tokens| {
            self.pipeline.to_tokens(tokens);
        });
    }
}

#[derive(Clone, Debug)]
pub struct PipelineLink {
    pub lhs: Box<Pipeline>,
    pub arrow: Token![->],
    pub rhs: Box<Pipeline>,
}
impl Parse for PipelineLink {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lhs = input.parse()?;
        let arrow = input.parse()?;
        let rhs = input.parse()?;

        Ok(Self { lhs, arrow, rhs })
    }
}
impl ToTokens for PipelineLink {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.lhs.to_tokens(tokens);
        self.arrow.to_tokens(tokens);
        self.rhs.to_tokens(tokens);
    }
}

#[derive(Clone, Debug)]
pub struct Indexing {
    pub bracket_token: Bracket,
    pub index: PortIndex,
}
impl Indexing {
    fn parse_opt(input: ParseStream) -> syn::Result<Option<Self>> {
        input.peek(Bracket).then(|| input.parse()).transpose()
    }
}
impl Parse for Indexing {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let bracket_token = bracketed!(content in input);
        let index = content.parse()?;
        Ok(Self {
            bracket_token,
            index,
        })
    }
}
impl ToTokens for Indexing {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.bracket_token.surround(tokens, |tokens| {
            self.index.to_tokens(tokens);
        });
    }
}

/// Port can either be an int or a name (path).
#[derive(Clone, Debug)]
pub enum PortIndex {
    Int(IndexInt),
    Path(ExprPath),
}
impl Parse for PortIndex {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitInt) {
            input.parse().map(Self::Int)
        } else {
            input.parse().map(Self::Path)
        }
    }
}
impl ToTokens for PortIndex {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            PortIndex::Int(index_int) => index_int.to_tokens(tokens),
            PortIndex::Path(expr_path) => expr_path.to_tokens(tokens),
        }
    }
}

struct TypeHintRemover;
impl syn::visit_mut::VisitMut for TypeHintRemover {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Call(expr_call) = expr &&
            let Expr::Path(path) = expr_call.func.as_ref() &&
                // if it is a call of the form `::...::*_type_hint(xyz)`,
                // typically `::stageleft::...`, replace it with `xyz`
                path
                    .path
                    .segments
                    .last()
                    .unwrap()
                    .ident
                    .to_string()
                    .ends_with("_type_hint")
        {
            *expr = expr_call.args.first().unwrap().clone();
        }

        syn::visit_mut::visit_expr_mut(self, expr);
    }
}

#[derive(Clone)]
pub struct Operator {
    pub path: Path,
    pub paren_token: Paren,
    pub args_raw: TokenStream,
    pub args: Punctuated<Expr, Token![,]>,
    pub singletons_referenced: Vec<Ident>,
}

impl Operator {
    pub fn name(&self) -> Path {
        Path {
            leading_colon: self.path.leading_colon,
            segments: self
                .path
                .segments
                .iter()
                .map(|seg| PathSegment {
                    ident: seg.ident.clone(),
                    arguments: PathArguments::None,
                })
                .collect(),
        }
    }

    pub fn name_string(&self) -> String {
        self.name().to_token_stream().to_string()
    }

    pub fn type_arguments(&self) -> Option<&Punctuated<GenericArgument, Token![,]>> {
        let end = self.path.segments.last()?;
        if let PathArguments::AngleBracketed(type_args) = &end.arguments {
            Some(&type_args.args)
        } else {
            None
        }
    }

    pub fn args(&self) -> &Punctuated<Expr, Token![,]> {
        &self.args
    }

    /// Output the operator as a formatted string using `prettyplease`.
    pub fn to_pretty_string(&self) -> String {
        // TODO(mingwei): preserve #args_raw instead of just args?
        let mut file: syn::File = syn::parse_quote! {
            fn main() {
                #self
            }
        };

        syn::visit_mut::visit_file_mut(&mut TypeHintRemover, &mut file);
        let str = prettyplease::unparse(&file);
        str.trim_start()
            .trim_start_matches("fn main()")
            .trim_start()
            .trim_start_matches('{')
            .trim_start()
            .trim_end()
            .trim_end_matches('}')
            .trim_end()
            .replace("\n    ", "\n") // Remove extra leading indent
    }
}
impl Parse for Operator {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        if let Some(path_seg) = path.segments.iter().find(|path_seg| {
            matches!(
                &path_seg.arguments,
                PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    colon2_token: None,
                    ..
                })
            )
        }) {
            return Err(syn::Error::new_spanned(
                path_seg,
                "Missing `::` before `<...>` generic arguments",
            ));
        }

        let content;
        let paren_token = parenthesized!(content in input);
        let args_raw: TokenStream = content.parse()?;
        let mut singletons_referenced = Vec::new();
        let args = parse_terminated(preprocess_singletons(
            args_raw.clone(),
            &mut singletons_referenced,
        ))?;

        Ok(Self {
            path,
            paren_token,
            args_raw,
            args,
            singletons_referenced,
        })
    }
}

impl ToTokens for Operator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.path.to_tokens(tokens);
        self.paren_token.surround(tokens, |tokens| {
            self.args.to_tokens(tokens);
        });
    }
}

impl Debug for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Operator")
            .field("path", &self.path.to_token_stream().to_string())
            .field(
                "args",
                &self
                    .args
                    .iter()
                    .map(|a| a.to_token_stream().to_string())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct IndexInt {
    pub value: isize,
    pub span: Span,
}
impl Parse for IndexInt {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit_int: LitInt = input.parse()?;
        let value = lit_int.base10_parse()?;
        Ok(Self {
            value,
            span: lit_int.span(),
        })
    }
}
impl ToTokens for IndexInt {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let lit_int = LitInt::new(&self.value.to_string(), self.span);
        lit_int.to_tokens(tokens)
    }
}
impl Hash for IndexInt {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}
impl PartialEq for IndexInt {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl Eq for IndexInt {}
impl PartialOrd for IndexInt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for IndexInt {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

pub fn parse_terminated<T, P>(tokens: TokenStream) -> syn::Result<Punctuated<T, P>>
where
    T: Parse,
    P: Parse,
{
    struct ParseTerminated<T, P>(pub Punctuated<T, P>);
    impl<T, P> Parse for ParseTerminated<T, P>
    where
        T: Parse,
        P: Parse,
    {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(Self(Punctuated::parse_terminated(input)?))
        }
    }

    Ok(syn::parse2::<ParseTerminated<T, P>>(tokens)?.0)
}

#[cfg(test)]
mod test {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_operator_to_pretty_string() {
        let op: Operator = parse_quote! {
            demux(|(msg, addr), var_args!(clients, msgs, errs)|
                match msg {
                    Message::ConnectRequest => clients.give(addr),
                    Message::ChatMsg {..} => msgs.give(msg),
                    _ => errs.give(msg),
                }
            )
        };
        assert_eq!(
            r"
demux(|(msg, addr), var_args!(clients, msgs, errs)| match msg {
    Message::ConnectRequest => clients.give(addr),
    Message::ChatMsg { .. } => msgs.give(msg),
    _ => errs.give(msg),
})
"
            .trim(),
            op.to_pretty_string()
        );
    }
}
