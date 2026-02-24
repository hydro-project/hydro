use proc_macro2::Span;
use syn::visit_mut::VisitMut;

pub struct UseTestModeStaged<'a> {
    pub crate_name: &'a str,
}

impl VisitMut for UseTestModeStaged<'_> {
    fn visit_type_path_mut(&mut self, i: &mut syn::TypePath) {
        if let Some(first) = i.path.segments.first()
            && first.ident == self.crate_name
            && let Some(second) = i.path.segments.get(1)
            && second.ident == "__staged"
        {
            i.path.segments.first_mut().unwrap().ident =
                syn::Ident::new("crate", Span::call_site());
        }

        syn::visit_mut::visit_type_path_mut(self, i);
    }

    fn visit_use_path_mut(&mut self, i: &mut syn::UsePath) {
        if i.ident == self.crate_name {
            i.ident = syn::Ident::new("crate", i.ident.span());
        }
    }

    fn visit_macro_mut(&mut self, i: &mut syn::Macro) {
        // Rewrite types inside __maybe_debug__!(Type) invocations.
        if i.path
            .segments
            .last()
            .is_some_and(|s| s.ident == "__maybe_debug__")
            && let Ok(mut ty) = i.parse_body::<syn::Type>()
        {
            self.visit_type_mut(&mut ty);
            i.tokens = quote::quote! { #ty };
        }

        syn::visit_mut::visit_macro_mut(self, i);
    }
}
