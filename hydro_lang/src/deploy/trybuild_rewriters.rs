use syn::visit_mut::VisitMut;

pub struct ReplaceCrateNameWithStaged {
    pub crate_name: String,
}

impl VisitMut for ReplaceCrateNameWithStaged {
    fn visit_type_path_mut(&mut self, i: &mut syn::TypePath) {
        if let Some(first) = i.path.segments.first() {
            if first.ident == self.crate_name {
                let tail = i.path.segments.iter().skip(1).collect::<Vec<_>>();
                *i = syn::parse_quote!(crate::__staged #(::#tail)*);
            }
        }

        syn::visit_mut::visit_type_path_mut(self, i);
    }
}

pub struct ReplaceCrateWithOrig {
    pub crate_name: String,
}

impl VisitMut for ReplaceCrateWithOrig {
    fn visit_item_use_mut(&mut self, i: &mut syn::ItemUse) {
        if let syn::UseTree::Path(p) = &mut i.tree {
            if p.ident == "crate" {
                p.ident = syn::Ident::new(&self.crate_name, p.ident.span());
                i.leading_colon = Some(Default::default());
            }
        }

        syn::visit_mut::visit_item_use_mut(self, i);
    }
}
