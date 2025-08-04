use hydro_lang::keyed_stream::KeyedStream;
use hydro_lang::*;

pub fn echo_server<'a, P>(
    in_stream: KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder>,
) -> KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder> {
    in_stream.inspect_with_key(q!(|(id, t)| println!(
        "...received request {} from client #{}, echoing back...",
        t, id
    )))
}
