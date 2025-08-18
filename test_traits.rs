use hydro_lang::ir::{StreamOrdering, StreamRetries};
use hydro_lang::stream::{
    AtLeastOnce, ExactlyOnce, NoOrder, OrderingKind, RetriesKind, TotalOrder,
};

fn test_ordering_traits() {
    // Test that our trait implementations work correctly
    assert_eq!(TotalOrder::ordering(), StreamOrdering::TotalOrder);
    assert_eq!(NoOrder::ordering(), StreamOrdering::NoOrder);

    assert_eq!(ExactlyOnce::retries(), StreamRetries::ExactlyOnce);
    assert_eq!(AtLeastOnce::retries(), StreamRetries::AtLeastOnce);

    println!("All trait implementations work correctly!");
}

fn main() {
    test_ordering_traits();
}
