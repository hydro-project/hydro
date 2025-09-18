use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_partition_fizzbuzz() {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel::<String>();

    let mut df = dfir_syntax! {
        my_partition = source_iter(1..=15)
            -> partition(|&v, [fzbz, fizz, buzz, vals]|
                match (v % 3, v % 5) {
                    (0, 0) => fzbz,
                    (0, _) => fizz,
                    (_, 0) => buzz,
                    (_, _) => vals,
                }
            );
        my_partition[vals] -> map(|x| format!("{}", x))
            -> for_each(|s| out_send.send(s).unwrap());
        my_partition[fizz] -> map(|_| "fizz".to_owned())
            -> for_each(|s| out_send.send(s).unwrap());
        my_partition[buzz] -> map(|_| "buzz".to_owned())
            -> for_each(|s| out_send.send(s).unwrap());
        my_partition[fzbz] -> map(|_| "fizzbuzz".to_owned())
            -> for_each(|s| out_send.send(s).unwrap());
    };
    df.run_available_sync();

    assert_eq!(
        &[
            "1", "2", "fizz", "4", "buzz", "fizz", "7", "8", "fizz", "buzz", "11", "fizz", "13",
            "14", "fizzbuzz"
        ],
        &*collect_ready::<Vec<_>, _>(out_recv)
    )
}

#[multiplatform_test]
pub fn test_partition_round() {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel::<String>();

    let mut df = dfir_syntax! {
        my_partition = source_iter(0..20)
            -> partition(|v, len| v % len);
        my_partition[2] -> map(|x| format!("{} 2", x))
            -> for_each(|s| out_send.send(s).unwrap());
        my_partition[1] -> map(|x| format!("{} 1", x))
            -> for_each(|s| out_send.send(s).unwrap());
        my_partition[3] -> map(|x| format!("{} 3", x))
            -> for_each(|s| out_send.send(s).unwrap());
        my_partition[0] -> map(|x| format!("{} 0", x))
            -> for_each(|s| out_send.send(s).unwrap());
    };
    df.run_available_sync();

    assert_eq!(
        &[
            "0 0", "1 1", "2 2", "3 3", "4 0", "5 1", "6 2", "7 3", "8 0", "9 1", "10 2", "11 3",
            "12 0", "13 1", "14 2", "15 3", "16 0", "17 1", "18 2", "19 3"
        ],
        &*collect_ready::<Vec<_>, _>(out_recv)
    )
}
