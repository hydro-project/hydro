use super::OperatorConstraints;

// Is actually the same as batch.
/// TODO(mingwei): docs
pub const ALL_ONCE: OperatorConstraints = OperatorConstraints {
    name: "all_once",
    ..super::batch::BATCH
};
