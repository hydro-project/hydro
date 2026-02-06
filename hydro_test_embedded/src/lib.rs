#[cfg(feature = "test_embedded")]
#[allow(unused_imports, missing_docs, non_snake_case)]
pub mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}
