#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};

#[derive(Clone, Copy)]
enum Platform {
    Default,
    Tokio,
    AsyncStd,
    Hydroflow,
    Wasm,
    EnvLogging,
    EnvTracing,
}
impl Platform {
    // All platforms.
    const ALL: [Self; 7] = [
        Self::Default,
        Self::Tokio,
        Self::AsyncStd,
        Self::Hydroflow,
        Self::Wasm,
        Self::EnvLogging,
        Self::EnvTracing,
    ];
    // Default when no platforms are specified.
    const DEFAULT: [Self; 2] = [Self::Default, Self::Wasm];

    /// Name of platform ident in attribute.
    const fn name(self) -> &'static str {
        match self {
            Self::Default => "test",
            Self::Tokio => "tokio",
            Self::AsyncStd => "async_std",
            Self::Hydroflow => "hydroflow",
            Self::Wasm => "wasm",
            Self::EnvLogging => "env_logging",
            Self::EnvTracing => "env_tracing",
        }
    }

    /// Generate the attribute for this platform (if any).
    fn make_attribute(self) -> proc_macro2::TokenStream {
        // Fully specify crate names so that the consumer does not need to add another
        // use statement. They still need to depend on the crate in their `Cargo.toml`,
        // though.
        // TODO(mingwei): use `proc_macro_crate::crate_name(...)` to handle renames.
        match self {
            Platform::Default => quote! { #[test] },
            Platform::Tokio => quote! { #[tokio::test ] },
            Platform::AsyncStd => quote! { #[async_std::test] },
            Platform::Hydroflow => quote! { #[hydroflow::test] },
            Platform::Wasm => {
                quote! { #[wasm_bindgen_test::wasm_bindgen_test] }
            }
            Platform::EnvLogging | Platform::EnvTracing => Default::default(),
        }
    }

    /// Generate the initialization code statements for this platform (if any).
    fn make_init_code(self) -> proc_macro2::TokenStream {
        match self {
            Platform::EnvLogging => quote! {
                let _ = env_logger::builder().is_test(true).try_init();
            },
            Platform::EnvTracing => quote! {
                let subscriber = tracing_subscriber::FmtSubscriber::builder()
                    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                    .with_test_writer()
                    .finish();
                let _ = tracing::subscriber::set_global_default(subscriber);
            },
            _ => Default::default(),
        }
    }
}

/// See the [crate] docs for usage information.
#[proc_macro_attribute]
pub fn multiplatform_test(attr: TokenStream, body: TokenStream) -> TokenStream {
    let ts = multiplatform_test_impl(
        proc_macro2::TokenStream::from(attr),
        proc_macro2::TokenStream::from(body),
    );
    TokenStream::from(ts)
}

fn multiplatform_test_impl(
    attr: proc_macro2::TokenStream,
    body: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let mut attr = attr.into_iter();
    let mut platforms = Vec::<Platform>::new();

    while let Some(token) = attr.next() {
        let proc_macro2::TokenTree::Ident(i) = &token else {
            return quote_spanned! {token.span()=>
                compile_error!("malformed #[multiplatform_test] attribute; expected identifier.");
            };
        };
        let name = i.to_string();
        let Some(&platform) = Platform::ALL
            .iter()
            .find(|platform| name == platform.name())
        else {
            let msg = proc_macro2::Literal::string(&format!(
                "unknown platform {}; expected one of [{}]",
                name,
                Platform::ALL.map(Platform::name).join(", "),
            ));
            return quote_spanned! {token.span()=> compile_error!(#msg); };
        };
        platforms.push(platform);

        match &attr.next() {
            Some(proc_macro2::TokenTree::Punct(op)) if op.as_char() == ',' => {}
            Some(other) => {
                return quote_spanned! {other.span()=>
                    compile_error!("malformed `#[multiplatform_test]` attribute; expected `,`.");
                };
            }
            None => break,
        }
    }
    if platforms.is_empty() {
        platforms.extend(Platform::DEFAULT.iter());
    }

    let mut output = proc_macro2::TokenStream::new();
    let mut init_code = proc_macro2::TokenStream::new();

    for p in platforms {
        output.extend(p.make_attribute());
        init_code.extend(p.make_init_code());
    }

    if init_code.is_empty() {
        output.extend(body);
    } else {
        let mut body_head = body.into_iter().collect::<Vec<_>>();
        let Some(proc_macro2::TokenTree::Group(body_code)) = body_head.pop() else {
            panic!();
        };

        output.extend(body_head);
        output.extend(quote! {
            {
                { #init_code };
                #body_code
            }
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_platforms() {
        let test_fn: proc_macro2::TokenStream = quote! { fn test() { } };
        let attrs = proc_macro2::TokenStream::new();
        let got: proc_macro2::TokenStream = multiplatform_test_impl(attrs, test_fn);
        let want = quote! {
            #[test]
            #[wasm_bindgen_test::wasm_bindgen_test]
            fn test() { }
        };

        assert_eq!(want.to_string(), got.to_string());
    }

    #[test]
    fn test_host_platform() {
        let test_fn = quote! { fn test() { } };
        let attrs = quote! { test };
        let got = multiplatform_test_impl(attrs, test_fn);
        let want = quote! {
            #[test]
            fn test() { }
        };

        assert_eq!(want.to_string(), got.to_string());
    }

    #[test]
    fn test_wasm_platform() {
        let test_fn = quote! { fn test() { } };
        let attrs = quote! { wasm };
        let got = multiplatform_test_impl(attrs, test_fn);
        let want = quote! {
            #[wasm_bindgen_test::wasm_bindgen_test]
            fn test() { }
        };

        assert_eq!(want.to_string(), got.to_string());
    }

    #[test]
    fn test_host_wasm_platform() {
        let test_fn = quote! { fn test() { } };
        let attrs = quote! { test, wasm };
        let got = multiplatform_test_impl(attrs, test_fn);
        let want = quote! {
            #[test]
            #[wasm_bindgen_test::wasm_bindgen_test]
            fn test() { }
        };

        assert_eq!(want.to_string(), got.to_string());
    }

    #[test]
    fn test_unknown_platform() {
        let test_fn = quote! { fn test() { } };
        let attrs = quote! { hello };
        let got = multiplatform_test_impl(attrs, test_fn);
        assert!(got.to_string().starts_with("compile_error !"));
    }

    #[test]
    fn test_invalid_attr_nocomma_platform() {
        let test_fn = quote! { fn test() { } };
        let attrs = quote! { wasm() };
        let got = multiplatform_test_impl(attrs, test_fn);
        assert!(got.to_string().starts_with("compile_error !"));
    }

    #[test]
    fn test_invalid_attr_noident_platform() {
        let test_fn = quote! { fn test() { } };
        let attrs = quote! { () };
        let got = multiplatform_test_impl(attrs, test_fn);
        assert!(got.to_string().starts_with("compile_error !"));
    }
}
