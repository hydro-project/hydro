#[derive(Copy, Clone)]
pub struct NonDet;

#[macro_export]
macro_rules! local_nondet {
    ($reason:expr) => {
        $crate::unsafety::NonDet
    };
}

#[macro_export]
macro_rules! partial_exposed_nondet {
    ($reason:expr, $exposed:expr) => {
        $exposed
    };
}
