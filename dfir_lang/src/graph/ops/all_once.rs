use super::{DelayType, OperatorConstraints};

// Same as batch, but with a stratum barrier.
/// TODO(mingwei): docs
pub const ALL_ONCE: OperatorConstraints = OperatorConstraints {
    name: "all_once",
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    ..super::batch::BATCH
};
