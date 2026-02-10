#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

#[cfg(all(test, feature = "test_embedded"))]
mod tests {
    #[tokio::test]
    async fn test_embedded_capitalize() {
        let input = dfir_rs::futures::stream::iter(vec![
            "hello".to_owned(),
            "world".to_owned(),
            "hydro".to_owned(),
        ]);

        let mut collected = vec![];
        let mut outputs = crate::embedded::capitalize::EmbeddedOutputs {
            output: |s: String| {
                collected.push(s);
            },
        };

        let mut flow = crate::embedded::capitalize(input, &mut outputs);
        tokio::task::LocalSet::new()
            .run_until(flow.run_available())
            .await;
        drop(flow);

        assert_eq!(collected, vec!["HELLO", "WORLD", "HYDRO"],);
    }
}
