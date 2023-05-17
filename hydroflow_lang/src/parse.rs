//! AST for surface syntax, modelled on [`syn`]'s ASTs.

use std::hash::Hash;

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{Bracket, Paren};
use syn::{
    bracketed, parenthesized, AngleBracketedGenericArguments, Expr, ExprPath, GenericArgument,
    Ident, LitInt, Path, PathArguments, PathSegment, Token,
};

pub struct HfCode {
    pub statements: Punctuated<HfStatement, Token![;]>,
}
impl Parse for HfCode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let statements = Punctuated::parse_terminated(input)?;
        if !statements.empty_or_trailing() {
            return Err(input.parse::<Token![;]>().unwrap_err());
        }
        Ok(HfCode { statements })
    }
}
impl ToTokens for HfCode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.statements.to_tokens(tokens)
    }
}

pub enum HfStatement {
    Named(NamedHfStatement),
    Pipeline(Pipeline),
}
impl Parse for HfStatement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek2(Token![=]) {
            Ok(Self::Named(NamedHfStatement::parse(input)?))
        } else {
            Ok(Self::Pipeline(Pipeline::parse(input)?))
        }
    }
}
impl ToTokens for HfStatement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            HfStatement::Named(x) => x.to_tokens(tokens),
            HfStatement::Pipeline(x) => x.to_tokens(tokens),
        }
    }
}

pub struct NamedHfStatement {
    pub name: Ident,
    pub equals: Token![=],
    pub pipeline: Pipeline,
}
impl Parse for NamedHfStatement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let equals = input.parse()?;
        let pipeline = input.parse()?;
        Ok(Self {
            name,
            equals,
            pipeline,
        })
    }
}
impl ToTokens for NamedHfStatement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.equals.to_tokens(tokens);
        self.pipeline.to_tokens(tokens);
    }
}

pub enum Pipeline {
    Paren(Ported<PipelineParen>),
    Name(Ported<Ident>),
    Link(PipelineLink),
    Operator(Operator),
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
            // Emit lookahead expected tokens errors.
            else {
                Err(lookahead2.error())
            }
        }
        // Ident
        else if lookahead1.peek(Ident) {
            // If has paren or generic next, it's an operator
            if input.peek2(Paren) || input.peek2(Token![<]) || input.peek2(Token![::]) {
                Ok(Self::Operator(input.parse()?))
            }
            // Otherwise it's a name
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
            Pipeline::Paren(x) => x.to_tokens(tokens),
            Pipeline::Link(x) => x.to_tokens(tokens),
            Pipeline::Name(x) => x.to_tokens(tokens),
            Pipeline::Operator(x) => x.to_tokens(tokens),
        }
    }
}

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

#[derive(Clone)]
pub struct Operator {
    pub path: Path,
    pub paren_token: Paren,
    pub args: Punctuated<Expr, Token![,]>,
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
        let file: syn::File = syn::parse_quote! {
            fn main() {
                #self
            }
        };
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
        let args = Punctuated::parse_terminated(&content)?;

        Ok(Self {
            path,
            paren_token,
            args,
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
        let lit_int = LitInt::new(&*self.value.to_string(), self.span);
        lit_int.to_tokens(tokens)
    }
}
impl Hash for IndexInt {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}
impl PartialOrd for IndexInt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}
impl PartialEq for IndexInt {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl Eq for IndexInt {}
impl Ord for IndexInt {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
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
