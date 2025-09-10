use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_multiset_delta() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut flow = dfir_syntax! {
        source_stream(input_recv)
            -> multiset_delta()
            -> for_each(|x| result_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(flow);

    input_send.send(3).unwrap();
    input_send.send(4).unwrap();
    input_send.send(3).unwrap();
    flow.run_tick_sync();
    assert_eq!(&[3, 4, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    input_send.send(3).unwrap();
    input_send.send(5).unwrap();
    input_send.send(3).unwrap();
    input_send.send(3).unwrap();
    flow.run_tick_sync();
    // First two "3"s are removed due to previous tick.
    assert_eq!(&[5, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
pub fn test_persist_multiset_delta() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(input_recv)
            -> persist::<'static>()
            -> multiset_delta()
            -> for_each(|x| output_send.send(x).unwrap());
    };

    input_send.send(1).unwrap();
    flow.run_tick_sync();
    assert_eq!(&[(1)], &*collect_ready::<Vec<_>, _>(&mut output_recv));

    flow.run_tick_sync();
    assert!(collect_ready::<Vec<_>, _>(&mut output_recv).is_empty());

    flow.run_tick_sync();
    assert!(collect_ready::<Vec<_>, _>(&mut output_recv).is_empty());
}

#[multiplatform_test]
pub fn test_multiset_delta_2() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut flow = dfir_syntax! {
        source_stream(input_recv)
            -> multiset_delta()
            -> for_each(|x| result_send.send(x).unwrap());
    };

    input_send.send(3).unwrap();
    input_send.send(4).unwrap();
    input_send.send(3).unwrap();
    flow.run_tick_sync();
    assert_eq!(&[3, 4, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    input_send.send(3).unwrap();
    input_send.send(4).unwrap();
    input_send.send(3).unwrap();
    input_send.send(3).unwrap();
    flow.run_tick_sync();

    assert_eq!(&[3], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
fn test_chat_app_replay() {
    let (users_send, users) = dfir_rs::util::unbounded_channel::<u32>();
    let (messages_send, messages) = dfir_rs::util::unbounded_channel::<String>();
    let (out, mut out_recv) = dfir_rs::util::unbounded_channel::<(u32, String)>();

    let mut chat_server = dfir_syntax! {
        users = source_stream(users) -> persist::<'static>();
        messages = source_stream(messages) -> persist::<'static>();
        users -> [0]crossed;
        messages -> [1]crossed;
        crossed = cross_join::<'tick, 'tick>()
            -> multiset_delta()
            -> for_each(|t| {
                out.send(t).unwrap();
            });
    };

    users_send.send(1).unwrap();
    users_send.send(2).unwrap();

    messages_send.send("hello".to_string()).unwrap();
    messages_send.send("world".to_string()).unwrap();

    chat_server.run_tick_sync();

    assert_eq!(
        &[
            (1, "hello".to_string()),
            (2, "hello".to_string()),
            (1, "world".to_string()),
            (2, "world".to_string())
        ],
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
    );

    users_send.send(3).unwrap();

    messages_send.send("goodbye".to_string()).unwrap();

    chat_server.run_tick_sync();

    // fails with: [(1, "hello"), (2, "hello"), (3, "hello"), (1, "world"), (2, "world"), (3, "world"), (1, "goodbye"), (2, "goodbye"), (3, "goodbye")]

    assert_eq!(
        &[
            (3, "hello".to_string()),
            (3, "world".to_string()),
            (1, "goodbye".to_string()),
            (2, "goodbye".to_string()),
            (3, "goodbye".to_string())
        ],
        &*collect_ready::<Vec<_>, _>(&mut out_recv),
    );
}

#[multiplatform_test]
fn test_chat_app_replay_manual() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<(u32, String)>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(u32, String)>();

    let mut flow = dfir_syntax! {
        source_stream(input_recv)
            -> multiset_delta()
            -> for_each(|x| result_send.send(x).unwrap());
    };

    input_send.send((1, "hello".to_owned())).unwrap();
    input_send.send((2, "hello".to_owned())).unwrap();
    input_send.send((1, "world".to_owned())).unwrap();
    input_send.send((2, "world".to_owned())).unwrap();

    flow.run_tick_sync();
    assert_eq!(
        &[
            (1, "hello".to_string()),
            (2, "hello".to_string()),
            (1, "world".to_string()),
            (2, "world".to_string())
        ],
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
    );

    input_send.send((1, "hello".to_owned())).unwrap();
    input_send.send((2, "hello".to_owned())).unwrap();
    input_send.send((3, "hello".to_owned())).unwrap();
    input_send.send((1, "world".to_owned())).unwrap();
    input_send.send((2, "world".to_owned())).unwrap();
    input_send.send((3, "world".to_owned())).unwrap();
    input_send.send((1, "goodbye".to_owned())).unwrap();
    input_send.send((2, "goodbye".to_owned())).unwrap();
    input_send.send((3, "goodbye".to_owned())).unwrap();

    flow.run_tick_sync();
    assert_eq!(
        &[
            (3, "hello".to_string()),
            (3, "world".to_string()),
            (1, "goodbye".to_string()),
            (2, "goodbye".to_string()),
            (3, "goodbye".to_string())
        ],
        &*collect_ready::<Vec<_>, _>(&mut result_recv),
    );
}
