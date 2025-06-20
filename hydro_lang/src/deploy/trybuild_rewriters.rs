use syn::visit_mut::VisitMut;

pub struct ReplaceCrateNameWithStaged {
    pub crate_name: String,
    pub is_test: bool,
}

impl VisitMut for ReplaceCrateNameWithStaged {
    fn visit_type_path_mut(&mut self, i: &mut syn::TypePath) {
        if let Some(first) = i.path.segments.first() {
            if first.ident == self.crate_name {
                let tail = i.path.segments.iter().skip(1).collect::<Vec<_>>();

                if let Some(first) = tail.first() {
                    if first.ident == "__staged" {
                        // If the first segment is already `staged`, we do not need to change it.
                        return;
                    }
                }

                if self.is_test {
                    *i = syn::parse_quote!(crate::__staged #(::#tail)*);
                } else {
                    let crate_ident = syn::Ident::new(&self.crate_name, first.ident.span());
                    *i = syn::parse_quote!(#crate_ident::__staged #(::#tail)*);
                }
            }
        }

        syn::visit_mut::visit_type_path_mut(self, i);
    }

    fn visit_use_path_mut(&mut self, i: &mut syn::UsePath) {
        if i.ident == "crate" && !self.is_test {
            i.ident = syn::Ident::new(&self.crate_name, i.ident.span());
        }
    }
}
