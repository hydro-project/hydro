use hydro_lang::live_collections::keyed_singleton::{BoundedValue, KeyedSingleton};
use hydro_lang::live_collections::stream::TotalOrder;
use hydro_lang::prelude::*;

pub fn http_serve_static<'a, P>(
    in_stream: KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder>,
    content: &'static str,
) -> KeyedSingleton<u64, String, Process<'a, P>, BoundedValue> {
    in_stream
        .fold_early_stop(
            q!(|| String::new()),
            q!(|buffer, line| {
                buffer.push_str(&line);
                buffer.push_str("\r\n");

                // Check if this is an empty line (end of HTTP headers)
                line.trim().is_empty()
            }),
        )
        .map(q!(move |_| {
            format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/html; charset=utf-8\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {}",
                content.len(),
                content
            )
        }))
}
