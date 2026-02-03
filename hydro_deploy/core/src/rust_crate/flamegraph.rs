use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{Error, Result};
use futures::stream::FuturesUnordered;
use tokio_stream::StreamExt;

use super::tracing_options::TracingOptions;

pub async fn handle_fold_data(
    tracing: &TracingOptions,
    fold_data: impl Into<Arc<[u8]>>,
) -> Result<()> {
    // Wrap in Arc to allow sharing data across multiple outputs.
    let fold_data = &fold_data.into();
    let output_tasks =
        FuturesUnordered::<Pin<Box<dyn Future<Output = Result<()>> + Send + Sync>>>::new();

    // fold_outfile
    if let Some(fold_outfile) = tracing.fold_outfile.clone() {
        let fold_data = Arc::clone(fold_data);
        output_tasks.push(Box::pin(async move {
            let mut reader = &*fold_data;
            let mut writer = tokio::fs::File::create(fold_outfile).await?;
            tokio::io::copy_buf(&mut reader, &mut writer).await?;
            Ok(())
        }));
    };

    // flamegraph_outfile
    if let Some(flamegraph_outfile) = tracing.flamegraph_outfile.clone() {
        let mut options = tracing
            .flamegraph_options
            .map(|f| (f)())
            .unwrap_or_default();
        output_tasks.push(Box::pin(async move {
            let writer = tokio::fs::File::create(flamegraph_outfile)
                .await?
                .into_std()
                .await;
            let fold_data = Arc::clone(fold_data);
            tokio::task::spawn_blocking(move || {
                inferno::flamegraph::from_lines(
                    &mut options,
                    fold_data
                        .split(|&b| b == b'\n')
                        .map(std::str::from_utf8)
                        .map(Result::unwrap),
                    writer,
                )
            })
            .await??;
            Ok(())
        }));
    };

    let errors = output_tasks
        .filter_map(Result::err)
        .collect::<Vec<_>>()
        .await;
    if !errors.is_empty() {
        return Err(MultipleErrors { errors }.into());
    };

    Ok(())
}

#[derive(Debug)]
struct MultipleErrors {
    errors: Vec<Error>,
}
impl std::fmt::Display for MultipleErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if 1 == self.errors.len() {
            self.errors.first().unwrap().fmt(f)
        } else {
            writeln!(f, "({}) errors occured:", self.errors.len())?;
            writeln!(f)?;
            for (i, error) in self.errors.iter().enumerate() {
                write!(f, "({}/{}):", i + 1, self.errors.len())?;
                error.fmt(f)?;
                writeln!(f)?;
            }
            Ok(())
        }
    }
}
impl std::error::Error for MultipleErrors {}
