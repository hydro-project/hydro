use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::parse_quote;

use crate::{Datum, RelExpr, ScalarExpr};

pub(crate) fn generate_dataflow(r: RelExpr) -> String {
    let mut builder = SubgraphBuilder::new();
    let id = builder.compile_op(&r);
    let code = builder.code;
    prettyplease::unparse(&parse_quote! {
        fn main() {
            #(#code)*

            for row in #id {
                println!("{:?}", row);
            }
        }
    })
}

struct SubgraphBuilder {
    sym_id: usize,
    code: Vec<TokenStream>,
}

// TODO(justin): we can manually inline all this to just get raw rust code, but should check
// to see how much of that the rust compiler will do for us first.
impl ToTokens for ScalarExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            ScalarExpr::Literal(Datum::Int(x)) => quote! { ScalarExpr::Literal(Datum::Int(#x)) },
            ScalarExpr::ColRef(x) => quote! { ScalarExpr::ColRef(#x) },
            ScalarExpr::Eq(a, b) => quote! {
                ScalarExpr::Eq(Box::new(#a), Box::new(#b))
            },
            ScalarExpr::Plus(a, b) => quote! {
                ScalarExpr::Plus(Box::new(#a), Box::new(#b))
            },
            _ => panic!("unhandled"),
        })
    }
}

impl SubgraphBuilder {
    fn gensym(&mut self, prefix: &str) -> Ident {
        self.sym_id += 1;
        format_ident!("__{}_{}", prefix, self.sym_id)
    }

    fn new() -> Self {
        SubgraphBuilder {
            sym_id: 0,
            code: Vec::new(),
        }
    }

    fn compile_op(&mut self, r: &RelExpr) -> Ident {
        match r {
            RelExpr::Values(v) => {
                let op_name = self.gensym("values");
                let mut payload = TokenStream::default();
                let iter = v.iter().map(|row| {
                    quote! {
                        vec![ #(#row.eval(&Vec::new())),* ]
                    }
                });
                payload.extend(quote! {
                    vec![
                        #(
                            #iter
                        ),*
                    ]
                });

                self.code.push(quote! {
                    let #op_name = #payload.into_iter();
                });

                op_name
            }
            RelExpr::Filter(preds, input) => {
                let op_name = self.gensym("filter");
                let input_name = self.compile_op(input);

                let pred = quote! {
                    #(#preds.eval(row).is_true())&&*
                };

                self.code.push(quote! {
                    let #op_name = #input_name.filter(|row| #pred);
                });

                op_name
            }
            RelExpr::Project(exprs, input) => {
                let op_name = self.gensym("project");
                let input_name = self.compile_op(input);

                let pred = quote! {
                    vec![ #(#exprs.eval(&row)),* ]
                };

                self.code.push(quote! {
                    let #op_name = #input_name.map(|row| #pred);
                });

                op_name
            }
        }
    }
}
