fn main() {
    #[cfg(feature = "test_embedded")]
    generate_embedded();
}

#[cfg(feature = "test_embedded")]
fn generate_embedded() {
    println!("cargo::rerun-if-changed=build.rs");

    let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
    let process = flow.process::<()>();
    hydro_test::local::first_ten::first_ten(&process);

    let code = flow
        .with_process(&process, "first_ten")
        .generate_embedded("hydro_test");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = format!("{out_dir}/embedded.rs");
    std::fs::write(&out_path, prettyplease::unparse(&code)).unwrap();
}
