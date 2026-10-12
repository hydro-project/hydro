fn main() {
    hydro_build_utils::emit_nightly_configuration!();

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

    // --- unordered_output (local; the dataflow assumes nothing of its input, and provides its
    // output unordered, at least once) ---
    {
        use hydro_lang::live_collections::boundedness::Unbounded;
        use hydro_lang::live_collections::stream::{AtLeastOnce, NoOrder};

        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let process = flow.process::<()>();
        process
            .embedded_input::<String, Unbounded, NoOrder, AtLeastOnce>("input")
            .embedded_output("output");

        let code = flow
            .with_process(&process, "unordered_echo")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/unordered_output.rs"),
            prettyplease::unparse(&code),
        )
        .unwrap();
    }

    // --- bounded_output (local; the dataflow assumes its input is complete from the start, so
    // its output is too) ---
    {
        use hydro_lang::live_collections::boundedness::Bounded;
        use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};

        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let process = flow.process::<()>();
        process
            .embedded_input::<String, Bounded, TotalOrder, ExactlyOnce>("input")
            .embedded_output("output");

        let code = flow
            .with_process(&process, "bounded_echo")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/bounded_output.rs"),
            prettyplease::unparse(&code),
        )
        .unwrap();
    }

    // --- duplicable_channel (o2o; the stream sent may already repeat messages) ---
    {
        use hydro_lang::live_collections::boundedness::Unbounded;
        use hydro_lang::live_collections::stream::{AtLeastOnce, TotalOrder};
        use hydro_lang::networking::EMBEDDED;

        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let sender = flow.process::<()>();
        let receiver = flow.process::<()>();
        sender
            .embedded_input::<String, Unbounded, TotalOrder, AtLeastOnce>("input")
            .send(&receiver, EMBEDDED.fail_stop().name("messages"))
            .embedded_output("output");

        let code = flow
            .with_process(&sender, "duplicable_sender")
            .with_process(&receiver, "duplicable_receiver")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/duplicable_channel.rs"),
            prettyplease::unparse(&code),
        )
        .unwrap();
    }

    // --- singleton_input (local, singleton + stream) ---
    {
        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let process = flow.process::<()>();
        hydro_test::local::singleton_input::prefix_names(
            process.embedded_input("names"),
            process.embedded_singleton_input("prefix"),
        );

        let code = flow
            .with_process(&process, "prefix_names")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/singleton_input.rs"),
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

    // --- echo_network_embedded (o2o networking, an `EMBEDDED` channel) ---
    {
        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let sender = flow.process::<hydro_test::embedded::echo_network_embedded::Sender>();
        let receiver = flow.process::<hydro_test::embedded::echo_network_embedded::Receiver>();
        hydro_test::embedded::echo_network_embedded::echo_network_embedded(
            &receiver,
            sender.embedded_input("input"),
        )
        .embedded_output("output");

        let code = flow
            .with_process(&sender, "echo_sender")
            .with_process(&receiver, "echo_receiver")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/echo_network_embedded.rs"),
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

    // --- o2m_demux_embedded (process -> cluster demux, an `EMBEDDED` channel) ---
    // The payload type has no serde derives; constructing this flow in a build script
    // (a non-test context) is the regression scenario for hydro-project/hydro#3158.
    {
        let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
        let process = flow.process::<hydro_test::embedded::o2m_demux_embedded::Src>();
        let cluster = flow.cluster::<hydro_test::embedded::o2m_demux_embedded::Dst>();
        hydro_test::embedded::o2m_demux_embedded::o2m_demux_embedded(
            &cluster,
            process.embedded_input("input"),
        )
        .embedded_output("output");

        let code = flow
            .with_process(&process, "o2m_demux_sender")
            .with_cluster(&cluster, "o2m_demux_receiver")
            .generate_embedded("hydro_test");

        std::fs::write(
            format!("{out_dir}/o2m_demux_embedded.rs"),
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
