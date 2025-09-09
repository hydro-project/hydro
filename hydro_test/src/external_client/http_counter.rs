use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
use hydro_lang::prelude::*;

#[derive(Debug, Clone)]
pub enum RequestType {
    Increment { key: i32 },
    Get { key: i32 },
    Invalid,
}

#[derive(Debug, Clone)]
pub struct ParsedRequest {
    pub connection_id: u64,
    pub request_type: RequestType,
    pub raw_request: String,
}

pub fn http_counter_server<'a, P>(
    in_stream: KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder>,
    process: &Process<'a, P>,
) -> KeyedStream<u64, String, Process<'a, P>, Unbounded, NoOrder> {
    let parsed_requests = in_stream
        .fold_early_stop(
            q!(|| String::new()),
            q!(|buffer, line| {
                buffer.push_str(&line);
                buffer.push_str("\r\n");
                // Check if this is an empty line (end of HTTP headers)
                line.trim().is_empty()
            }),
        )
        .map_with_key(q!(|(connection_id, raw_request)| {
            let lines: Vec<&str> = raw_request.lines().collect();
            let request_line = lines.first().unwrap_or(&"");
            let parts: Vec<&str> = request_line.split_whitespace().collect();
            let method = parts.first().unwrap_or(&"GET");
            let path = parts.get(1).unwrap_or(&"/");

            let request_type = if method == &"POST" && path.starts_with("/increment/") {
                if let Ok(key) = path[11..].parse::<i32>() {
                    RequestType::Increment { key }
                } else {
                    RequestType::Invalid
                }
            } else if method == &"GET" && path.starts_with("/get/") {
                if let Ok(key) = path[5..].parse::<i32>() {
                    RequestType::Get { key }
                } else {
                    RequestType::Invalid
                }
            } else {
                RequestType::Invalid
            };

            ParsedRequest {
                connection_id,
                request_type,
                raw_request,
            }
        }));

    let increment_lookup_tick = process.tick();
    let increment_stream = parsed_requests
        .clone()
        .filter_map(q!(|req| match req.request_type {
            RequestType::Increment { key } => Some(key),
            _ => None,
        }))
        .atomic(&increment_lookup_tick);

    let get_stream = parsed_requests
        .clone()
        .filter_map(q!(|req| match req.request_type {
            RequestType::Get { key } => Some(key),
            _ => None,
        }));

    let invalid_requests = parsed_requests.filter_map(q!(|req| match req.request_type {
        RequestType::Invalid => Some(req.raw_request),
        _ => None,
    }));

    let counters = increment_stream
        .clone()
        .values()
        .map(q!(|key| (key, ())))
        .into_keyed()
        .fold_commutative(q!(|| 0i32), q!(|acc, _| *acc += 1));

    let lookup_result = get_stream
        .batch(&increment_lookup_tick, nondet!(/** batch get requests */))
        .get_from(counters.snapshot(nondet!(/** intentional non-determinism for get timing */)));
    let get_responses = lookup_result
        .clone()
        .map(q!(|(key, maybe_count)| {
            if let Some(count) = maybe_count {
                format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Type: application/json\r\n\
                    Content-Length: {}\r\n\
                    Connection: close\r\n\
                    \r\n\
                    {{\"key\": {}, \"count\": {}}}",
                    format!("{{\"key\": {}, \"count\": {}}}", key, count).len(),
                    key,
                    count
                )
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\n\
                        Content-Type: application/json\r\n\
                        Content-Length: {}\r\n\
                        Connection: close\r\n\
                        \r\n\
                        {{\"key\": {}, \"count\": 0}}",
                    format!("{{\"key\": {}, \"count\": 0}}", key).len(),
                    key
                )
            }
        }))
        .into_keyed_stream()
        .all_ticks();

    // Handle increment responses (just acknowledge)
    let increment_responses = increment_stream
        .map(q!(|key| {
            format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {{\"key\": {}, \"status\": \"incremented\"}}",
                format!("{{\"key\": {}, \"status\": \"incremented\"}}", key).len(),
                key
            )
        }))
        .end_atomic();

    let invalid_responses = invalid_requests.map(q!(|_raw_request| {
        let error_body =
            "{\"error\": \"Invalid request. Use POST /increment/{key} or GET /get/{key}\"}";
        format!(
            "HTTP/1.1 400 Bad Request\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {}",
            error_body.len(),
            error_body
        )
    }));

    get_responses
        .interleave(increment_responses.into_keyed_stream())
        .interleave(invalid_responses.into_keyed_stream())
}
