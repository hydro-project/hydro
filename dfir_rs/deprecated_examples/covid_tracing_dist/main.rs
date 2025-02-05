use clap::{ArgEnum, Parser};
use database::run_database;
use dfir_rs::tokio;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracker::run_tracker;

mod database;
mod people;
mod tracker;

// This is a distributed version of the covid tracing app. It somewhat
// arbitrarily splits apart two "responsibilities" of the app to exercise
// network communication.

#[derive(Clone, ArgEnum, Debug)]
enum Role {
    // The Database role is responsible for all the actual interfacing with the
    // external world: it knows about all the people, diagnoses, and contacts,
    // and ships off the diagnoses and contacts to the Tracker. It then receives
    // back from the tracker the IDs of people who should be notified, which are
    // then joined to look up the name and phone number of the individual.
    Database,
    // The tracker takes information from the Database and runs a reachability
    // computation on it, and sends back the IDs of people who should be
    // notified.
    Tracker,
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(long)]
    name: String,
    #[clap(arg_enum, long)]
    role: Role,
    #[clap(long)]
    port: usize,
    #[clap(long)]
    addr: String,
}

// TODO(justin): this trait kind of sucks but it's a placeholder.
trait Encode {
    fn encode(&self, v: &mut Vec<u8>);
}

impl<T> Encode for T
where
    T: Serialize,
{
    fn encode(&self, v: &mut Vec<u8>) {
        v.extend(serde_json::to_vec(self).unwrap());
    }
}

trait Decode {
    fn decode(v: bytes::Bytes) -> Self;
}

// TODO(justin): figure out how to do this without DeserializeOwned
impl<T> Decode for T
where
    T: 'static + DeserializeOwned,
{
    fn decode(v: bytes::Bytes) -> Self {
        let st = std::str::from_utf8(&v).unwrap().to_owned();
        serde_json::from_str(&st).unwrap()
    }
}

const CONTACTS_ADDR: u32 = 0;
const DIAGNOSES_ADDR: u32 = 1;

#[dfir_rs::main]
async fn main() {
    let opts = Opts::parse();

    match opts.role {
        Role::Database => {
            run_database(opts).await;
        }
        Role::Tracker => {
            run_tracker(opts).await;
        }
    }
}
