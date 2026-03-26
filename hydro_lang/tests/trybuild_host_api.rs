//! Tests for the `TrybuildHost` builder API improvements (issue #2697).

use hydro_deploy::Deployment;
use hydro_lang::deploy::TrybuildHost;

fn make_host() -> TrybuildHost {
    let deployment = Deployment::new();
    TrybuildHost::new(deployment.Localhost())
}

#[test]
fn features_accepts_str_slice_iter() {
    let _host = make_host().features(["feat_a", "feat_b"]);
}

#[test]
fn features_accepts_string_vec() {
    let _host = make_host().features(vec!["a".to_owned(), "b".to_owned()]);
}

#[test]
fn feature_singular_accepts_str() {
    let _host = make_host().feature("single");
}

#[test]
fn feature_chains() {
    let _host = make_host().feature("a").feature("b").features(["c"]);
}

#[test]
fn additional_hydro_feature_singular() {
    let _host = make_host().additional_hydro_feature("runtime_measure");
}

#[test]
fn additional_hydro_features_accepts_str_iter() {
    let _host = make_host().additional_hydro_features(["a", "b"]);
}
