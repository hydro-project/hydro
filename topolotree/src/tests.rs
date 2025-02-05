use std::time::Duration;

use dfir_rs::bytes::{Bytes, BytesMut};
use dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream;
use dfir_rs::util::multiset::HashMultiSet;
use dfir_rs::util::{
    collect_ready_async, deserialize_from_bytes, serialize_to_bytes, unbounded_channel,
};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::UnboundedSender;

use crate::protocol::{QueryResponse, Timestamped, TopolotreeMessage};
use crate::{run_topolotree, OperationPayload, Payload};

type InputSendResult = Result<(), SendError<Result<(u32, BytesMut), std::io::Error>>>;

pub fn simulate_input(
    input_send: &mut UnboundedSender<Result<(u32, BytesMut), std::io::Error>>,
    (id, msg): (u32, TopolotreeMessage),
) -> InputSendResult {
    input_send.send(Ok((id, BytesMut::from(&serialize_to_bytes(msg)[..]))))
}

type OperationSendResult = Result<(), SendError<Result<BytesMut, std::io::Error>>>;

pub fn simulate_operation(
    input_send: &mut UnboundedSender<Result<BytesMut, std::io::Error>>,
    payload: OperationPayload,
) -> OperationSendResult {
    input_send.send(Ok(BytesMut::from(&serialize_to_bytes(payload)[..])))
}

pub async fn read_all(
    mut output_recv: &mut UnboundedReceiverStream<(u32, Bytes)>,
) -> HashMultiSet<(u32, TopolotreeMessage)> {
    let collected = collect_ready_async::<Vec<_>, _>(&mut output_recv).await;
    collected
        .iter()
        .map(|(id, bytes)| {
            (
                *id,
                deserialize_from_bytes::<TopolotreeMessage>(&bytes[..]).unwrap(),
            )
        })
        .collect::<HashMultiSet<_>>()
}

pub async fn read_all_query(
    mut output_recv: &mut UnboundedReceiverStream<Bytes>,
) -> HashMultiSet<QueryResponse> {
    let collected = collect_ready_async::<Vec<_>, _>(&mut output_recv).await;
    collected
        .iter()
        .map(|bytes| deserialize_from_bytes::<QueryResponse>(&bytes[..]).unwrap())
        .collect::<HashMultiSet<_>>()
}

#[dfir_rs::test(start_paused = true)]
async fn simple_payload_test() {
    let neighbors: Vec<u32> = vec![1, 2, 3];

    let (_operations_tx, operations_rx) = unbounded_channel::<Result<BytesMut, std::io::Error>>();
    let (mut input_send, input_recv) =
        unbounded_channel::<Result<(u32, BytesMut), std::io::Error>>();
    let (output_send, mut output_recv) = unbounded_channel::<(u32, Bytes)>();
    let (query_send, mut query_recv) = unbounded_channel::<Bytes>();

    #[rustfmt::skip]
    simulate_input(&mut input_send, (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 2 } }))).unwrap();

    let mut flow = run_topolotree(
        neighbors,
        input_recv,
        operations_rx,
        output_send,
        query_send,
    );

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 2 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 2 } })),
        (1, TopolotreeMessage::Ping()),
        (2, TopolotreeMessage::Ping()),
        (3, TopolotreeMessage::Ping())
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 2 }
    ]));
}

#[dfir_rs::test(start_paused = true)]
async fn idempotence_test() {
    let neighbors: Vec<u32> = vec![1, 2, 3];
    let (_operations_tx, operations_rx) = unbounded_channel::<Result<BytesMut, std::io::Error>>();

    let (mut input_send, input_recv) =
        unbounded_channel::<Result<(u32, BytesMut), std::io::Error>>();
    let (output_send, mut output_recv) = unbounded_channel::<(u32, Bytes)>();
    let (query_send, mut query_recv) = unbounded_channel::<Bytes>();

    #[rustfmt::skip]
    {
        simulate_input(&mut input_send, (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 4, data: 2 } }))).unwrap();
        simulate_input(&mut input_send, (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 4, data: 2 } }))).unwrap();
    };

    let mut flow = run_topolotree(
        neighbors,
        input_recv,
        operations_rx,
        output_send,
        query_send,
    );

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 2 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 2 } })),
        (1, TopolotreeMessage::Ping()),
        (2, TopolotreeMessage::Ping()),
        (3, TopolotreeMessage::Ping())
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 2 }
    ]));
}

#[dfir_rs::test(start_paused = true)]
async fn backwards_in_time_test() {
    let neighbors: Vec<u32> = vec![1, 2, 3];

    let (_operations_tx, operations_rx) = unbounded_channel::<Result<BytesMut, std::io::Error>>();
    let (mut input_send, input_recv) =
        unbounded_channel::<Result<(u32, BytesMut), std::io::Error>>();
    let (output_send, mut output_recv) = unbounded_channel::<(u32, Bytes)>();
    let (query_send, mut query_recv) = unbounded_channel::<Bytes>();

    #[rustfmt::skip]
    {
        simulate_input(&mut input_send, (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 5, data: 7 } }))).unwrap();
        simulate_input(&mut input_send, (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 4, data: 2 } }))).unwrap();
    };

    let mut flow = run_topolotree(
        neighbors,
        input_recv,
        operations_rx,
        output_send,
        query_send,
    );

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 7 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 7 } })),
        (1, TopolotreeMessage::Ping()),
        (2, TopolotreeMessage::Ping()),
        (3, TopolotreeMessage::Ping())
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 7 }
    ]));
}

#[dfir_rs::test(start_paused = true)]
async fn multiple_input_sources_test() {
    let neighbors: Vec<u32> = vec![1, 2, 3];
    let (_operations_tx, operations_rx) = unbounded_channel::<Result<BytesMut, std::io::Error>>();

    let (mut input_send, input_recv) =
        unbounded_channel::<Result<(u32, BytesMut), std::io::Error>>();
    let (output_send, mut output_recv) = unbounded_channel::<(u32, Bytes)>();
    let (query_send, mut query_recv) = unbounded_channel::<Bytes>();

    #[rustfmt::skip]
    {
        simulate_input(&mut input_send, (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 5, data: 7 } }))).unwrap();
        simulate_input(&mut input_send, (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 4, data: 2 } }))).unwrap();
    };

    let mut flow = run_topolotree(
        neighbors,
        input_recv,
        operations_rx,
        output_send,
        query_send,
    );

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 2, data: 2 } })),
        (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 2, data: 7 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 2, data: 9 } })),
        (1, TopolotreeMessage::Ping()),
        (2, TopolotreeMessage::Ping()),
        (3, TopolotreeMessage::Ping())
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 9 }
    ]));
}

#[dfir_rs::test(start_paused = true)]
async fn operations_across_ticks() {
    let neighbors: Vec<u32> = vec![1, 2, 3];

    let (mut operations_tx, operations_rx) =
        unbounded_channel::<Result<BytesMut, std::io::Error>>();
    let (mut input_send, input_recv) =
        unbounded_channel::<Result<(u32, BytesMut), std::io::Error>>();
    let (output_send, mut output_recv) = unbounded_channel::<(u32, Bytes)>();
    let (query_send, mut query_recv) = unbounded_channel::<Bytes>();

    let mut flow = run_topolotree(
        neighbors,
        input_recv,
        operations_rx,
        output_send,
        query_send,
    );

    #[rustfmt::skip]
    {
        simulate_input(&mut input_send, (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 2 } }))).unwrap();
        simulate_operation(&mut operations_tx, OperationPayload { key: 123, change: 5 }).unwrap();
        simulate_operation(&mut operations_tx, OperationPayload { key: 123, change: 7 }).unwrap();
    };

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 3, data: 12 } })),
        (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 3, data: 14 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 3, data: 14 } })),
        (1, TopolotreeMessage::Ping()),
        (2, TopolotreeMessage::Ping()),
        (3, TopolotreeMessage::Ping())
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 14 }
    ]));

    #[rustfmt::skip]
    {
        simulate_operation(&mut operations_tx, OperationPayload { key: 123, change: 1 }).unwrap();
    };

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 4, data: 13 } })),
        (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 4, data: 15 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 4, data: 15 } })),
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 15 }
    ]));
}

#[dfir_rs::test(start_paused = true)]
async fn operations_multiple_keys() {
    let neighbors: Vec<u32> = vec![1, 2, 3];

    let (mut operations_tx, operations_rx) =
        unbounded_channel::<Result<BytesMut, std::io::Error>>();
    let (mut _input_send, input_recv) =
        unbounded_channel::<Result<(u32, BytesMut), std::io::Error>>();
    let (output_send, mut output_recv) = unbounded_channel::<(u32, Bytes)>();
    let (query_send, mut query_recv) = unbounded_channel::<Bytes>();

    let mut flow = run_topolotree(
        neighbors,
        input_recv,
        operations_rx,
        output_send,
        query_send,
    );

    #[rustfmt::skip]
    {
        simulate_operation(&mut operations_tx, OperationPayload { key: 123, change: 5 }).unwrap();
        simulate_operation(&mut operations_tx, OperationPayload { key: 456, change: 7 }).unwrap();
    };

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 5 } })),
        (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 5 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 5 } })),

        (1, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 1, data: 7 } })),
        (2, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 1, data: 7 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 1, data: 7 } })),

        (1, TopolotreeMessage::Ping()),
        (2, TopolotreeMessage::Ping()),
        (3, TopolotreeMessage::Ping())
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 5 },
        QueryResponse { key: 456, value: 7 }
    ]));

    #[rustfmt::skip]
    {
        simulate_operation(&mut operations_tx, OperationPayload { key: 123, change: 1 }).unwrap();
    };

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 2, data: 6 } })),
        (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 2, data: 6 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 2, data: 6 } }))
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 6 }
    ]));

    #[rustfmt::skip]
    {
        simulate_operation(&mut operations_tx, OperationPayload { key: 456, change: 2 }).unwrap();
    };

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (1, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 2, data: 9 } })),
        (2, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 2, data: 9 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 2, data: 9 } }))
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 456, value: 9 }
    ]));
}

#[dfir_rs::test(start_paused = true)]
async fn gossip_multiple_keys() {
    let neighbors: Vec<u32> = vec![1, 2, 3];

    let (mut _operations_tx, operations_rx) =
        unbounded_channel::<Result<BytesMut, std::io::Error>>();
    let (mut input_send, input_recv) =
        unbounded_channel::<Result<(u32, BytesMut), std::io::Error>>();
    let (output_send, mut output_recv) = unbounded_channel::<(u32, Bytes)>();
    let (query_send, mut query_recv) = unbounded_channel::<Bytes>();

    let mut flow = run_topolotree(
        neighbors,
        input_recv,
        operations_rx,
        output_send,
        query_send,
    );

    #[rustfmt::skip]
    {
        simulate_input(&mut input_send, (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 0, data: 5 } }))).unwrap();
        simulate_input(&mut input_send, (2, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 0, data: 7 } }))).unwrap();
    };

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 5 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 1, data: 5 } })),

        (1, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 1, data: 7 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 1, data: 7 } })),

        (1, TopolotreeMessage::Ping()),
        (2, TopolotreeMessage::Ping()),
        (3, TopolotreeMessage::Ping())
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 5 },
        QueryResponse { key: 456, value: 7 },
    ]));

    #[rustfmt::skip]
    {
        simulate_input(&mut input_send, (2, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 0, data: 5 } }))).unwrap();
        simulate_input(&mut input_send, (3, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 0, data: 7 } }))).unwrap();
    };

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (1, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 2, data: 5 } })),
        (3, TopolotreeMessage::Payload(Payload { key: 123, contents: Timestamped { timestamp: 2, data: 10 } })),

        (1, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 2, data: 14 } })),
        (2, TopolotreeMessage::Payload(Payload { key: 456, contents: Timestamped { timestamp: 2, data: 7 } })),
    ]));

    #[rustfmt::skip]
    assert_eq!(read_all_query(&mut query_recv).await, HashMultiSet::from_iter([
        QueryResponse { key: 123, value: 10 },
        QueryResponse { key: 456, value: 14 },
    ]));
}

#[dfir_rs::test(start_paused = true)]
async fn ping_pongs() {
    let neighbors: Vec<u32> = vec![1];

    let (mut _operations_tx, operations_rx) =
        unbounded_channel::<Result<BytesMut, std::io::Error>>();
    let (mut _input_send, input_recv) =
        unbounded_channel::<Result<(u32, BytesMut), std::io::Error>>();
    let (output_send, mut output_recv) = unbounded_channel::<(u32, Bytes)>();
    let (query_send, mut _query_recv) = unbounded_channel::<Bytes>();

    let mut flow = run_topolotree(
        neighbors,
        input_recv,
        operations_rx,
        output_send,
        query_send,
    );

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (1, TopolotreeMessage::Ping())
    ]));

    tokio::time::advance(Duration::from_millis(500)).await;

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([]));

    tokio::time::advance(Duration::from_millis(500)).await;

    flow.run_tick();

    #[rustfmt::skip]
    assert_eq!(read_all(&mut output_recv).await, HashMultiSet::from_iter([
        (1, TopolotreeMessage::Ping())
    ]));
}
