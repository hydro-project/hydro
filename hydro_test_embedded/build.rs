fn main() {
    #[cfg(feature = "test_embedded")]
    generate_embedded();
}

#[cfg(feature = "test_embedded")]
fn generate_embedded() {
    use hydro_lang::location::Location;
    use hydro_lang::prelude::nondet;

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

    // --- o2m_broadcast (process -> cluster) ---
    {
        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let process = flow.process::<hydro_test::embedded::o2m_broadcast::Src>();
        let cluster = flow.cluster::<hydro_test::embedded::o2m_broadcast::Dst>();
        hydro_test::embedded::o2m_broadcast::o2m_broadcast(
            &cluster,
            process.embedded_input("input"),
        )
        .assume_ordering(nondet!(/** test */))
        .embedded_output("output");

        let code = flow
            .with_process(&process, "o2m_sender")
            .with_cluster(&cluster, "o2m_receiver")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/o2m_broadcast.rs"),
            prettyplease::unparse(&code),
        )
        .unwrap();
    }

    // --- m2o_send (cluster -> process) ---
    {
        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let cluster = flow.cluster::<hydro_test::embedded::m2o_send::Src>();
        let process = flow.process::<hydro_test::embedded::m2o_send::Dst>();
        hydro_test::embedded::m2o_send::m2o_send(&process, cluster.embedded_input("input"))
            .assume_ordering(nondet!(/** test */))
            .embedded_output("output");

        let code = flow
            .with_cluster(&cluster, "m2o_sender")
            .with_process(&process, "m2o_receiver")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/m2o_send.rs"),
            prettyplease::unparse(&code),
        )
        .unwrap();
    }

    // --- m2m_broadcast (cluster -> cluster) ---
    {
        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let src = flow.cluster::<hydro_test::embedded::m2m_broadcast::Src>();
        let dst = flow.cluster::<hydro_test::embedded::m2m_broadcast::Dst>();
        hydro_test::embedded::m2m_broadcast::m2m_broadcast(&dst, src.embedded_input("input"))
            .assume_ordering(nondet!(/** test */))
            .embedded_output("output");

        let code = flow
            .with_cluster(&src, "m2m_sender")
            .with_cluster(&dst, "m2m_receiver")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/m2m_broadcast.rs"),
            prettyplease::unparse(&code),
        )
        .unwrap();
    }
}
