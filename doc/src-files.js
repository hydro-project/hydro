var srcIndex = new Map(JSON.parse('[["gossip_cli",["",[],["main.rs"]]],["gossip_kv",["",[["lattices",[],["mod.rs"]]],["lib.rs","membership.rs","model.rs","server.rs","util.rs"]]],["gossip_server",["",[["config",[],["mod.rs"]]],["main.rs","membership.rs"]]],["hydro_cli",["",[],["cli.rs","lib.rs"]]],["hydro_deploy",["",[["hydroflow_crate",[],["build.rs","flamegraph.rs","mod.rs","ports.rs","service.rs","tracing_options.rs"]],["localhost",[],["launched_binary.rs","mod.rs"]]],["azure.rs","custom_service.rs","deployment.rs","gcp.rs","lib.rs","progress.rs","ssh.rs","terraform.rs","util.rs"]]],["hydro_lang",["",[["builder",[],["built.rs","compiled.rs","deploy.rs","mod.rs"]],["deploy",[],["in_memory_graph.rs","macro_runtime.rs","mod.rs"]],["location",[["cluster",[],["cluster_id.rs","mod.rs"]]],["can_send.rs","external_process.rs","mod.rs","process.rs","tick.rs"]],["rewrites",[],["mod.rs","persist_pullup.rs","profiler.rs","properties.rs"]]],["boundedness.rs","cycle.rs","deploy_runtime.rs","ir.rs","lib.rs","optional.rs","runtime_context.rs","singleton.rs","staging_util.rs","stream.rs"]]],["hydro_std",["",[],["lib.rs","quorum.rs","request_response.rs"]]],["hydro_test",["",[["cluster",[],["compute_pi.rs","many_to_many.rs","map_reduce.rs","mod.rs","paxos.rs","paxos_bench.rs","paxos_kv.rs","simple_cluster.rs","two_pc.rs"]],["distributed",[],["first_ten.rs","mod.rs"]]],["lib.rs"]]],["hydro_test_local",["",[["local",[],["chat_app.rs","compute_pi.rs","count_elems.rs","first_ten.rs","graph_reachability.rs","mod.rs","negation.rs","teed_join.rs"]]],["lib.rs"]]],["hydro_test_local_macro",["",[["local",[],["chat_app.rs","compute_pi.rs","count_elems.rs","first_ten.rs","graph_reachability.rs","mod.rs","negation.rs","teed_join.rs"]]],["lib.rs"]]],["hydroflow",["",[["compiled",[["pull",[["half_join_state",[],["fold.rs","fold_from.rs","mod.rs","multiset.rs","reduce.rs","set.rs"]]],["anti_join.rs","cross_join.rs","mod.rs","symmetric_hash_join.rs"]]],["mod.rs"]],["scheduled",[["handoff",[],["handoff_list.rs","mod.rs","tee.rs","vector.rs"]],["net",[],["mod.rs","network_vertex.rs"]]],["context.rs","graph.rs","graph_ext.rs","input.rs","mod.rs","port.rs","query.rs","reactor.rs","state.rs","subgraph.rs","ticks.rs"]],["util",[["unsync",[],["mod.rs","mpsc.rs"]]],["clear.rs","demux_enum.rs","deploy.rs","mod.rs","monotonic.rs","monotonic_map.rs","multiset.rs","simulation.rs","socket.rs","sparse_vec.rs","tcp.rs","udp.rs"]]],["declarative_macro.rs","lib.rs"]]],["hydroflow_datalog",["",[],["lib.rs"]]],["hydroflow_datalog_core",["",[],["grammar.rs","join_plan.rs","lib.rs","util.rs"]]],["hydroflow_deploy_integration",["",[],["lib.rs"]]],["hydroflow_lang",["",[["graph",[["ops",[],["_lattice_fold_batch.rs","_lattice_join_fused_join.rs","all_once.rs","anti_join.rs","anti_join_multiset.rs","assert.rs","assert_eq.rs","batch.rs","chain.rs","cross_join.rs","cross_join_multiset.rs","cross_singleton.rs","defer_signal.rs","defer_tick.rs","defer_tick_lazy.rs","demux.rs","demux_enum.rs","dest_file.rs","dest_sink.rs","dest_sink_serde.rs","difference.rs","difference_multiset.rs","enumerate.rs","filter.rs","filter_map.rs","flat_map.rs","flatten.rs","fold.rs","fold_keyed.rs","for_each.rs","identity.rs","initialize.rs","inspect.rs","join.rs","join_fused.rs","join_fused_lhs.rs","join_fused_rhs.rs","join_multiset.rs","lattice_bimorphism.rs","lattice_fold.rs","lattice_reduce.rs","map.rs","mod.rs","multiset_delta.rs","next_stratum.rs","null.rs","partition.rs","persist.rs","persist_mut.rs","persist_mut_keyed.rs","py_udf.rs","reduce.rs","reduce_keyed.rs","sort.rs","sort_by_key.rs","source_file.rs","source_interval.rs","source_iter.rs","source_json.rs","source_stdin.rs","source_stream.rs","source_stream_serde.rs","spin.rs","state.rs","state_by.rs","tee.rs","union.rs","unique.rs","unzip.rs","zip.rs","zip_longest.rs"]]],["di_mul_graph.rs","eliminate_extra_unions_tees.rs","flat_graph_builder.rs","flat_to_partitioned.rs","graph_algorithms.rs","graph_write.rs","hydroflow_graph.rs","hydroflow_graph_debugging.rs","mod.rs"]]],["diagnostic.rs","lib.rs","parse.rs","pretty_span.rs","process_singletons.rs","union_find.rs"]]],["hydroflow_macro",["",[],["lib.rs"]]],["latency_measure",["",[],["latency_measure.rs","protocol.rs"]]],["lattices",["",[["ght",[],["colt.rs","lattice.rs","macros.rs","mod.rs","test.rs"]]],["algebra.rs","collections.rs","conflict.rs","dom_pair.rs","lib.rs","map_union.rs","map_union_with_tombstones.rs","ord.rs","pair.rs","point.rs","semiring_application.rs","set_union.rs","set_union_with_tombstones.rs","test.rs","union_find.rs","unit.rs","vec_union.rs","with_bot.rs","with_top.rs"]]],["lattices_macro",["",[],["lib.rs"]]],["load_test_server",["",[],["server.rs"]]],["multiplatform_test",["",[],["lib.rs"]]],["pn",["",[],["pn.rs","protocol.rs"]]],["pn_delta",["",[],["pn_delta.rs","protocol.rs"]]],["pusherator",["",[],["demux.rs","filter.rs","filter_map.rs","flatten.rs","for_each.rs","inspect.rs","lib.rs","map.rs","null.rs","partition.rs","pivot.rs","switch.rs","tee.rs","unzip.rs"]]],["relalg",["",[],["codegen.rs","lib.rs","runtime.rs","sexp.rs"]]],["stageleft",["",[],["lib.rs","runtime_support.rs","type_name.rs"]]],["stageleft_macro",["",[["quote_impl",[["free_variable",[],["mod.rs","prelude.rs"]]],["mod.rs"]]],["lib.rs"]]],["stageleft_test",["",[],["lib.rs","submodule.rs"]]],["stageleft_test_macro",["",[],["lib.rs","submodule.rs"]]],["stageleft_tool",["",[],["lib.rs"]]],["topolotree",["",[],["main.rs","protocol.rs"]]],["variadics",["",[],["lib.rs","variadic_collections.rs"]]],["variadics_macro",["",[],["lib.rs"]]],["website_playground",["",[],["lib.rs","utils.rs"]]]]'));
createSrcSidebar();
//{"start":36,"fragment_lengths":[34,108,78,42,294,501,67,228,178,184,713,41,84,52,1482,39,65,383,38,43,42,39,51,190,66,69,111,53,59,38,49,59,39,53]}