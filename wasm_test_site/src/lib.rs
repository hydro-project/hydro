use futures::channel::mpsc;
use futures::stream::StreamExt;
use pharos::Observable;
use pharos::ObserveConfig;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, HtmlElement, HtmlInputElement};
use ws_stream_wasm::WsMessage;
use ws_stream_wasm::WsMeta; // Use the correct future module from gloo

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    // Create a pair of channels for sending messages to the server
    let (send_tx, send_rx) = mpsc::unbounded::<String>();

    // Create a pair of channels for receiving messages from the server
    let (recv_tx, mut recv_rx) = mpsc::unbounded::<String>();

    // Spawn an async task to handle receiving messages from the server
    spawn_local(async move {
        // Get the document and chat container
        let window = window().expect("should have a window in this context");
        let document = window.document().expect("window should have a document");
        let chat_container = document
            .get_element_by_id("chat-container")
            .expect("document should have a chat container with id 'chat-container'")
            .dyn_into::<HtmlElement>()
            .unwrap();

        // Process each message received from the server (via recv_rx)
        while let Some(message) = recv_rx.next().await {
            let new_message_element = document.create_element("div").unwrap();
            new_message_element.set_text_content(Some(&message));
            chat_container.append_child(&new_message_element).unwrap();
            chat_container.set_scroll_top(chat_container.scroll_height()); // Scroll to bottom
        }
    });

    // Set up the event listener for the Send button to send messages to the server
    setup_send_button(send_tx)?;

    spawn_local(async move {
        let (_ws_meta, ws_sink_stream) = WsMeta::connect("ws://127.0.0.1:59063", None).await.unwrap();
        let (ws_sink, ws_stream) = ws_sink_stream.split();
        // let evts = ws.observe(ObserveConfig::default()).await.unwrap();

        let mut df = hydroflow::hydroflow_syntax! {

            source_stream(send_rx) -> map(|s| WsMessage::Text(s)) -> dest_sink(ws_sink);
            source_stream(ws_stream) -> map(|msg| match msg {
                WsMessage::Text(s) => s,
                WsMessage::Binary(bytes) => format!("{:?}", bytes),
            }) -> dest_sink(recv_tx);
        };

        let local = hydroflow::tokio::task::LocalSet::new();
        local.spawn_local(async move { df.run_async().await });
        local.await
    });

    Ok(())
}

// Set up the Send button to send messages to the server
fn setup_send_button(send_tx: mpsc::UnboundedSender<String>) -> Result<(), JsValue> {
    // Get the window and document objects
    let window = window().expect("should have a window in this context");
    let document = window.document().expect("window should have a document");

    // Get the send button and message input box
    let send_button = document
        .get_element_by_id("send-button")
        .expect("document should have a send button with id 'send-button'")
        .dyn_into::<HtmlElement>()?;
    let input_box = document
        .get_element_by_id("new-message")
        .expect("document should have an input box with id 'new-message'")
        .dyn_into::<HtmlInputElement>()?;

    // Clone the input_box and send_tx for use inside the closure
    let input_box_clone = input_box.clone();
    let send_tx_clone = send_tx.clone();

    // Create the closure that will send a message when the button is clicked
    let closure = Closure::wrap(Box::new(move || {
        let message = input_box_clone.value();
        if !message.is_empty() {
            // Send the message to the channel for server dispatching
            send_tx_clone.unbounded_send(message.clone()).unwrap();
            // Clear the input box after sending the message
            input_box_clone.set_value("");
        }
    }) as Box<dyn FnMut()>);

    // Attach the event listener to the send button
    send_button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;

    // Keep the closure alive
    closure.forget();

    Ok(())
}
