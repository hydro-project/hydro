use std::collections::BTreeSet;
use std::path::Path;
use std::{env, fs};

use proc_macro2::Span;
use quote::ToTokens;
use sha2::{Digest, Sha256};
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{UsePath, Visibility, parse_quote};

struct GenMacroVistor {
    exported_macros: BTreeSet<(String, String)>,
    current_mod: syn::Path,
}

// marks everything as pub(crate) because proc-macros cannot actually export anything
impl<'a> Visit<'a> for GenMacroVistor {
    fn visit_item_mod(&mut self, i: &'a syn::ItemMod) {
        let old_mod = self.current_mod.clone();
        let i_ident = &i.ident;
        self.current_mod = parse_quote!(#old_mod::#i_ident);

        syn::visit::visit_item_mod(self, i);

        self.current_mod = old_mod;
    }

    fn visit_item_fn(&mut self, i: &'a syn::ItemFn) {
        let is_entry = i
            .attrs
            .iter()
            .any(|a| a.path().to_token_stream().to_string() == "stageleft :: entry");

        if is_entry {
            let cur_path = &self.current_mod;
            let mut i_cloned = i.clone();
            i_cloned.attrs = vec![];
            i_cloned.vis = Visibility::Inherited; // normalize pub

            let contents = i_cloned
                .to_token_stream()
                .to_string()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>();
            let contents_hash = format!("{:X}", Sha256::digest(contents));
            self.exported_macros
                .insert((contents_hash, cur_path.to_token_stream().to_string()));
        }
    }
}

pub fn gen_macro(staged_path: &Path, crate_name: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("lib_macro.rs");

    let flow_lib =
        syn_inline_mod::parse_and_inline_modules(&staged_path.join("src").join("lib.rs"));
    let mut visitor = GenMacroVistor {
        exported_macros: Default::default(),
        current_mod: parse_quote!(crate),
    };
    visitor.visit_file(&flow_lib);

    let staged_path_absolute = fs::canonicalize(staged_path).unwrap();

    let mut out_file: syn::File = parse_quote!();

    for (hash, exported_from) in visitor.exported_macros {
        let underscored_path = syn::Ident::new(&("macro_".to_string() + &hash), Span::call_site());
        let underscored_path_impl =
            syn::Ident::new(&("macro_".to_string() + &hash + "_impl"), Span::call_site());
        let exported_from_parsed: syn::Path = syn::parse_str(&exported_from).unwrap();

        let proc_macro_wrapper: syn::ItemFn = parse_quote!(
            #[proc_macro]
            #[expect(unused_qualifications, non_snake_case, reason = "generated code")]
            pub fn #underscored_path(input: ::proc_macro::TokenStream) -> ::proc_macro::TokenStream {
                let input = ::stageleft::internal::TokenStream::from(input);
                let out = #exported_from_parsed::#underscored_path_impl(input);
                ::proc_macro::TokenStream::from(out)
            }
        );

        out_file.items.push(syn::Item::Fn(proc_macro_wrapper));
    }

    fs::write(dest_path, prettyplease::unparse(&out_file)).unwrap();

    println!("cargo::rustc-check-cfg=cfg(stageleft_macro)");
    println!("cargo::rustc-check-cfg=cfg(stageleft_runtime)");
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-env=STAGELEFT_FINAL_CRATE_NAME={}", crate_name);
    println!("cargo::rustc-cfg=stageleft_macro");

    println!(
        "cargo::rerun-if-changed={}",
        staged_path_absolute.to_string_lossy()
    );
}

struct InlineTopLevelMod {}

impl VisitMut for InlineTopLevelMod {
    fn visit_file_mut(&mut self, i: &mut syn::File) {
        i.attrs = vec![];
        i.items.iter_mut().for_each(|i| {
            if let syn::Item::Macro(e) = i {
                if e.mac.path.to_token_stream().to_string() == "stageleft :: top_level_mod" {
                    let inner = &e.mac.tokens;
                    *i = parse_quote!(
                        pub mod #inner;
                    );
                }
            }
        });
    }
}

struct GenFinalPubVistor {
    current_mod: Option<syn::Path>,
    test_mode: bool,
}

fn get_cfg_attrs(attrs: &[syn::Attribute]) -> impl Iterator<Item = &syn::Attribute> + '_ {
    attrs.iter().filter(|attr| attr.path().is_ident("cfg"))
}

fn is_runtime(attrs: &[syn::Attribute]) -> bool {
    get_cfg_attrs(attrs)
        .any(|attr| attr.to_token_stream().to_string() == "# [cfg (stageleft_runtime)]")
}

impl VisitMut for GenFinalPubVistor {
    fn visit_item_enum_mut(&mut self, i: &mut syn::ItemEnum) {
        i.vis = parse_quote!(pub);
        syn::visit_mut::visit_item_enum_mut(self, i);
    }

    fn visit_variant_mut(&mut self, _i: &mut syn::Variant) {
        // variant fields do not have visibility modifiers
    }

    fn visit_item_struct_mut(&mut self, i: &mut syn::ItemStruct) {
        i.vis = parse_quote!(pub);
        syn::visit_mut::visit_item_struct_mut(self, i);
    }

    fn visit_field_mut(&mut self, i: &mut syn::Field) {
        i.vis = parse_quote!(pub);
        syn::visit_mut::visit_field_mut(self, i);
    }

    fn visit_item_use_mut(&mut self, i: &mut syn::ItemUse) {
        if is_runtime(&i.attrs) {
            i.attrs.insert(
                0,
                parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
            );
            return;
        }

        i.vis = parse_quote!(pub);
        syn::visit_mut::visit_item_use_mut(self, i);
    }

    fn visit_use_path_mut(&mut self, i: &mut UsePath) {
        if i.ident == "crate" {
            i.tree = Box::new(syn::UseTree::Path(UsePath {
                ident: parse_quote!(__staged),
                colon2_token: Default::default(),
                tree: i.tree.clone(),
            }));
        }

        syn::visit_mut::visit_use_path_mut(self, i);
    }

    fn visit_vis_restricted_mut(&mut self, _i: &mut syn::VisRestricted) {
        // don't treat the restriction as a path, we don't want to rewrite that to `__staged`
    }

    fn visit_path_mut(&mut self, i: &mut syn::Path) {
        if !i.segments.is_empty() && i.segments[0].ident == "crate" {
            i.segments.insert(
                1,
                syn::PathSegment {
                    ident: parse_quote!(__staged),
                    arguments: Default::default(),
                },
            );
        }
    }

    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        let is_test_mod = i
            .attrs
            .iter()
            .any(|a| a.to_token_stream().to_string() == "# [cfg (test)]");

        if is_runtime(&i.attrs) {
            // no quoted code depends on this module, so we do not need to copy it
            i.attrs.insert(
                0,
                parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
            );

            return;
        } else if is_test_mod {
            i.attrs
                .retain(|a| a.to_token_stream().to_string() != "# [cfg (test)]");

            if !self.test_mode {
                // if test mode is not true, there are no quoted snippets behind #[cfg(test)],
                // so no #[cfg(test)] modules will ever be reachable
                i.attrs.insert(
                    0,
                    parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
                );
            }
        }

        let old_mod = self.current_mod.clone();
        let i_ident = &i.ident;
        self.current_mod = self
            .current_mod
            .as_ref()
            .map(|old_mod| parse_quote!(#old_mod::#i_ident));

        i.vis = parse_quote!(pub);

        syn::visit_mut::visit_item_mod_mut(self, i);

        self.current_mod = old_mod;
    }

    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        if is_runtime(&i.attrs) {
            i.attrs.insert(
                0,
                parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
            );
            return;
        }

        let is_ctor = i
            .attrs
            .iter()
            .any(|a| a.path().to_token_stream().to_string() == "ctor :: ctor");

        let is_test = i.attrs.iter().any(|a| {
            a.path().to_token_stream().to_string() == "test"
                || a.path().to_token_stream().to_string() == "tokio :: test"
        });

        if is_ctor || is_test {
            // don't want ctors or tests to be leaked into copied code
            i.attrs.insert(
                0,
                parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
            );
        }

        i.vis = parse_quote!(pub);

        syn::visit_mut::visit_item_fn_mut(self, i);
    }

    fn visit_item_mut(&mut self, i: &mut syn::Item) {
        // TODO(shadaj): warn if a pub struct or enum has private fields
        // and is not marked for runtime
        if let Some(cur_path) = self.current_mod.as_ref() {
            if let syn::Item::Struct(e) = i {
                if is_runtime(&e.attrs) {
                    e.attrs.insert(
                        0,
                        parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
                    );
                    return;
                }

                if matches!(e.vis, Visibility::Public(_)) {
                    let e_name = &e.ident;
                    let e_attrs = get_cfg_attrs(&e.attrs);
                    *i = parse_quote!(#(#e_attrs)* pub use #cur_path::#e_name;);
                    return;
                }
            } else if let syn::Item::Enum(e) = i {
                if is_runtime(&e.attrs) {
                    e.attrs.insert(
                        0,
                        parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
                    );
                    return;
                }

                if matches!(e.vis, Visibility::Public(_)) {
                    let e_name = &e.ident;
                    let e_attrs = get_cfg_attrs(&e.attrs);
                    *i = parse_quote!(#(#e_attrs)* pub use #cur_path::#e_name;);
                    return;
                }
            } else if let syn::Item::Trait(e) = i {
                if is_runtime(&e.attrs) {
                    e.attrs.insert(
                        0,
                        parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
                    );
                    return;
                }

                if matches!(e.vis, Visibility::Public(_)) {
                    let e_name = &e.ident;
                    let e_attrs = get_cfg_attrs(&e.attrs);
                    *i = parse_quote!(#(#e_attrs)* pub use #cur_path::#e_name;);
                    return;
                }
            } else if let syn::Item::Static(e) = i {
                if is_runtime(&e.attrs) {
                    e.attrs.insert(
                        0,
                        parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
                    );
                    return;
                }

                if matches!(e.vis, Visibility::Public(_)) {
                    let e_name = &e.ident;
                    let e_attrs = get_cfg_attrs(&e.attrs);
                    *i = parse_quote!(#(#e_attrs)* pub use #cur_path::#e_name;);
                    return;
                }
            } else if let syn::Item::Const(e) = i {
                if is_runtime(&e.attrs) {
                    e.attrs.insert(
                        0,
                        parse_quote!(#[cfg(all(stageleft_macro, not(stageleft_macro)))]),
                    );
                    return;
                }

                if matches!(e.vis, Visibility::Public(_)) {
                    let e_name = &e.ident;
                    let e_attrs = get_cfg_attrs(&e.attrs);
                    *i = parse_quote!(#(#e_attrs)* pub use #cur_path::#e_name;);
                    return;
                }
            } else if let syn::Item::Impl(e) = i {
                // TODO(shadaj): emit impls if the struct is private
                // currently, we just skip all impls
                *i = parse_quote!(
                    #[cfg(all(stageleft_macro, not(stageleft_macro)))]
                    #e
                );
            }
        }

        syn::visit_mut::visit_item_mut(self, i);
    }

    fn visit_file_mut(&mut self, i: &mut syn::File) {
        i.attrs = vec![];
        i.items.retain(|i| match i {
            syn::Item::Macro(m) => {
                m.mac.path.to_token_stream().to_string() != "stageleft :: stageleft_crate"
                    && m.mac.path.to_token_stream().to_string()
                        != "stageleft :: stageleft_no_entry_crate"
            }
            _ => true,
        });

        syn::visit_mut::visit_file_mut(self, i);
    }
}

pub fn gen_staged_trybuild(lib_path: &Path, orig_crate_name: String, test_mode: bool) -> syn::File {
    let mut orig_flow_lib = syn_inline_mod::parse_and_inline_modules(lib_path);
    InlineTopLevelMod {}.visit_file_mut(&mut orig_flow_lib);

    let mut flow_lib_pub = syn_inline_mod::parse_and_inline_modules(lib_path);

    let orig_crate_ident = syn::Ident::new(&orig_crate_name, Span::call_site());
    let mut final_pub_visitor = GenFinalPubVistor {
        current_mod: Some(parse_quote!(#orig_crate_ident)),
        test_mode,
    };
    final_pub_visitor.visit_file_mut(&mut flow_lib_pub);

    flow_lib_pub
}

pub fn gen_final_helper() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let mut orig_flow_lib = syn_inline_mod::parse_and_inline_modules(Path::new("src/lib.rs"));
    InlineTopLevelMod {}.visit_file_mut(&mut orig_flow_lib);

    let mut flow_lib_pub = syn_inline_mod::parse_and_inline_modules(Path::new("src/lib.rs"));

    let mut final_pub_visitor = GenFinalPubVistor {
        current_mod: Some(parse_quote!(crate)),
        test_mode: false,
    };
    final_pub_visitor.visit_file_mut(&mut flow_lib_pub);

    fs::write(
        Path::new(&out_dir).join("lib_pub.rs"),
        prettyplease::unparse(&flow_lib_pub),
    )
    .unwrap();

    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=src");
}

#[macro_export]
macro_rules! gen_final {
    () => {
        #[allow(
            unexpected_cfgs,
            reason = "Consumer crates may optionally add the `stageleft_devel` feature."
        )]
        {
            println!("cargo::rustc-check-cfg=cfg(stageleft_macro)");
            println!("cargo::rustc-check-cfg=cfg(stageleft_runtime)");
            println!("cargo::rustc-cfg=stageleft_runtime");

            #[cfg(not(feature = "stageleft_devel"))]
            $crate::gen_final_helper()
        }
    };
}
