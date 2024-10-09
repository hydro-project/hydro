use proc_macro2::Span;
use syn::visit_mut::VisitMut;
use syn::{parse_quote, TypeInfer};

use crate::runtime_support::get_final_crate_name;

/// Rewrites use of alloc::string::* to use std::string::*
struct RewriteAlloc {
    mapping: Option<(String, String)>,
}

impl VisitMut for RewriteAlloc {
    fn visit_path_mut(&mut self, i: &mut syn::Path) {
        if i.segments.iter().take(1).collect::<Vec<_>>()
            == vec![&syn::PathSegment::from(syn::Ident::new(
                "alloc",
                Span::call_site(),
            ))]
        {
            *i.segments.first_mut().unwrap() =
                syn::PathSegment::from(syn::Ident::new("std", Span::call_site()));
        } else if i.segments.iter().take(3).collect::<Vec<_>>()
            == vec![
                &syn::PathSegment::from(syn::Ident::new("core", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("ops", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("range", Span::call_site())),
            ]
        {
            *i = syn::Path {
                leading_colon: i.leading_colon,
                segments: syn::punctuated::Punctuated::from_iter(
                    vec![
                        syn::PathSegment::from(syn::Ident::new("std", Span::call_site())),
                        syn::PathSegment::from(syn::Ident::new("ops", Span::call_site())),
                    ]
                    .into_iter()
                    .chain(i.segments.iter().skip(3).cloned()),
                ),
            };
        } else if i.segments.iter().take(3).collect::<Vec<_>>()
            == vec![
                &syn::PathSegment::from(syn::Ident::new("core", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("slice", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("iter", Span::call_site())),
            ]
        {
            *i = syn::Path {
                leading_colon: i.leading_colon,
                segments: syn::punctuated::Punctuated::from_iter(
                    vec![
                        syn::PathSegment::from(syn::Ident::new("std", Span::call_site())),
                        syn::PathSegment::from(syn::Ident::new("slice", Span::call_site())),
                    ]
                    .into_iter()
                    .chain(i.segments.iter().skip(3).cloned()),
                ),
            };
        } else if i.segments.iter().take(3).collect::<Vec<_>>()
            == vec![
                &syn::PathSegment::from(syn::Ident::new("core", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("iter", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("adapters", Span::call_site())),
            ]
        {
            *i = syn::Path {
                leading_colon: i.leading_colon,
                segments: syn::punctuated::Punctuated::from_iter(
                    vec![
                        syn::PathSegment::from(syn::Ident::new("std", Span::call_site())),
                        syn::PathSegment::from(syn::Ident::new("iter", Span::call_site())),
                    ]
                    .into_iter()
                    .chain(i.segments.iter().skip(4).cloned()),
                ),
            };
        } else if i.segments.iter().take(4).collect::<Vec<_>>()
            == vec![
                &syn::PathSegment::from(syn::Ident::new("std", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("collections", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("hash", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("map", Span::call_site())),
            ]
        {
            *i = syn::Path {
                leading_colon: i.leading_colon,
                segments: syn::punctuated::Punctuated::from_iter(
                    vec![
                        syn::PathSegment::from(syn::Ident::new("std", Span::call_site())),
                        syn::PathSegment::from(syn::Ident::new("collections", Span::call_site())),
                        syn::PathSegment::from(syn::Ident::new("hash_map", Span::call_site())),
                    ]
                    .into_iter()
                    .chain(i.segments.iter().skip(4).cloned()),
                ),
            };
        } else if i.segments.iter().take(3).collect::<Vec<_>>()
            == vec![
                &syn::PathSegment::from(syn::Ident::new("tokio", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("time", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("instant", Span::call_site())),
            ]
        {
            *i = syn::Path {
                leading_colon: i.leading_colon,
                segments: syn::punctuated::Punctuated::from_iter(
                    vec![
                        syn::PathSegment::from(syn::Ident::new("tokio", Span::call_site())),
                        syn::PathSegment::from(syn::Ident::new("time", Span::call_site())),
                    ]
                    .into_iter()
                    .chain(i.segments.iter().skip(3).cloned()),
                ),
            };
        } else if i.segments.iter().take(2).collect::<Vec<_>>()
            == vec![
                &syn::PathSegment::from(syn::Ident::new("bytes", Span::call_site())),
                &syn::PathSegment::from(syn::Ident::new("bytes", Span::call_site())),
            ]
        {
            *i = syn::Path {
                leading_colon: i.leading_colon,
                segments: syn::punctuated::Punctuated::from_iter(
                    vec![syn::PathSegment::from(syn::Ident::new(
                        "bytes",
                        Span::call_site(),
                    ))]
                    .into_iter()
                    .chain(i.segments.iter().skip(2).cloned()),
                ),
            };
        } else if let Some((macro_name, final_name)) = &self.mapping {
            if i.segments.first().unwrap().ident == macro_name {
                *i.segments.first_mut().unwrap() =
                    syn::parse2(get_final_crate_name(final_name)).unwrap();

                i.segments.insert(1, parse_quote!(__staged));
            }
        }

        syn::visit_mut::visit_path_mut(self, i);
    }
}

struct ElimClosureToInfer;

impl VisitMut for ElimClosureToInfer {
    fn visit_type_mut(&mut self, i: &mut syn::Type) {
        if let syn::Type::Path(p) = i {
            if p.path
                .segments
                .iter()
                .any(|s| s.ident == "CLOSURE_TO_INFER")
            {
                *i = syn::Type::Infer(TypeInfer {
                    underscore_token: Default::default(),
                });
                return;
            }
        }

        syn::visit_mut::visit_type_mut(self, i);
    }
}

/// Captures a fully qualified path to a given type, which is useful when
/// the generated code needs to explicitly refer to a type known at staging time.
///
/// This API is fairly experimental, and comes with caveats. For example, it cannot
/// handle closure types. In addition, when a user refers to a re-exported type,
/// the original type path may be returned here, which could involve private modules.
///
/// Also, users must be careful to ensure that any crates referred in the type are
/// available where it is spliced.
pub fn quote_type<T>() -> syn::Type {
    let name = std::any::type_name::<T>().replace("{{closure}}", "CLOSURE_TO_INFER");
    let mut t_type: syn::Type = syn::parse_str(&name).unwrap_or_else(|_| {
        panic!("Could not parse type name: {}", name);
    });
    let mapping = super::runtime_support::MACRO_TO_CRATE.with(|m| m.borrow().clone());
    ElimClosureToInfer.visit_type_mut(&mut t_type);
    RewriteAlloc { mapping }.visit_type_mut(&mut t_type);

    t_type
}
