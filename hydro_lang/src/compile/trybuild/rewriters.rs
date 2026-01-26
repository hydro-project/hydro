use proc_macro2::Span;
use syn::visit_mut::VisitMut;

pub struct UseTestModeStaged<'a> {
    pub crate_name: &'a str,
}

impl VisitMut for UseTestModeStaged<'_> {
    fn visit_type_path_mut(&mut self, i: &mut syn::TypePath) {
        if let Some(first) = i.path.segments.first_mut()
            && first.ident == self.crate_name
        {
            first.ident = syn::Ident::new("crate", Span::call_site());
        }

        syn::visit_mut::visit_type_path_mut(self, i);
    }

    fn visit_use_path_mut(&mut self, i: &mut syn::UsePath) {
        if i.ident == self.crate_name {
            i.ident = syn::Ident::new("crate", i.ident.span());
        }
    }
}
