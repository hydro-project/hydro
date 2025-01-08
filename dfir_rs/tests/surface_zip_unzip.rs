use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_zip_basic() {
    let (result_send, mut result_recv) =
        dfir_rs::util::unbounded_channel::<(usize, &'static str)>();

    let mut df = dfir_syntax! {
        source_iter(0..5) -> [0]my_zip;
        source_iter(["Hello", "World"]) -> [1]my_zip;
        my_zip = zip() -> for_each(|pair| result_send.send(pair).unwrap());
    };
    df.run_available();

    let result: Vec<_> = collect_ready(&mut result_recv);
    assert_eq!(&[(0, "Hello"), (1, "World")], &*result);
}

#[multiplatform_test]
pub fn test_zip_loop() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(char, usize)>();

    let mut df = dfir_syntax! {
        source_iter("Hello World".chars()) -> [0]my_zip;
        source_iter(0..5) -> rhs;

        rhs = union() -> tee();
        rhs -> [1]my_zip;
        rhs -> filter_map(|x: usize| x.checked_sub(1)) -> rhs; // Loop

        my_zip = zip() -> for_each(|pair| result_send.send(pair).unwrap());
    };
    df.run_available();

    let result: Vec<_> = collect_ready(&mut result_recv);
    assert_eq!(
        &[
            ('H', 0),
            ('e', 1),
            ('l', 2),
            ('l', 3),
            ('o', 4),
            (' ', 0),
            ('W', 1),
            ('o', 2),
            ('r', 3),
            ('l', 0),
            ('d', 1)
        ],
        &*result
    );
}

#[multiplatform_test]
pub fn test_zip_longest_basic() {
    use dfir_rs::itertools::EitherOrBoth::{self, *};

    let (result_send, mut result_recv) =
        dfir_rs::util::unbounded_channel::<EitherOrBoth<usize, &'static str>>();

    let mut df = dfir_syntax! {
        source_iter(0..5) -> [0]my_zip_longest;
        source_iter(["Hello", "World"]) -> [1]my_zip_longest;
        my_zip_longest = zip_longest() -> for_each(|pair| result_send.send(pair).unwrap());
    };
    df.run_available();

    let result: Vec<_> = collect_ready(&mut result_recv);
    assert_eq!(
        &[
            Both(0, "Hello"),
            Both(1, "World"),
            Left(2),
            Left(3),
            Left(4)
        ],
        &*result
    );
}

#[multiplatform_test]
pub fn test_unzip_basic() {
    let (send0, mut recv0) = dfir_rs::util::unbounded_channel::<&'static str>();
    let (send1, mut recv1) = dfir_rs::util::unbounded_channel::<&'static str>();
    let mut df = dfir_syntax! {
        my_unzip = source_iter(vec![("Hello", "Foo"), ("World", "Bar")]) -> unzip();
        my_unzip[0] -> for_each(|v| send0.send(v).unwrap());
        my_unzip[1] -> for_each(|v| send1.send(v).unwrap());
    };

    df.run_available();

    let out0: Vec<_> = collect_ready(&mut recv0);
    assert_eq!(&["Hello", "World"], &*out0);
    let out1: Vec<_> = collect_ready(&mut recv1);
    assert_eq!(&["Foo", "Bar"], &*out1);
}
