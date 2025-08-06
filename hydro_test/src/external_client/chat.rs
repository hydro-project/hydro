use std::hash::{DefaultHasher, Hash, Hasher};

use colored::{Color, Colorize};
use hydro_lang::keyed_stream::KeyedStream;
use hydro_lang::location::MembershipEvent;
use hydro_lang::*;
use hydro_std::membership::track_membership;
use palette::{FromColor, Hsv, Srgb};

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

// To enable colored output in the terminal, set the environment variable
// `CLICOLOR_FORCE=1`. By default, the `colored` crate only applies color
// when the output is a terminal, to avoid issues with terminals that do
// not support color.
pub fn chat_server<'a, P>(
    process: &Process<'a, P>,
    in_stream: KeyedStream<u64, String, Process<'a, P>, Unbounded>,
    membership: KeyedStream<u64, MembershipEvent, Process<'a, P>, Unbounded>,
) -> KeyedStream<u64, String, Process<'a, P>, Unbounded, NoOrder> {
    let current_members = track_membership(membership);

    let tick = process.tick();

    unsafe {
        current_members
            .snapshot(&tick)
            .keys()
            .cross_product(in_stream.entries().tick_batch(&tick))
            .into_keyed()
    }
    .all_ticks()
    .filter_map_with_key(q!(|(recipient_id, (sender_id, line))| {
        if sender_id != recipient_id {
            Some(format!(
                "From {}: {:}",
                sender_id,
                line.color(self::hash_to_color(sender_id + 10))
            ))
        } else {
            None
        }
    }))
}
