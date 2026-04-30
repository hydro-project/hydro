//! Tests for IR JSON serialization.

#[cfg(test)]
#[cfg(feature = "viz")]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use hydro_build_utils::insta;
    use proc_macro2::Span;
    use stageleft::q;

    use crate::compile::ir::*;
    use crate::location::dynamic::LocationId;
    use crate::location::{Location, LocationKey};

    #[test]
    fn serialize_debug_expr() {
        let expr: syn::Expr = syn::parse_str("x + 1").unwrap();
        let de = DebugExpr(Box::new(expr));
        // DebugExpr::Display wraps in q!(...)
        assert_eq!(serde_json::to_string(&de).unwrap(), r#""q!(x + 1)""#);
    }

    #[test]
    fn serialize_debug_type() {
        let ty: syn::Type = syn::parse_str("Vec<i32>").unwrap();
        let dt = DebugType(Box::new(ty));
        assert_eq!(serde_json::to_string(&dt).unwrap(), r#""Vec < i32 >""#);
    }

    #[test]
    fn serialize_debug_instantiate() {
        assert_eq!(
            serde_json::to_string(&DebugInstantiate::Building).unwrap(),
            r#""Building""#
        );

        let finalized: DebugInstantiate = DebugInstantiateFinalized {
            sink: syn::parse_str("sink_expr").unwrap(),
            source: syn::parse_str("source_expr").unwrap(),
            connect_fn: None,
        }
        .into();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            serde_json::to_string(&finalized).unwrap()
        }));
        assert!(result.is_err(), "Finalized should panic on serialize");
    }

    #[test]
    fn serialize_hydro_source_all_variants() {
        let expr = DebugExpr(Box::new(syn::parse_str("my_stream").unwrap()));
        let ident = syn::Ident::new("my_ident", Span::call_site());
        let loc = LocationId::Process(LocationKey::TEST_KEY_1);

        // With derive(Serialize), HydroSource serializes as a tagged enum.
        // DebugExpr::Display wraps in q!(...), so stream/iter include that.
        let cases: Vec<(HydroSource, &str)> = vec![
            (
                HydroSource::Stream(expr.clone()),
                r#"{"Stream":"q!(my_stream)"}"#,
            ),
            (HydroSource::ExternalNetwork(), r#"{"ExternalNetwork":[]}"#),
            (HydroSource::Iter(expr), r#"{"Iter":"q!(my_stream)"}"#),
            (HydroSource::Spin(), r#"{"Spin":[]}"#),
            (
                HydroSource::ClusterMembers(loc, ClusterMembersState::Uninit),
                r#"{"ClusterMembers":[{"Process":{"idx":1,"version":255}},"Uninit"]}"#,
            ),
            (
                HydroSource::Embedded(ident.clone()),
                r#"{"Embedded":"my_ident"}"#,
            ),
            (
                HydroSource::EmbeddedSingleton(ident),
                r#"{"EmbeddedSingleton":"my_ident"}"#,
            ),
        ];

        for (source, expected) in cases {
            let json = serde_json::to_string(&source).unwrap();
            assert_eq!(json, expected, "failed for {source:?}");
        }
    }

    #[test]
    fn serialize_shared_node_dedup() {
        let node = HydroNode::Placeholder;
        let shared = Rc::new(RefCell::new(node));
        let sn1 = SharedNode(shared.clone());
        let sn2 = SharedNode(shared);

        let (j1, j2) = serialize_dedup_shared(|| {
            let j1 = serde_json::to_value(&sn1).unwrap();
            let j2 = serde_json::to_value(&sn2).unwrap();
            (j1, j2)
        });

        assert_eq!(j1["$shared"], 0);
        assert!(j1.get("node").is_some());
        assert_eq!(j2["$shared_ref"], 0);
        assert!(j2.get("node").is_none());
    }

    #[test]
    fn serialize_shared_node_requires_scope() {
        let shared = SharedNode(Rc::new(RefCell::new(HydroNode::Placeholder)));
        let result = serde_json::to_string(&shared);
        assert!(result.is_err(), "should fail without dedup scope");
    }

    /// Builds a small flow (source → map → tee → two for_each sinks),
    /// serializes the IR to JSON, and snapshots the output.
    #[test]
    fn ir_json_snapshot() {
        let mut flow = crate::prelude::FlowBuilder::new();
        let process = flow.process::<()>();

        let stream = process.source_iter(q!(0..10)).map(q!(|x| x * 2));
        let tee1 = stream.clone();
        tee1.for_each(q!(|v| println!("{}", v)));
        stream.for_each(q!(|v| eprintln!("{}", v)));

        let built = flow.finalize();
        let json = serialize_dedup_shared(|| serde_json::to_string_pretty(built.ir()).unwrap());

        // Redact absolute paths for CI portability
        let workspace_root = env!("CARGO_MANIFEST_DIR")
            .strip_suffix("/hydro_lang")
            .unwrap_or(env!("CARGO_MANIFEST_DIR"));
        let json = json.replace(workspace_root, "[workspace]");

        insta::assert_snapshot!(json);
    }
}
