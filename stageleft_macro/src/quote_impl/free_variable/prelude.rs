const PRELUDE: &[&str] = &["Err", "None", "Ok", "Some"];

pub fn is_prelude(ident: &syn::Ident) -> bool {
    let ident_str = ident.to_string();
    // Linear search is fast enough for four items.
    PRELUDE.contains(&ident_str.as_str())
}
