use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::visit::Visit;

use self::free_variable::FreeVariableVisitor;

mod free_variable;

pub fn q_impl(root: TokenStream, expr: syn::Expr) -> TokenStream {
    let mut visitor = FreeVariableVisitor::default();
    visitor.visit_expr(&expr);

    let unitialized_free_variables = visitor.free_variables.iter().map(|i| {
        let mut i_without_span = i.clone();
        i_without_span.set_span(Span::call_site());
        let ident_str = i.to_string();
        quote!(
            #[allow(unused, non_upper_case_globals, non_snake_case)]
            let #i = {
                let _out = ::#root::runtime_support::FreeVariable::uninitialized(&#i_without_span);
                _vec_to_set.push((#ident_str.to_string(), ::#root::runtime_support::FreeVariable::to_tokens(#i_without_span)));
                _out
            };
        )
    });

    let uninit_forgets = visitor.free_variables.iter().map(|i| {
        let mut i_without_span = i.clone();
        i_without_span.set_span(Span::call_site());
        quote!(
            #[allow(unused, non_upper_case_globals, non_snake_case)]
            ::std::mem::forget(#i_without_span);
        )
    });

    // necessary to ensure proper hover in Rust Analyzer
    let expr_without_spans: syn::Expr =
        syn::parse_str(&expr.clone().into_token_stream().to_string()).unwrap();

    quote!({
        move |set_mod: &mut String, set_crate_name: &mut &'static str, set_tokens: &mut #root::internal::TokenStream, _vec_to_set: &mut #root::internal::CaptureVec, run: bool| {
            #(#unitialized_free_variables;)*

            *set_mod = module_path!().to_string();
            *set_crate_name = option_env!("STAGELEFT_FINAL_CRATE_NAME").unwrap_or(env!("CARGO_PKG_NAME"));
            *set_tokens = #root::internal::quote! {
                #expr_without_spans
            };

            if !run {
                #(#uninit_forgets;)*
                unsafe {
                    return ::std::mem::MaybeUninit::uninit().assume_init();
                }
            }

            #[allow(unreachable_code, unused_qualifications)]
            {
                #expr
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::q_impl;

    macro_rules! test_quote {
        ($program:expr) => {
            let quoted_tokens = q_impl(quote!(stageleft), parse_quote!($program));
            let wrapped: syn::File = parse_quote! {
                fn main() {
                    #quoted_tokens
                }
            };

            insta::with_settings!({snapshot_suffix => "macro_tokens"}, {
                insta::assert_snapshot!(
                    prettyplease::unparse(&wrapped)
                );
            });
        };
    }

    #[test]
    fn test_simple() {
        test_quote! {
            1 + 2
        }
    }

    #[test]
    fn test_capture_local() {
        test_quote! {
            x + 2
        }
    }

    #[test]
    fn test_capture_copy_local() {
        test_quote! {
            (x + 2) + (x + 2)
        }
    }

    #[test]
    fn test_capture_copy_local_block() {
        test_quote! {{
            let _ = x + 2;
            let _ = x + 2;
        }}
    }

    #[test]
    fn test_capture_copy_local_block_let() {
        test_quote! {{
            let x = x + 2;
            let _ = x + 2;
        }}
    }

    #[test]
    fn test_capture_local_mut() {
        test_quote! {
            x += 2
        }
    }

    #[test]
    fn test_non_capture_local() {
        test_quote! {
            {
                let x = 1;
                x + 2
            }
        }
    }

    #[test]
    fn test_capture_in_macro() {
        test_quote! {
            dbg!(x)
        }
    }

    #[test]
    fn test_non_capture_struct_creation() {
        test_quote! {
            Foo { x: 1 }
        }
    }

    #[test]
    fn test_non_capture_enum_creation() {
        test_quote! {
            Foo::Bar { x: 1 }
        }
    }

    #[test]
    fn test_prelude_enum_variants() {
        test_quote! {
            Some(1)
        }

        test_quote! {
            None
        }

        test_quote! {
            Ok(1)
        }

        test_quote! {
            Err(1)
        }
    }
}
