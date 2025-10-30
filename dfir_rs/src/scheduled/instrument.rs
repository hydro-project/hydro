use std::{
    pin::Pin,
    task::{Context, Poll}, time::{Duration, Instant},
};

use pin_project_lite::pin_project;

#[derive(Default)]
pub struct Metrics {
    poll_duration: Duration,
    poll_count: usize,
    idle_duration: Duration,
    idle_count: usize,
}

pin_project! {
    pub(crate) struct Instrument<'a, Fut> {
        #[pin]
        future: Fut,
        idle_start: Option<Instant>,
        metrics: &'a mut Metrics,
    }
}

impl<'a, Fut> Instrument<'a, Fut> {
    pub(crate) fn new(future: Fut, metrics: &'a mut Metrics) -> Self {
        Self {
            future,
            idle_start: None,
            metrics,
        }
    }
}

impl<'a, Fut> Future for Instrument<'a, Fut>
where
    Fut: Future,
{
    type Output = Fut::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        if let Some(idle_start) = this.idle_start {
            this.metrics.idle_duration += idle_start.elapsed();
            this.metrics.idle_count += 1;
        }

        let poll_start = Instant::now();
        let out = this.future.poll(cx);
        this.idle_start.replace(Instant::now());

        this.metrics.poll_duration += poll_start.elapsed();
        this.metrics.poll_count += 1;

        out
    }
}
