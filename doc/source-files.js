var sourcesIndex = JSON.parse('{\
"hydro":["",[["core",[["hydroflow_crate",[],["build.rs","mod.rs","ports.rs"]]],["custom_service.rs","deployment.rs","gcp.rs","localhost.rs","mod.rs","progress.rs","ssh.rs","terraform.rs","util.rs"]]],["cli.rs","lib.rs"]],\
"hydroflow":["",[["compiled",[["pull",[["half_join_state",[],["fold.rs","fold_from.rs","mod.rs","multiset.rs","multiset2.rs","reduce.rs","set.rs"]]],["anti_join.rs","cross_join.rs","mod.rs","symmetric_hash_join.rs"]]],["mod.rs"]],["props",[],["mod.rs","wrap.rs"]],["scheduled",[["handoff",[],["handoff_list.rs","mod.rs","tee.rs","vector.rs"]],["net",[],["mod.rs","network_vertex.rs"]]],["context.rs","graph.rs","graph_ext.rs","input.rs","mod.rs","port.rs","query.rs","reactor.rs","state.rs","subgraph.rs"]],["util",[["unsync",[],["mod.rs","mpsc.rs"]]],["clear.rs","demux_enum.rs","mod.rs","monotonic.rs","monotonic_map.rs","multiset.rs","socket.rs","sparse_vec.rs","tcp.rs","udp.rs"]]],["declarative_macro.rs","lib.rs"]],\
"hydroflow_cli_integration":["",[],["lib.rs"]],\
"hydroflow_datalog":["",[],["lib.rs"]],\
"hydroflow_datalog_core":["",[],["grammar.rs","join_plan.rs","lib.rs","util.rs"]],\
"hydroflow_lang":["",[["graph",[["ops",[],["_lattice_fold_batch.rs","_lattice_join_fused_join.rs","anti_join.rs","anti_join_multiset.rs","assert.rs","assert_eq.rs","cast.rs","cross_join.rs","cross_join_multiset.rs","defer_signal.rs","defer_tick.rs","demux.rs","demux_enum.rs","dest_file.rs","dest_sink.rs","dest_sink_serde.rs","difference.rs","difference_multiset.rs","enumerate.rs","filter.rs","filter_map.rs","flat_map.rs","flatten.rs","fold.rs","fold_keyed.rs","for_each.rs","identity.rs","initialize.rs","inspect.rs","join.rs","join_fused.rs","join_fused_lhs.rs","join_fused_rhs.rs","join_multiset.rs","lattice_fold.rs","lattice_reduce.rs","map.rs","mod.rs","multiset_delta.rs","next_stratum.rs","null.rs","partition.rs","persist.rs","persist_mut.rs","persist_mut_keyed.rs","py_udf.rs","reduce.rs","reduce_keyed.rs","sort.rs","sort_by_key.rs","source_file.rs","source_interval.rs","source_iter.rs","source_iter_delta.rs","source_json.rs","source_stdin.rs","source_stream.rs","source_stream_serde.rs","spin.rs","tee.rs","union.rs","unique.rs","unzip.rs","zip.rs","zip_longest.rs"]]],["di_mul_graph.rs","eliminate_extra_unions_tees.rs","flat_graph_builder.rs","flat_to_partitioned.rs","flow_props.rs","graph_algorithms.rs","graph_write.rs","hydroflow_graph.rs","mod.rs","propegate_flow_props.rs"]]],["diagnostic.rs","lib.rs","parse.rs","pretty_span.rs","union_find.rs"]],\
"hydroflow_macro":["",[],["lib.rs"]],\
"lattices":["",[],["collections.rs","conflict.rs","dom_pair.rs","lib.rs","map_union.rs","ord.rs","pair.rs","point.rs","set_union.rs","test.rs","union_find.rs","unit.rs","vec_union.rs","with_bot.rs","with_top.rs"]],\
"multiplatform_test":["",[],["lib.rs"]],\
"pusherator":["",[],["demux.rs","filter.rs","filter_map.rs","flatten.rs","for_each.rs","inspect.rs","lib.rs","map.rs","null.rs","partition.rs","pivot.rs","switch.rs","tee.rs","unzip.rs"]],\
"relalg":["",[],["codegen.rs","lib.rs","runtime.rs","sexp.rs"]],\
"variadics":["",[],["lib.rs"]],\
"website_playground":["",[],["lib.rs","utils.rs"]]\
}');
createSourceSidebar();
