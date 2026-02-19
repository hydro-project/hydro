fn main() {
    #[cfg(feature = "test_embedded")]
    generate_embedded();
}

#[cfg(feature = "test_embedded")]
fn generate_embedded() {
    use hydro_lang::location::Location;

    println!("cargo::rerun-if-changed=build.rs");

    let out_dir = std::env::var("OUT_DIR").unwrap();

    // --- capitalize (local, no networking) ---
    {
        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let process = flow.process::<()>();
        hydro_test::local::capitalize::capitalize(process.embedded_input("input"));

        let code = flow
            .with_process(&process, "capitalize")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/embedded.rs"),
            prettyplease::unparse(&code),
        )
        .unwrap();
    }

    // --- echo_network (o2o networking) ---
    {
        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let sender = flow.process::<hydro_test::embedded::echo_network::Sender>();
        let receiver = flow.process::<hydro_test::embedded::echo_network::Receiver>();
        hydro_test::embedded::echo_network::echo_network(&receiver, sender.embedded_input("input"))
            .embedded_output("output");

        let code = flow
            .with_process(&sender, "echo_sender")
            .with_process(&receiver, "echo_receiver")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/echo_network.rs"),
            prettyplease::unparse(&code),
        )
        .unwrap();
    }
}
