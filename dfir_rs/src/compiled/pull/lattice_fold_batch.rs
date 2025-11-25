use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{FusedStream, Stream};
use lattices::Merge;
use pin_project_lite::pin_project;

pin_project! {
    #[project = LatticeFoldBatchStateProj]
    pub enum LatticeFoldBatchState<'a, InputStream, SignalStream, LatticeType> {
        Streaming {
            #[pin]
            input: InputStream,
            #[pin]
            signal: SignalStream,

            lattice_state: &'a mut LatticeType,

            signalled: bool,
        },
        Done,
    }
}

pin_project! {
    /// Special stream for the `_lattice_fold_batch` operator.
    #[must_use = "streams do nothing unless polled"]
    pub struct LatticeFoldBatch<'a, InputStream, SignalStream, LatticeType> {
        #[pin]
        state: LatticeFoldBatchState<'a, InputStream, SignalStream, LatticeType>,
    }
}

impl<'a, InputStream, SignalStream, LatticeType>
    LatticeFoldBatch<'a, InputStream, SignalStream, LatticeType>
where
    InputStream: Stream,
    SignalStream: FusedStream,
    LatticeType: Clone + Merge<InputStream::Item>,
{
    /// Creates a new `LatticeFoldBatch` stream.
    pub fn new(
        input: InputStream,
        signal: SignalStream,
        lattice_state: &'a mut LatticeType,
    ) -> Self {
        Self {
            state: LatticeFoldBatchState::Streaming {
                input,
                signal,
                lattice_state,
                signalled: false,
            },
        }
    }
}

impl<'a, InputStream, SignalStream, LatticeType> Stream
    for LatticeFoldBatch<'a, InputStream, SignalStream, LatticeType>
where
    InputStream: Stream,
    SignalStream: FusedStream,
    LatticeType: Clone + Merge<InputStream::Item>,
{
    type Item = LatticeType;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.state.as_mut().project() {
            LatticeFoldBatchStateProj::Streaming {
                mut input,
                mut signal,
                lattice_state,
                signalled,
            } => {
                // Stage 1: Exhaust the signal.
                // This will get called every time even after returning EOS (`None`), so we need fused.
                while let Some(_signal) = ready!(signal.as_mut().poll_next(cx)) {
                    *signalled = true;
                }

                // Stage 2: Exhaust the input stream
                while let Some(item) = ready!(input.as_mut().poll_next(cx)) {
                    Merge::merge(&mut **lattice_state, item);
                }

                // Stage 3: Emit item once and be done.
                let opt_item = (*signalled).then(|| lattice_state.clone());
                this.state.set(LatticeFoldBatchState::Done);
                Poll::Ready(opt_item)
            }
            // Stage 4: Done.
            LatticeFoldBatchStateProj::Done => Poll::Ready(None),
        }
    }
}
