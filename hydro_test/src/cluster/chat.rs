use std::hash::{DefaultHasher, Hash, Hasher};

use colored::{Color, Colorize};
use hydro_lang::*;
use palette::{FromColor, Hsv, Srgb};

pub struct Server {}

pub struct Clients {}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMsg {
    pub content: String,
}

// To enable colored output in the terminal, set the environment variable
// `CLICOLOR_FORCE=1`. By default, the `colored` crate only applies color
// when the output is a terminal, to avoid issues with terminals that do
// not support color.
pub fn chat_server<'a>(flow: &FlowBuilder<'a>) -> (Process<'a, Server>, Cluster<'a, Clients>) {
    // For testing, use a fixed cluster of clients. In future we should support unbounded clients.
    // This is a workaround for the fact that we don't have a way to create a cluster of
    // unbounded clients in the current version of hydro.

    let clients = flow.cluster::<Clients>();
    // Assume single server.
    let server = flow.process::<Server>();

    // 1 chat message is generated from each client
    let client_requests = clients
        .source_iter(q!([ChatMsg {
            content: format!("Hi, it's me! Client #{}!", CLUSTER_SELF_ID.raw_id)
        }]))
        .send_bincode(&server)
        .clone()
        .inspect(q!(|(id, msg)| println!(
            "...forwarding chat {} from client #{}...",
            msg.content, id
        )));
    client_requests
        .broadcast_bincode(&clients)
        .for_each(q!(|(id, msg)| println!(
            "From {}: {:}",
            id.raw_id,
            msg.content.color(self::hash_to_color(id.raw_id + 10))
        )));

    (server, clients)
}

fn hash_to_color<T: Hash>(input: T) -> Color {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let hash = hasher.finish();

    // Map hash to a hue between 0–360
    let hue = (hash % 360) as f32;
    let hsv = Hsv::new(hue, 1.0, 1.0);
    let rgb: Srgb<u8> = Srgb::from_color(hsv).into_format();

    Color::TrueColor {
        r: rgb.red,
        g: rgb.green,
        b: rgb.blue,
    }
}
