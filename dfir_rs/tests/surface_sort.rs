use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_sort() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_stream(items_recv)
            -> sort()
            -> for_each(|v| print!("{:?}, ", v));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    print!("\nA: ");

    items_send.send(9).unwrap();
    items_send.send(2).unwrap();
    items_send.send(5).unwrap();
    df.run_available_sync();

    print!("\nB: ");

    items_send.send(9).unwrap();
    items_send.send(5).unwrap();
    items_send.send(2).unwrap();
    items_send.send(0).unwrap();
    items_send.send(3).unwrap();
    df.run_available_sync();

    println!();
}

#[multiplatform_test]
pub fn test_sort_by_key() {
    let mut df = dfir_syntax! {
        source_iter(vec!((2, 'y'), (3, 'x'), (1, 'z')))
            -> sort_by_key(|(k, _v)| k)
            -> for_each(|v| println!("{:?}", v));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();
    println!();
}

#[multiplatform_test]
fn test_sort_by_owned() {
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
    struct Dummy {
        x: String,
        y: i8,
    }

    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<Dummy>();

    let dummies: Vec<Dummy> = vec![
        Dummy {
            x: "a".to_string(),
            y: 2,
        },
        Dummy {
            x: "b".to_string(),
            y: 1,
        },
    ];
    let mut dummies_saved = dummies.clone();

    let mut df = dfir_syntax! {
        source_iter(dummies) -> sort_by_key(|d| &d.x) -> for_each(|d| out_send.send(d).unwrap());
    };
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    dummies_saved.sort_unstable_by(|d1, d2| d1.y.cmp(&d2.y));
    assert_ne!(&dummies_saved, &*results);
    dummies_saved.sort_unstable_by(|d1, d2| d1.x.cmp(&d2.x));
    assert_eq!(&dummies_saved, &*results);
}
