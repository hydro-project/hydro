fn main() {
    #[cfg(feature = "test_embedded")]
    generate_embedded();
}

#[cfg(feature = "test_embedded")]
fn generate_embedded() {
    use hydro_lang::location::Location;

    println!("cargo::rerun-if-changed=build.rs");

    let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
    let process = flow.process::<()>();
    let input = process.embedded_input::<String>("input");
    hydro_test::local::capitalize::capitalize(input);

    let code = flow
        .with_process(&process, "capitalize")
        .generate_embedded("hydro_test");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = format!("{out_dir}/embedded.rs");
    std::fs::write(&out_path, prettyplease::unparse(&code)).unwrap();
}
