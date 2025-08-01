use std::fmt::Debug;

use hydro_lang::*;

pub fn echo_server<'a, P, E: Debug>(
    in_stream: Stream<Result<(u64, String), E>, Process<'a, P>, Unbounded, NoOrder>,
) -> Stream<(u64, String), Process<'a, P>, Unbounded, NoOrder> {
    in_stream.map(q!(|r| r.unwrap()))
}
