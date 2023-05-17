use std::fmt::Display;
use std::net::SocketAddr;

use regex::Regex;
use serde::Serialize;
use serde_json::json;
use tokio_util::codec::LinesCodecError;

pub fn serialize_msg<T>(msg: T) -> String
where
    T: Serialize + serde::de::DeserializeOwned,
{
    json!(msg).to_string()
}

pub fn deserialize_msg<T>(msg: Result<(String, SocketAddr), LinesCodecError>) -> T
where
    T: Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_str(&(msg.unwrap().0)).unwrap()
}

use rand::Rng;
pub fn gen_bool(odds: f64) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_bool(odds)
}

pub fn parse_edge(line: String) -> Option<(u32, u32)> {
    let re = Regex::new(r"\((\d+),\s*(\d+)\)").unwrap();
    let caps = re.captures(line.as_str())?;

    Some((
        caps.get(1).unwrap().as_str().parse::<u32>().ok()?,
        caps.get(2).unwrap().as_str().parse::<u32>().ok()?,
    ))
}

pub fn format_cycle<T>(cycle: Vec<T>) -> String
where
    T: Display,
{
    let sep_str: String = cycle.iter().map(|i: &T| format!("{} -> ", i)).collect();
    format!("{}{}", sep_str, cycle[0])
}
