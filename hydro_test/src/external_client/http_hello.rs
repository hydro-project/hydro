use hydro_lang::live_collections::keyed_singleton::{BoundedValue, KeyedSingleton};
use hydro_lang::live_collections::stream::TotalOrder;
use hydro_lang::prelude::*;

pub fn http_hello_server<'a, P>(
    in_stream: KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder>,
) -> KeyedSingleton<u64, String, Process<'a, P>, BoundedValue> {
    in_stream
        .inspect_with_key(q!(|(id, line)| println!(
            "Received line from client #{}: '{}'",
            id, line
        )))
        .fold_early_stop(
            q!(|| String::new()),
            q!(|buffer, line| {
                buffer.push_str(&line);
                buffer.push_str("\r\n");

                // Check if this is an empty line (end of HTTP headers)
                line.trim().is_empty()
            }),
        )
        .map_with_key(q!(|(id, raw_request)| {
            let lines: Vec<&str> = raw_request.lines().collect();

            // Parse request line
            let request_line = lines.first().unwrap_or(&"");
            let parts: Vec<&str> = request_line.split_whitespace().collect();
            let method = parts.first().unwrap_or(&"GET");
            let path = parts.get(1).unwrap_or(&"/");
            let version_str = parts.get(2).unwrap_or(&"HTTP/1.1");
            let version = if version_str.ends_with("1.0") { 0 } else { 1 };

            // Extract specific headers we need
            let mut user_agent = "Unknown";
            let mut host = "Unknown";

            for line in &lines[1..] {
                if line.trim().is_empty() {
                    break;
                }
                if let Some(colon_pos) = line.find(':') {
                    let name = line[..colon_pos].trim();
                    let value = line[colon_pos + 1..].trim();

                    match name.to_lowercase().as_str() {
                        "user-agent" => user_agent = value,
                        "host" => host = value,
                        _ => {}
                    }
                }
            }

            let html_content = format!(
                "<!DOCTYPE html>\
                <html><head><title>Hello from Hydro!</title></head>\
                <body>\
                <h1>🌊 Hello from Hydro HTTP Server!</h1>\
                <p><strong>Your browser:</strong> {}</p>\
                <p><strong>Host:</strong> {}</p>\
                <p><strong>Method:</strong> {}</p>\
                <p><strong>Path:</strong> {}</p>\
                <p><strong>HTTP Version:</strong> 1.{}</p>\
                <p><em>Connection #{}</em></p>\
                <hr>\
                <h3>Raw Request:</h3>\
                <pre>{}</pre>\
                </body></html>",
                user_agent, host, method, path, version, id, raw_request
            );

            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/html; charset=utf-8\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {}",
                html_content.len(),
                html_content
            );
            response
        }))
        .inspect_with_key(q!(|(id, response)| println!(
            "Sending HTTP response to client #{}: {} bytes",
            id,
            response.len()
        )))
}
