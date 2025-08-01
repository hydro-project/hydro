use hydro_lang::*;

pub fn echo_server<'a, P>(
    in_stream: Stream<(u64, String), Process<'a, P>, Unbounded, NoOrder>,
) -> Stream<(u64, String), Process<'a, P>, Unbounded, NoOrder> {
    in_stream.inspect(q!(|(id, t)| println!(
        "...received request {} from client #{}, echoing back...",
        t, id
    )))
}
