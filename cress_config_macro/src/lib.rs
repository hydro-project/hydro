//! Macros that generate a Cress amplification-check harness from a Hydro function's signature.
//! They are re-exported by the `cress` crate, which is where users name them:
//! `#[cress::amplification_check(...)]` and `#[derive(cress::SimOutputs)]`.
//!
//! # `#[amplification_check(...)]`
//!
//! Placed on a Hydro function, this attribute leaves the function unchanged and registers a
//! checker entry next to it. The entry builds a simulation of the function, feeds it a steady
//! workload, runs `cress::check`, and records both a concise result for the `cress` command and a
//! full report artifact. It is ordinary tool code, not a Rust test. The crate under check lists
//! `cress` as an optional dependency and writes the attribute as
//! `#[cfg_attr(feature = "cress", cress::amplification_check(...))]`, so that the checker and the
//! simulator are linked only when the `cress` command turns that feature on.
//!
//! ## What the macro reads from the signature
//!
//! Every parameter is classified by its type.
//!
//! * `&Process<'a, X>` is a location. The harness creates it with `flow.process::<X>()`.
//! * `&Cluster<'a, X>` is a location. The harness creates it with `flow.cluster::<X>()` and
//!   needs its size from the attribute, as `x = <members>` where `x` is the parameter's name.
//! * `Stream<T, Loc, Unbounded, O, R>` (the last two may be omitted) is an input. The harness
//!   creates a `sim_input` on the matching location and, each round, sends it `rate` values of
//!   `T` taken from `cress::InputValue`, or `rate` values to every
//!   member when the location is a cluster. The rate is one unless the `workload(...)` clause
//!   says otherwise. A stream whose item type is `()` is treated exactly the same way, which is
//!   what a timer needs; nothing in the attribute is required for timers. The ordering may be
//!   `TotalOrder` or `NoOrder` (the harness uses the sender method the simulator provides for
//!   each); the retries must be `ExactlyOnce`, since the simulator's input senders feed nothing
//!   else.
//! * `KeyedStream<K, V, Loc, Unbounded, O, R>` is an input fed the same way with `(K, V)` pairs
//!   from `cress::InputValue`, converted with `.into_keyed()`.
//! * Anything else is a plain value that must be given in the attribute by the parameter's
//!   name, for example `policy = RetryPolicy { timeout_ticks: 40, max_attempts: 3 }`.
//!
//! Every generic type parameter of the function must be given a concrete type, as `T = u64`. A
//! stream whose location does not appear among the parameters gets a location of its own.
//!
//! The function's return value is sunk with `cress::SimOutputs`, which
//! covers a single stream, a keyed stream, a tuple of them, `()`, and any struct that derives
//! [`SimOutputs`](macro@SimOutputs).
//!
//! ## What the attribute accepts
//!
//! * `name = ident` or `name = "string"`: the configuration's name, shown with its verdict.
//! * `T = Type`: the concrete type for generic parameter `T`.
//! * `param = expr`: the value of a plain parameter, or the size of a cluster parameter.
//! * `workload(param = rate, other = [r1, r2, r3])`: per-round rates for stream parameters. A
//!   list runs the check once per rate, and once per combination when several parameters carry
//!   lists, so a verdict that changes with the rate is visible in the summary. Streams not named
//!   get one value per round.
//! * `round = |round, inputs| { ... }`: replaces the generated workload with a closure that is
//!   called once per round. `inputs` has one field per stream parameter, named after it, of type
//!   `SimSender<T, O, R>` or `SimClusterSender<T, O, R>` (with `T = (K, V)` for a keyed stream),
//!   and one field `<cluster>_members: u32`
//!   per cluster parameter. This cannot be combined with `workload(...)`.
//! * `horizon = rounds`: passed to `CheckConfig::new`; the default is the checker's default.
//! * `workload_rounds = rounds`: confines the data to the first `rounds` rounds of the run, so
//!   that holds can outlast the input and the report's growth label can tell extra work that
//!   stops once the input is over from extra work that keeps rising with the delay. The streams
//!   named in `workload(...)` are the data and stop after `rounds`; the streams not named are
//!   timers and are fed in every round. With `round = ...` the closure receives every round and
//!   must itself stop sending data at `rounds`; the value is only passed to the config. For the
//!   growth label to be read, at least two finite hold lengths must outlast the input, which
//!   with the doubling sequence means about a quarter of the horizon or less (125 of 1000).
//! * `ignore = "reason"`: omits this check from the checker command unless its
//!   `--include-ignored` option is given.
//! * `skip_consistency_assertions`: calls `SimFlow::skip_consistency_assertions` on the
//!   simulation, for programs that use `assert_has_consistency`.
//! * `cress = path`: the path to the `cress` crate, `::cress` by default. The generated code
//!   reaches `hydro_lang` through `cress`, so the crate under check need not depend on it directly.
//!
//! # `#[derive(SimOutputs)]`
//!
//! Implements `cress::SimOutputs` for a struct by sinking every field in turn. The derive always
//! names the crate as `::cress`; it takes no path option. Each field's type must itself implement `SimOutputs`, which every `Stream` and
//! `KeyedStream` on a `Process` or `Cluster` does.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{
    Expr, FnArg, GenericArgument, GenericParam, Ident, ItemFn, LitStr, Pat, PathArguments,
    ReturnType, Token, Type, TypePath, parenthesized,
};

// ---------------------------------------------------------------------------------------------
// Attribute arguments

/// One `name = tokens`, `name(...)` or bare `name` entry of the attribute.
struct Entry {
    key: Ident,
    /// Present for `key = ...`.
    value: Option<TokenStream2>,
    /// Present for `key(...)`.
    group: Option<Vec<Entry>>,
}

/// Reads token trees up to, and not including, the next top-level comma.
fn tokens_until_comma(input: ParseStream<'_>) -> syn::Result<TokenStream2> {
    input.step(|cursor| {
        let mut rest = *cursor;
        let mut out = Vec::new();
        while let Some((tt, next)) = rest.token_tree() {
            if let TokenTree::Punct(p) = &tt
                && p.as_char() == ','
            {
                break;
            }
            out.push(tt);
            rest = next;
        }
        Ok((TokenStream2::from_iter(out), rest))
    })
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            // A closure's parameter list has top-level commas, so `round` is parsed as an
            // expression; every other value is the raw tokens up to the next comma, because a
            // type such as `Vec<(u32, u32)>` is not an expression.
            let value = if key == "round" {
                input.parse::<Expr>()?.to_token_stream()
            } else {
                tokens_until_comma(input)?
            };
            if value.is_empty() {
                return Err(syn::Error::new(key.span(), "expected a value after `=`"));
            }
            Ok(Entry {
                key,
                value: Some(value),
                group: None,
            })
        } else if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            let inner: Punctuated<Entry, Token![,]> = content.parse_terminated(Entry::parse, Token![,])?;
            Ok(Entry {
                key,
                value: None,
                group: Some(inner.into_iter().collect()),
            })
        } else {
            Ok(Entry {
                key,
                value: None,
                group: None,
            })
        }
    }
}

struct Args {
    entries: Vec<Entry>,
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let entries: Punctuated<Entry, Token![,]> = input.parse_terminated(Entry::parse, Token![,])?;
        Ok(Args {
            entries: entries.into_iter().collect(),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Signature analysis

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LocKind {
    Process,
    Cluster,
}

/// A location the harness must create, identified by its kind and marker type.
struct Loc {
    kind: LocKind,
    marker: Type,
    /// The variable holding it in the generated code: the parameter's name when the location is
    /// a parameter, otherwise a synthesized one.
    var: Ident,
    /// Whether the location is a parameter of the function.
    is_param: bool,
}

struct StreamParam {
    name: Ident,
    /// The item type fed to the `sim_input`: `T` for a `Stream<T, ..>`, `(K, V)` for a
    /// `KeyedStream<K, V, ..>`.
    item: Type,
    order: Type,
    retries: Type,
    /// Index into the harness's location list.
    loc: usize,
    /// Whether the parameter is a `KeyedStream`, so the harness converts the `sim_input` stream
    /// of `(K, V)` pairs with `.into_keyed()` before passing it.
    keyed: bool,
    /// Whether the stream's ordering is `NoOrder`, in which case the simulator's sender only has
    /// `send_many_unordered`.
    unordered: bool,
}

enum Param {
    Loc(usize),
    Stream(usize),
    Plain(Ident),
}

fn last_ident(ty: &Type) -> Option<(&Ident, &PathArguments)> {
    if let Type::Path(TypePath { qself: None, path }) = ty {
        let seg = path.segments.last()?;
        Some((&seg.ident, &seg.arguments))
    } else {
        None
    }
}

fn type_args(args: &PathArguments) -> Vec<&Type> {
    match args {
        PathArguments::AngleBracketed(a) => a
            .args
            .iter()
            .filter_map(|g| match g {
                GenericArgument::Type(t) => Some(t),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn unit_type() -> Type {
    syn::parse_quote!(())
}

/// Classifies a `Process<'a, X>` or `Cluster<'a, X>` type, if it is one.
fn as_location(ty: &Type) -> syn::Result<Option<(LocKind, Type)>> {
    let Some((ident, args)) = last_ident(ty) else {
        return Ok(None);
    };
    let kind = match ident.to_string().as_str() {
        "Process" => LocKind::Process,
        "Cluster" => LocKind::Cluster,
        _ => return Ok(None),
    };
    let targs = type_args(args);
    if kind == LocKind::Cluster && targs.len() > 1 {
        return Err(syn::Error::new(
            ty.span(),
            "cluster locations with an explicit consistency type parameter are not supported by #[amplification_check]",
        ));
    }
    let marker = targs.first().copied().cloned().unwrap_or_else(unit_type);
    Ok(Some((kind, marker)))
}

/// Replaces single-segment type paths naming a generic parameter with the concrete type.
struct Substitute<'s> {
    map: &'s [(String, Type)],
}

impl VisitMut for Substitute<'_> {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Path(TypePath { qself: None, path }) = ty
            && path.segments.len() == 1
            && path.segments[0].arguments.is_none()
        {
            let name = path.segments[0].ident.to_string();
            if let Some((_, concrete)) = self.map.iter().find(|(n, _)| *n == name) {
                *ty = concrete.clone();
                return;
            }
        }
        syn::visit_mut::visit_type_mut(self, ty);
    }
}

fn type_key(ty: &Type) -> String {
    ty.to_token_stream().to_string()
}

// ---------------------------------------------------------------------------------------------
// The attribute

/// See the [crate documentation](crate).
#[proc_macro_attribute]
pub fn amplification_check(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = match syn::parse::<ItemFn>(item.clone()) {
        Ok(f) => f,
        Err(e) => {
            let item = TokenStream2::from(item);
            let err = e.to_compile_error();
            return quote!(#item #err).into();
        }
    };
    let args = match syn::parse::<Args>(attr) {
        Ok(a) => a,
        Err(e) => {
            let err = e.to_compile_error();
            return quote!(#item_fn #err).into();
        }
    };
    match expand(&item_fn, args) {
        Ok(entry) => quote!(#item_fn #entry).into(),
        Err(e) => {
            let err = e.to_compile_error();
            quote!(#item_fn #err).into()
        }
    }
}

fn expand(f: &ItemFn, args: Args) -> syn::Result<TokenStream2> {
    let fn_name = &f.sig.ident;

    // Generic type parameters that need concrete types.
    let generic_names: Vec<String> = f
        .sig
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Type(t) => Some(t.ident.to_string()),
            _ => None,
        })
        .collect();

    // Sort the attribute's entries.
    let mut name: Option<String> = None;
    let mut horizon: Option<Expr> = None;
    let mut workload_rounds: Option<Expr> = None;
    let mut ignore: Option<LitStr> = None;
    let mut cress: syn::Path = syn::parse_quote!(::cress);
    let mut hydro: syn::Path = syn::parse_quote!(::cress::__private::hydro_lang);
    let mut skip_consistency = false;
    let mut round_closure: Option<Expr> = None;
    let mut workload: Vec<(Ident, Expr)> = Vec::new();
    let mut generic_types: Vec<(String, Type)> = Vec::new();
    let mut values: Vec<(String, Expr, Span)> = Vec::new();

    for e in args.entries {
        let key = e.key.to_string();
        match (key.as_str(), &e.value, &e.group) {
            ("name", Some(v), _) => {
                name = Some(match syn::parse2::<LitStr>(v.clone()) {
                    Ok(s) => s.value(),
                    Err(_) => syn::parse2::<Ident>(v.clone())
                        .map_err(|_| syn::Error::new(e.key.span(), "`name` takes an identifier or a string"))?
                        .to_string(),
                });
            }
            ("horizon", Some(v), _) => horizon = Some(syn::parse2(v.clone())?),
            ("workload_rounds", Some(v), _) => workload_rounds = Some(syn::parse2(v.clone())?),
            ("ignore", Some(v), _) => ignore = Some(syn::parse2(v.clone())?),
            ("cress", Some(v), _) => {
                cress = syn::parse2(v.clone())?;
                hydro = syn::parse_quote!(#cress::__private::hydro_lang);
            }
            ("skip_consistency_assertions", None, None) => skip_consistency = true,
            ("round", Some(v), _) => round_closure = Some(syn::parse2(v.clone())?),
            ("workload", None, Some(g)) => {
                for w in g {
                    let Some(v) = &w.value else {
                        return Err(syn::Error::new(w.key.span(), "workload entries look like `param = rate` or `param = [r1, r2]`"));
                    };
                    workload.push((w.key.clone(), syn::parse2(v.clone())?));
                }
            }
            (k, Some(v), _) if generic_names.iter().any(|g| g == k) => {
                generic_types.push((k.to_owned(), syn::parse2::<Type>(v.clone())?));
            }
            (_, Some(v), _) => {
                values.push((key.clone(), syn::parse2::<Expr>(v.clone())?, e.key.span()));
            }
            _ => {
                return Err(syn::Error::new(
                    e.key.span(),
                    format!("unrecognized entry `{key}`; see the documentation of #[amplification_check]"),
                ));
            }
        }
    }
    if round_closure.is_some() && !workload.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "`round = ...` replaces the generated workload, so `workload(...)` cannot be given with it",
        ));
    }
    if workload_rounds.is_some() && round_closure.is_none() && workload.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "`workload_rounds = ...` confines the streams named in `workload(...)` to the first rounds, so `workload(...)` must name the data streams (the streams it does not name are timers, fed in every round)",
        ));
    }
    for g in &generic_names {
        if !generic_types.iter().any(|(n, _)| n == g) {
            return Err(syn::Error::new(
                f.sig.generics.span(),
                format!("generic type parameter `{g}` needs a concrete type, for example `{g} = u64`"),
            ));
        }
    }
    let mut subst = Substitute { map: &generic_types };

    // Classify the parameters.
    let mut locs: Vec<Loc> = Vec::new();
    let mut streams: Vec<StreamParam> = Vec::new();
    let mut params: Vec<Param> = Vec::new();
    let find_or_add_loc = |kind: LocKind, marker: Type, param: Option<&Ident>, locs: &mut Vec<Loc>| -> usize {
        let key = type_key(&marker);
        if let Some(i) = locs.iter().position(|l| l.kind == kind && type_key(&l.marker) == key) {
            if let Some(p) = param
                && !locs[i].is_param
            {
                locs[i].var = p.clone();
                locs[i].is_param = true;
            }
            return i;
        }
        let var = param
            .cloned()
            .unwrap_or_else(|| format_ident!("__loc_{}", locs.len()));
        locs.push(Loc {
            kind,
            marker,
            var,
            is_param: param.is_some(),
        });
        locs.len() - 1
    };

    for arg in &f.sig.inputs {
        let FnArg::Typed(pt) = arg else {
            return Err(syn::Error::new(arg.span(), "#[amplification_check] does not support `self` parameters"));
        };
        let Pat::Ident(pi) = &*pt.pat else {
            return Err(syn::Error::new(pt.pat.span(), "#[amplification_check] needs every parameter to have a plain name"));
        };
        let pname = pi.ident.clone();
        let mut ty = (*pt.ty).clone();
        subst.visit_type_mut(&mut ty);

        if let Type::Reference(r) = &ty
            && let Some((kind, marker)) = as_location(&r.elem)?
        {
            let i = find_or_add_loc(kind, marker, Some(&pname), &mut locs);
            params.push(Param::Loc(i));
            continue;
        }
        if let Some((ident, pargs)) = last_ident(&ty)
            && (ident == "Stream" || ident == "KeyedStream")
        {
            let keyed = ident == "KeyedStream";
            let targs = type_args(pargs);
            // `Stream<T, Loc, Bound, Order, Retries>`; `KeyedStream<K, V, Loc, Bound, Order, Retries>`.
            let (item, rest): (Type, &[&Type]) = if keyed {
                if targs.len() < 3 {
                    return Err(syn::Error::new(ty.span(), "a KeyedStream parameter needs at least its key, value and location types"));
                }
                let (k, v) = (targs[0], targs[1]);
                (syn::parse_quote!((#k, #v)), &targs[2..])
            } else {
                if targs.len() < 2 {
                    return Err(syn::Error::new(ty.span(), "a Stream parameter needs at least its item and location types"));
                }
                (targs[0].clone(), &targs[1..])
            };
            let Some((kind, marker)) = as_location(rest[0])? else {
                return Err(syn::Error::new(rest[0].span(), "a stream parameter's location must be a Process or a Cluster"));
            };
            if let Some(bound) = rest.get(1)
                && last_ident(bound).map(|(i, _)| i.to_string()) != Some("Unbounded".to_owned())
            {
                return Err(syn::Error::new(bound.span(), "only Unbounded stream parameters can be fed by the harness"));
            }
            let order = rest.get(2).copied().cloned().unwrap_or_else(|| syn::parse_quote!(#hydro::live_collections::stream::TotalOrder));
            let retries = rest.get(3).copied().cloned().unwrap_or_else(|| syn::parse_quote!(#hydro::live_collections::stream::ExactlyOnce));
            let unordered = match last_ident(&order).map(|(i, _)| i.to_string()).as_deref() {
                Some("TotalOrder") => false,
                Some("NoOrder") => true,
                _ => {
                    return Err(syn::Error::new(order.span(), "a stream parameter's ordering must be TotalOrder or NoOrder (a generic ordering needs `O = TotalOrder` in the attribute)"));
                }
            };
            if last_ident(&retries).map(|(i, _)| i.to_string()).as_deref() != Some("ExactlyOnce") {
                return Err(syn::Error::new(retries.span(), "the simulator's input senders only feed ExactlyOnce streams; an AtLeastOnce stream parameter cannot be fed by the harness"));
            }
            let loc = find_or_add_loc(kind, marker, None, &mut locs);
            streams.push(StreamParam {
                name: pname,
                item,
                order,
                retries,
                loc,
                keyed,
                unordered,
            });
            params.push(Param::Stream(streams.len() - 1));
            continue;
        }
        params.push(Param::Plain(pname));
    }

    // Every plain parameter and every cluster needs a value from the attribute.
    let mut take_value = |n: &str, what: &str, span: Span| -> syn::Result<Expr> {
        match values.iter().position(|(k, _, _)| k == n) {
            Some(i) => Ok(values.remove(i).1),
            None => Err(syn::Error::new(span, format!("{what} `{n}` must be given in the attribute, as `{n} = ...`"))),
        }
    };
    let mut plain_values: Vec<(Ident, Expr)> = Vec::new();
    for p in &params {
        if let Param::Plain(n) = p {
            plain_values.push((n.clone(), take_value(&n.to_string(), "parameter", n.span())?));
        }
    }
    let mut cluster_sizes: Vec<(usize, Expr)> = Vec::new();
    for (i, l) in locs.iter().enumerate() {
        if l.kind == LocKind::Cluster {
            if !l.is_param {
                return Err(syn::Error::new(
                    f.sig.span(),
                    format!(
                        "a stream lives on a cluster ({}) that is not a parameter, so the harness cannot learn its size",
                        type_key(&l.marker)
                    ),
                ));
            }
            cluster_sizes.push((i, take_value(&l.var.to_string(), "cluster size for", l.var.span())?));
        }
    }
    if let Some((k, _, span)) = values.first() {
        return Err(syn::Error::new(*span, format!("`{k}` is not a parameter of `{fn_name}`")));
    }
    for (w, _) in &workload {
        if !streams.iter().any(|s| s.name == *w) {
            return Err(syn::Error::new(w.span(), format!("`{w}` is not a stream parameter of `{fn_name}`")));
        }
    }

    // ---- Code generation -----------------------------------------------------------------

    let amp = quote!(#cress);
    let config_name = name.clone().unwrap_or_else(|| "default".to_owned());
    let entry_name = match &name {
        Some(n) => format_ident!("__amplification_check_{}_{}", fn_name, n),
        None => format_ident!("__amplification_check_{}", fn_name),
    };
    let register_name = format_ident!("{}_register", entry_name);

    // Locations.
    let make_locs = locs.iter().map(|l| {
        let var = &l.var;
        let marker = &l.marker;
        match l.kind {
            LocKind::Process => quote!(let #var = __flow.process::<#marker>();),
            LocKind::Cluster => quote!(let #var = __flow.cluster::<#marker>();),
        }
    });
    let size_vars: Vec<(usize, Ident)> = cluster_sizes
        .iter()
        .map(|(i, _)| (*i, format_ident!("__members_{}", locs[*i].var)))
        .collect();
    let size_var = |loc: usize| size_vars.iter().find(|(i, _)| *i == loc).map(|(_, v)| v.clone());
    let make_sizes = cluster_sizes.iter().map(|(i, e)| {
        let v = size_var(*i).unwrap();
        quote!(let #v: usize = #e;)
    });

    // Inputs. A keyed stream parameter is fed as a stream of `(K, V)` pairs and converted with
    // `.into_keyed()`, which is a zero-cost cast.
    let tx = |s: &StreamParam| format_ident!("__tx_{}", s.name);
    let make_inputs = streams.iter().map(|s| {
        let txv = tx(s);
        let sv = &s.name;
        let (item, order, retries) = (&s.item, &s.order, &s.retries);
        let locv = &locs[s.loc].var;
        if s.keyed {
            quote!(
                let (#txv, #sv) = #locv.sim_input::<#item, #order, #retries>();
                let #sv = #sv.into_keyed();
            )
        } else {
            quote!(let (#txv, #sv) = #locv.sim_input::<#item, #order, #retries>();)
        }
    });

    // The call.
    let call_args = params.iter().map(|p| match p {
        Param::Loc(i) => {
            let v = &locs[*i].var;
            quote!(&#v)
        }
        Param::Stream(i) => {
            let v = &streams[*i].name;
            quote!(#v)
        }
        Param::Plain(n) => {
            let e = &plain_values.iter().find(|(k, _)| k == n).unwrap().1;
            quote!((#e))
        }
    });
    let sink = match &f.sig.output {
        ReturnType::Default => quote!(let _ = #fn_name(#(#call_args),*);),
        ReturnType::Type(..) => quote!(
            let __outputs = #fn_name(#(#call_args),*);
            #amp::SimOutputs::sink_all(__outputs);
        ),
    };

    // The simulation.
    let with_sizes = cluster_sizes.iter().map(|(i, _)| {
        let v = &locs[*i].var;
        let n = size_var(*i).unwrap();
        quote!(.with_cluster_size(&#v, #n))
    });
    let skip = if skip_consistency {
        quote!(.skip_consistency_assertions())
    } else {
        quote!()
    };
    let cfg = if let Some(h) = &horizon {
        quote!(#amp::CheckConfig::new(#h))
    } else {
        quote!(#amp::CheckConfig::default())
    };
    let cfg = if let Some(r) = &workload_rounds {
        quote!(#cfg.with_workload_rounds((#r) as usize))
    } else {
        cfg
    };

    // The workload: rates, loops, and the per-round closure.
    let rate_var = |s: &StreamParam| format_ident!("__rate_{}", s.name);
    let rates_var = |s: &StreamParam| format_ident!("__rates_{}", s.name);
    let make_rates = streams.iter().map(|s| {
        let rv = rates_var(s);
        let spec = workload.iter().find(|(w, _)| *w == s.name).map(|(_, e)| e);
        match spec {
            Some(Expr::Array(a)) => {
                let elems = a.elems.iter();
                quote!(let #rv: ::std::vec::Vec<usize> = ::std::vec![#(#elems as usize),*];)
            }
            Some(e) => quote!(let #rv: ::std::vec::Vec<usize> = ::std::vec![(#e) as usize];),
            None => quote!(let #rv: ::std::vec::Vec<usize> = ::std::vec![1usize];),
        }
    });
    let workload_fmt = {
        let mut parts: Vec<String> = Vec::new();
        let mut fmt_args: Vec<TokenStream2> = Vec::new();
        for (i, _) in &cluster_sizes {
            parts.push(format!("{}={{}} members", locs[*i].var));
            let n = size_var(*i).unwrap();
            fmt_args.push(quote!(#n));
        }
        if round_closure.is_some() {
            parts.push("round closure".to_owned());
        } else {
            for s in &streams {
                parts.push(format!("{}={{}}", s.name));
                let r = rate_var(s);
                fmt_args.push(quote!(#r));
            }
        }
        if let Some(r) = &workload_rounds {
            parts.push("data in rounds 0..{}".to_owned());
            fmt_args.push(quote!((#r) as usize));
        }
        let fmt = parts.join(", ");
        quote!(::std::format!(#fmt, #(#fmt_args),*))
    };

    let per_round = match &round_closure {
        None => {
            let sends = streams.iter().map(|s| {
                let txv = tx(s);
                let r = rate_var(s);
                let item = &s.item;
                // The simulator's sender has `send` for `TotalOrder` inputs and only
                // `send_many_unordered` for `NoOrder` ones.
                let send = match (locs[s.loc].kind, s.unordered) {
                    (LocKind::Process, false) => quote!(for _ in 0..#r { #txv.send(__counter.next_value::<#item>()); }),
                    (LocKind::Process, true) => quote!(for _ in 0..#r { #txv.send_many_unordered([__counter.next_value::<#item>()]); }),
                    (LocKind::Cluster, false) => {
                        let n = size_var(s.loc).unwrap();
                        quote!(for __m in 0..(#n as u32) { for _ in 0..#r { #txv.send(__m, __counter.next_value::<#item>()); } })
                    }
                    (LocKind::Cluster, true) => {
                        let n = size_var(s.loc).unwrap();
                        quote!(for __m in 0..(#n as u32) { for _ in 0..#r { #txv.send_many_unordered([(__m, __counter.next_value::<#item>())]); } })
                    }
                };
                // A stream named in `workload(...)` is data and stops after `workload_rounds`;
                // one not named is a timer and is fed in every round.
                let is_data = workload.iter().any(|(w, _)| *w == s.name);
                match (&workload_rounds, is_data) {
                    (Some(wr), true) => quote!(if __round < (#wr) as usize { #send }),
                    _ => send,
                }
            });
            quote!(
                let mut __counter = #amp::InputCounter::new();
                let __per_round = async |__round: usize| {
                    if __round == 0 { __counter.reset(); }
                    let _ = __round;
                    #(#sends)*
                };
            )
        }
        Some(closure) => {
            let fields = streams.iter().map(|s| {
                let n = &s.name;
                let (item, order, retries) = (&s.item, &s.order, &s.retries);
                match locs[s.loc].kind {
                    LocKind::Process => quote!(pub #n: #hydro::sim::SimSender<#item, #order, #retries>,),
                    LocKind::Cluster => quote!(pub #n: #hydro::sim::SimClusterSender<#item, #order, #retries>,),
                }
            });
            let member_fields = cluster_sizes.iter().map(|(i, _)| {
                let n = format_ident!("{}_members", locs[*i].var);
                quote!(pub #n: u32,)
            });
            let field_inits = streams.iter().map(|s| {
                let n = &s.name;
                let txv = tx(s);
                quote!(#n: #txv,)
            });
            let member_inits = cluster_sizes.iter().map(|(i, _)| {
                let n = format_ident!("{}_members", locs[*i].var);
                let v = size_var(*i).unwrap();
                quote!(#n: #v as u32,)
            });
            quote!(
                #[allow(non_camel_case_types, dead_code)]
                struct __Inputs { #(#fields)* #(#member_fields)* }
                fn __coerce<F: ::std::ops::FnMut(usize, &__Inputs)>(f: F) -> F { f }
                let __inputs = __Inputs { #(#field_inits)* #(#member_inits)* };
                let mut __user_round = __coerce(#closure);
                let __per_round = async |__round: usize| { __user_round(__round, &__inputs); };
            )
        }
    };

    // One check, wrapped in one loop per stream over its rates.
    let one_check = quote!(
        let mut __flow = #hydro::compile::builder::FlowBuilder::new();
        #(#make_locs)*
        #(#make_inputs)*
        #sink
        let __sim = __flow.sim() #(#with_sizes)* #skip;
        let __cfg = (#cfg).quiet();
        let __workload: ::std::string::String = #workload_fmt;
        #per_round
        let __report = #amp::check(__sim, &__cfg, __per_round);
        #amp::harness::record(&#amp::harness::Summary {
            crate_name: ::std::env!("CARGO_PKG_NAME"),
            manifest_dir: ::std::env!("CARGO_MANIFEST_DIR"),
            module: ::std::module_path!(),
            function: stringify!(#fn_name),
            configuration: #config_name,
            workload: __workload.clone(),
            report: &__report,
        });
        __results.push(#amp::CheckResult {
            function: stringify!(#fn_name),
            configuration: #config_name,
            workload: __workload,
            verdict: __report.verdict.clone(),
            growth: __report
                .growth
                .as_ref()
                .map(|growth| growth.word())
                .unwrap_or_else(|| "-".to_owned()),
            multiplier: __report
                .growth
                .as_ref()
                .map(|growth| match growth.multiplier_value() {
                    Some(multiplier) => ::std::format!("{multiplier:.2}x"),
                    None => "from 0".to_owned(),
                })
                .unwrap_or_else(|| "-".to_owned()),
            location: __report.location.as_ref().map(|location| #amp::readable_hook_name(&location.hook, false)),
        });
    );
    let mut body = one_check;
    if round_closure.is_none() {
        for s in streams.iter().rev() {
            let r = rate_var(s);
            let rs = rates_var(s);
            body = quote!(for &#r in &#rs { #body });
        }
    }
    let fn_name_str = fn_name.to_string();
    let doc = format!(
        "Generated by `#[amplification_check]`: runs the amplification checker on `{fn_name_str}` (configuration `{config_name}`)."
    );

    let ignore_value = ignore
        .as_ref()
        .map(|reason| quote!(::std::option::Option::Some(#reason)))
        .unwrap_or_else(|| quote!(::std::option::Option::None));

    Ok(quote!(
        #[doc = #doc]
        #[allow(non_snake_case, unused_variables, clippy::all)]
        fn #entry_name() -> ::std::vec::Vec<#amp::CheckResult> {
            use #hydro::location::Location as _;
            let mut __results = ::std::vec::Vec::new();
            #(#make_sizes)*
            #(#make_rates)*
            #body
            __results
        }

        #hydro::macro_support::ctor::declarative::ctor! {
            #[ctor(unsafe)]
            fn #register_name() {
                #amp::harness::register_check(#amp::harness::RegisteredCheck {
                    function: stringify!(#fn_name),
                    configuration: #config_name,
                    ignore: #ignore_value,
                    run: #entry_name,
                });
            }
        }
    ))
}

// ---------------------------------------------------------------------------------------------
// The derive

/// See the [crate documentation](crate).
#[proc_macro_derive(SimOutputs)]
pub fn derive_sim_outputs(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let cress: syn::Path = syn::parse_quote!(::cress);

    let syn::Data::Struct(data) = &input.data else {
        return syn::Error::new(input.span(), "#[derive(SimOutputs)] only supports structs")
            .to_compile_error()
            .into();
    };
    let (accessors, types): (Vec<TokenStream2>, Vec<&Type>) = match &data.fields {
        syn::Fields::Named(f) => f
            .named
            .iter()
            .map(|fld| {
                let id = fld.ident.as_ref().unwrap();
                (quote!(self.#id), &fld.ty)
            })
            .unzip(),
        syn::Fields::Unnamed(f) => f
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, fld)| {
                let idx = syn::Index::from(i);
                (quote!(self.#idx), &fld.ty)
            })
            .unzip(),
        syn::Fields::Unit => (Vec::new(), Vec::new()),
    };
    let bounds: Vec<TokenStream2> = types
        .iter()
        .map(|t| quote!(#t: #cress::SimOutputs))
        .collect();
    let where_tokens = match where_clause {
        Some(w) => {
            let preds = w.predicates.iter();
            quote!(where #(#preds,)* #(#bounds,)*)
        }
        None if !bounds.is_empty() => quote!(where #(#bounds,)*),
        None => quote!(),
    };

    quote!(
        #[automatically_derived]
        impl #impl_generics #cress::SimOutputs for #name #ty_generics #where_tokens {
            fn sink_all(self) {
                #(#cress::SimOutputs::sink_all(#accessors);)*
            }
        }
    )
    .into()
}
