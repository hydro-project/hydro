use std::path::PathBuf;

use config::{Config, ConfigError, File};
use dfir_rs::futures::future::ready;
use dfir_rs::futures::{Stream, StreamExt};
use notify::{Event, EventHandler, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tracing::trace;

/// L0 Data Store settings.
#[derive(Debug, Deserialize, Serialize)]
pub struct ServerSettings {
    /// An initial set of "seed nodes" that can be used to bootstrap the gossip cluster. These
    /// won't be the only nodes in the cluster, but they can be used to discover other nodes.
    pub seed_nodes: Vec<SeedNodeSettings>,
}

const CONFIG_ROOT: &str = "config";
const STATIC_CONFIG_PATH: &str = "static";
const DYNAMIC_CONFIG_PATH: &str = "dynamic";

fn static_config_path(subpath: &str) -> PathBuf {
    PathBuf::from(CONFIG_ROOT)
        .join(STATIC_CONFIG_PATH)
        .join(subpath)
}

fn dynamic_config_path(subpath: &str) -> PathBuf {
    PathBuf::from(CONFIG_ROOT)
        .join(DYNAMIC_CONFIG_PATH)
        .join(subpath)
}

impl ServerSettings {
    /// Load the settings from the configuration files.
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let settings = Config::builder()
            /* Load the default settings from the `config/default.toml` file. */
            .add_source(File::from(static_config_path("default.toml")).required(false))

            /* Load additional overrides based on context (alpha, beta, production, etc.), if they exist. */
            .add_source(File::from(static_config_path(&run_mode)).required(false))

            /* Load the local settings, if they exist. These are .gitignored to prevent accidental
               check-in. */
            .add_source(File::from(static_config_path("local")).required(false))

            /* Load the dynamic settings, if they exist. These always override any static configuration*/
            .add_source(File::from(dynamic_config_path("dynamic.toml")).required(false))
            .build()?;

        settings.try_deserialize()
    }
}

/// A seed node that can be used to bootstrap the gossip cluster.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct SeedNodeSettings {
    /// The ID of the seed node.
    pub id: String,

    /// The address on which the seed node is listening for gossip messages.
    pub address: String,
}

/// Setup a watcher for the settings files and return a stream of settings changes.
///
/// Returns the watcher, the initial settings, and a stream of settings changes. The watcher is
/// returned so that it can be kept alive for the lifetime of the application. Also returns a
/// snapshot of the current settings.
pub fn setup_settings_watch() -> (
    RecommendedWatcher,
    ServerSettings,
    impl Stream<Item = ServerSettings>,
) {
    let (tx, rx) = dfir_rs::util::unbounded_channel();

    // Setup the watcher
    let mut watcher = RecommendedWatcher::new(
        UnboundedSenderEventHandler::new(tx),
        notify::Config::default(),
    )
    .unwrap();
    watcher
        .watch(&PathBuf::from(CONFIG_ROOT), RecursiveMode::Recursive)
        .unwrap();

    // Read initial settings
    let initial_settings = ServerSettings::new().unwrap();

    let change_stream = rx
        .map(Result::unwrap)
        .map(|event| {
            trace!("Event: {:?}", event);
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    Some(ServerSettings::new().unwrap())
                }
                _ => {
                    trace!("Unhandled event: {:?}", event);
                    None
                }
            }
        })
        .filter_map(ready);

    // If the watcher is dropped, the stream will stop producing events. So, returning the watcher.
    (watcher, initial_settings, change_stream)
}

/// Wraps an UnboundedSender to implement the notify::EventHandler trait. This allows sending
/// file notification evnts to UnboundedSender instances.
struct UnboundedSenderEventHandler {
    tx: UnboundedSender<notify::Result<Event>>,
}

impl UnboundedSenderEventHandler {
    fn new(tx: UnboundedSender<notify::Result<Event>>) -> Self {
        Self { tx }
    }
}

impl EventHandler for UnboundedSenderEventHandler {
    fn handle_event(&mut self, event: notify::Result<Event>) {
        self.tx.send(event).unwrap();
    }
}
