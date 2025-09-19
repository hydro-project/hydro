use base64::Engine;
use bytes::BytesMut;
use hydro_lang::live_collections::keyed_stream::Generate;
use hydro_lang::live_collections::stream::TotalOrder;
use hydro_lang::prelude::*;
use sha1::{Digest, Sha1};

const WEBSOCKET_MAGIC_STRING: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// WebSocket frame opcodes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xa,
}

impl OpCode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(OpCode::Continuation),
            0x1 => Some(OpCode::Text),
            0x2 => Some(OpCode::Binary),
            0x8 => Some(OpCode::Close),
            0x9 => Some(OpCode::Ping),
            0xa => Some(OpCode::Pong),
            _ => None,
        }
    }
}

/// Represents a parsed WebSocket frame
#[derive(Debug, Clone)]
struct WebSocketFrame {
    _fin: bool,
    opcode: OpCode,
    _masked: bool,
    payload: Vec<u8>,
}

pub enum WebSocketMessage {
    Text(String),
    Binary(BytesMut),
}

/// Represents different types of WebSocket messages
#[derive(Debug, Clone)]
enum WebSocketRawMessage {
    HandshakeResponse(BytesMut),
    Frame(WebSocketFrame),
    Error(String),
}

/// State machine for WebSocket connections
#[derive(Debug, Clone)]
pub enum WebSocketState {
    WaitingForHandshake,
    Connected,
}

/// Connection state tracking for WebSocket protocol
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub state: WebSocketState,
    pub buffer: Vec<u8>, // Changed to Vec<u8> to handle binary data
    pub handshake_complete: bool,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            state: WebSocketState::WaitingForHandshake,
            buffer: Vec::new(),
            handshake_complete: false,
        }
    }
}

pub fn websocket_protocol<'a, P>(
    in_stream: KeyedStream<u64, BytesMut, Process<'a, P>, Unbounded, TotalOrder>,
    handle_messages: impl FnOnce(
        KeyedStream<u64, WebSocketMessage, Process<'a, P>, Unbounded>,
        KeyedSingleton<u64, (), Process<'a, P>, Unbounded>,
    ) -> KeyedStream<u64, WebSocketMessage, Process<'a, P>, Unbounded>,
) -> KeyedStream<u64, BytesMut, Process<'a, P>, Unbounded, TotalOrder> {
    let parsed_messages = in_stream.flatten_ordered().generator(
        q!(|| ConnectionState::default()),
        q!(|state, byte| {
            state.buffer.push(byte);

            match state.state {
                WebSocketState::WaitingForHandshake => {
                    // Check if we have a complete HTTP request (empty line indicates end)
                    let buffer_str = String::from_utf8_lossy(&state.buffer);
                    if buffer_str.contains("\r\n\r\n") {
                        let handshake_response =
                            self::handle_websocket_handshake_no_id(&buffer_str);
                        state.state = WebSocketState::Connected;
                        state.buffer.clear();
                        Generate::Yield(WebSocketRawMessage::HandshakeResponse(
                            handshake_response.as_bytes().into(),
                        ))
                    } else {
                        Generate::Continue
                    }
                }
                WebSocketState::Connected => {
                    // Try to parse WebSocket frames from buffer
                    if state.buffer.len() >= 2 {
                        match self::parse_websocket_frame(&state.buffer) {
                            Ok(None) => Generate::Continue,
                            Ok(Some((frame, bytes_consumed))) => {
                                // Remove the consumed bytes from the buffer
                                state.buffer.drain(0..bytes_consumed);

                                if frame.opcode == OpCode::Close {
                                    Generate::Return(WebSocketRawMessage::Frame(frame))
                                } else {
                                    Generate::Yield(WebSocketRawMessage::Frame(frame))
                                }
                            }
                            Err(e) => {
                                println!("Error parsing WebSocket frame: {}", e);
                                Generate::Return(WebSocketRawMessage::Error(e))
                            }
                        }
                    } else {
                        Generate::Continue
                    }
                }
            }
        }),
    );

    let open_connections = parsed_messages
        .clone()
        .fold(
            q!(|| false),
            q!(|connected, msg| {
                match msg {
                    WebSocketRawMessage::HandshakeResponse(_) => {
                        *connected = true;
                    }
                    WebSocketRawMessage::Frame(frame) => {
                        if frame.opcode == OpCode::Close {
                            *connected = false;
                        }
                    }
                    WebSocketRawMessage::Error(_) => {
                        *connected = false;
                    }
                }
            }),
        )
        .filter_map(q!(|c| if c { Some(()) } else { None }));

    // Split into app messages (text/binary) and protocol messages
    let app_messages = parsed_messages.clone().filter_map(q!(|msg| {
        match msg {
            WebSocketRawMessage::Frame(frame) => match frame.opcode {
                OpCode::Text => {
                    let text = String::from_utf8_lossy(&frame.payload);
                    Some(WebSocketMessage::Text(text.into()))
                }
                OpCode::Binary => Some(WebSocketMessage::Binary(frame.payload.as_slice().into())),
                _ => None,
            },
            _ => None,
        }
    }));

    let protocol_messages = parsed_messages.clone().filter_map(q!(|msg| {
        match msg {
            WebSocketRawMessage::HandshakeResponse(response) => Some(response),
            WebSocketRawMessage::Frame(frame) => {
                match frame.opcode {
                    OpCode::Ping => Some(self::create_websocket_pong_frame(&frame.payload)),
                    OpCode::Pong => {
                        None // No response needed
                    }
                    OpCode::Close => Some(self::create_websocket_close_frame()),
                    OpCode::Continuation => None,
                    _ => None, // Text and Binary handled in app_messages
                }
            }
            WebSocketRawMessage::Error(e) => {
                eprintln!("Protocol Error: {:?}", e);
                Some(self::create_websocket_close_frame())
            }
        }
    }));

    // Handle echo logic using Hydro streams
    let echo_responses = handle_messages(app_messages, open_connections);

    let encoded_responses = echo_responses.map(q!(|m| match m {
        WebSocketMessage::Text(text) => {
            self::create_websocket_text_frame(&text)
        }
        WebSocketMessage::Binary(binary) => {
            self::create_websocket_binary_frame(&binary)
        }
    }));

    // Combine protocol messages and echo responses
    protocol_messages
        .interleave(encoded_responses)
        .assume_ordering(nondet!(
            /// As long as the protocol messages are ordered wrt each other, and same for the responses,
            /// the output is indistinguishable to a websocket client.
        ))
}

/// Handle the WebSocket handshake without connection ID
fn handle_websocket_handshake_no_id(request: &str) -> String {
    let lines: Vec<&str> = request.lines().collect();

    // Parse request line
    let request_line = lines.first().unwrap_or(&"");
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let method = parts.first().unwrap_or(&"GET");
    let _path = parts.get(1).unwrap_or(&"/");

    // Extract WebSocket headers
    let mut websocket_key = None;
    let mut websocket_version = None;
    let mut connection_upgrade = false;
    let mut upgrade_websocket = false;

    for line in &lines[1..] {
        if line.trim().is_empty() {
            break;
        }
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_lowercase();
            let value = line[colon_pos + 1..].trim();

            match name.as_str() {
                "sec-websocket-key" => websocket_key = Some(value),
                "sec-websocket-version" => websocket_version = Some(value),
                "connection" => connection_upgrade = value.to_lowercase().contains("upgrade"),
                "upgrade" => upgrade_websocket = value.to_lowercase() == "websocket",
                _ => {}
            }
        }
    }

    // Validate WebSocket handshake
    if *method != "GET" || !connection_upgrade || !upgrade_websocket {
        return format!(
            "HTTP/1.1 400 Bad Request\r\n\
             Content-Type: text/plain\r\n\
             Content-Length: 21\r\n\
             Connection: close\r\n\
             \r\n\
             Invalid WebSocket request"
        );
    }

    let Some(key) = websocket_key else {
        return format!(
            "HTTP/1.1 400 Bad Request\r\n\
             Content-Type: text/plain\r\n\
             Content-Length: 28\r\n\
             Connection: close\r\n\
             \r\n\
             Missing Sec-WebSocket-Key"
        );
    };

    if websocket_version != Some("13") {
        return format!(
            "HTTP/1.1 426 Upgrade Required\r\n\
             Sec-WebSocket-Version: 13\r\n\
             Content-Type: text/plain\r\n\
             Content-Length: 38\r\n\
             Connection: close\r\n\
             \r\n\
             WebSocket version 13 required"
        );
    }

    // Generate accept key
    let accept_key = generate_websocket_accept_key(key);

    format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\
         \r\n",
        accept_key
    )
}

/// Generate the WebSocket accept key from the client's key
fn generate_websocket_accept_key(client_key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(client_key.as_bytes());
    hasher.update(WEBSOCKET_MAGIC_STRING.as_bytes());
    let hash = hasher.finalize();
    base64::engine::general_purpose::STANDARD.encode(hash)
}

/// Parse a WebSocket frame from binary data, returning the frame and bytes consumed
fn parse_websocket_frame(data: &[u8]) -> Result<Option<(WebSocketFrame, usize)>, String> {
    if data.len() < 2 {
        return Ok(None);
    }

    let first_byte = data[0];
    let second_byte = data[1];

    let fin = (first_byte & 0x80) != 0;
    let opcode = OpCode::from_u8(first_byte & 0x0f).ok_or("Invalid opcode")?;

    let masked = (second_byte & 0x80) != 0;
    let mut payload_len = (second_byte & 0x7f) as usize;

    let mut offset = 2;

    // Handle extended payload length
    if payload_len == 126 {
        if data.len() < offset + 2 {
            return Ok(None);
        }
        payload_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
    } else if payload_len == 127 {
        if data.len() < offset + 8 {
            return Ok(None);
        }
        payload_len = u64::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        offset += 8;
    }

    // Handle masking key
    let mask_key = if masked {
        if data.len() < offset + 4 {
            return Ok(None);
        }
        let key = [
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ];
        offset += 4;
        Some(key)
    } else {
        None
    };

    // Extract payload
    if data.len() < offset + payload_len {
        return Ok(None);
    }

    let mut payload = data[offset..offset + payload_len].to_vec();

    // Unmask payload if needed
    if let Some(mask) = mask_key {
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
    }

    let total_frame_size = offset + payload_len;

    Ok(Some((
        WebSocketFrame {
            _fin: fin,
            opcode,
            _masked: masked,
            payload,
        },
        total_frame_size,
    )))
}

/// Create a WebSocket text frame
fn create_websocket_text_frame(text: &str) -> BytesMut {
    create_websocket_frame(OpCode::Text, text.as_bytes())
}

fn create_websocket_binary_frame(binary: &[u8]) -> BytesMut {
    create_websocket_frame(OpCode::Binary, binary)
}

/// Create a WebSocket pong frame
fn create_websocket_pong_frame(data: &[u8]) -> BytesMut {
    create_websocket_frame(OpCode::Pong, data)
}

/// Create a WebSocket close frame
fn create_websocket_close_frame() -> BytesMut {
    create_websocket_frame(OpCode::Close, &[])
}

/// Create a WebSocket frame with the given opcode and payload
fn create_websocket_frame(opcode: OpCode, payload: &[u8]) -> BytesMut {
    let mut frame = Vec::new();

    // First byte: FIN=1, RSV=000, OpCode
    frame.push(0x80 | (opcode as u8));

    // Second byte and payload length
    let payload_len = payload.len();
    if payload_len < 126 {
        frame.push(payload_len as u8);
    } else if payload_len < 65536 {
        frame.push(126);
        frame.extend_from_slice(&(payload_len as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload_len as u64).to_be_bytes());
    }

    // Payload (no masking for server-to-client)
    frame.extend_from_slice(payload);
    frame.as_slice().into()
}
