//! Direct polling of the AppConfigData runtime API — no local agent
//! sidecar needed. Each call to [`appconfig_bool_stream`] returns a
//! [`futures::Stream`] that:
//!
//! 1. Lazily creates an AppConfigData client (default AWS config: region
//!    + creds from env / task role).
//! 2. Starts a configuration session via `StartConfigurationSession`.
//! 3. On each tick, calls `GetLatestConfiguration` with the running
//!    session token. If the response carries fresh content, parses it
//!    as `"true"` / `"false"` and yields the value. If it's a no-change
//!    tick (empty content), re-yields the last-known value. If the call
//!    errors (network blip, throttling, missing profile in local dev),
//!    yields the last-known value unchanged.
//! 4. Honors the server-suggested `next_poll_interval_in_seconds`
//!    between ticks, falling back to [`DEFAULT_POLL_INTERVAL`] if the
//!    server does not provide one.
//!
//! Intended to be consumed by `Location::source_stream(...)` + `.fold(...)`
//! to expose the flag as an `Unbounded` `Singleton` inside the dataflow.
//! See the `appconfig_bool_flag` wrapper in `lib.rs`.

use std::time::Duration;

use aws_sdk_appconfigdata::Client;
use futures::Stream;

/// Default tick interval if the AppConfigData service doesn't suggest
/// one. AppConfigData's minimum is 15 seconds.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Produce a stream of boolean values polled from an AppConfig
/// configuration profile. See the module docs for behavior on errors
/// and no-change ticks.
///
/// Resource identifiers are read from environment variables rather than
/// arguments so that the same binary can run in multiple AppConfig
/// applications/environments (dev, beta, prod) without recompilation.
/// The CDK stack is expected to set the following on each task:
///
/// - `APPCONFIG_APPLICATION_ID` — shared across all flags.
/// - `APPCONFIG_ENVIRONMENT_ID` — shared across all flags.
/// - `profile_env_var` — the profile id for this specific flag.
///
/// If any of the three variables is missing or empty the stream stays
/// idle (never yields). This lets local/sim runs and development
/// environments run without an AppConfig setup.
pub fn appconfig_bool_stream(profile_env_var: &'static str) -> impl Stream<Item = bool> + Unpin {
    Box::pin(async_stream::stream! {
        let application = std::env::var("APPCONFIG_APPLICATION_ID").unwrap_or_default();
        let environment = std::env::var("APPCONFIG_ENVIRONMENT_ID").unwrap_or_default();
        let profile = std::env::var(profile_env_var).unwrap_or_default();
        if application.is_empty() || environment.is_empty() || profile.is_empty() {
            tracing::info!(
                name: "appconfig_bool_stream_disabled",
                %profile_env_var,
                application_set = !application.is_empty(),
                environment_set = !environment.is_empty(),
                profile_set = !profile.is_empty(),
            );
            // Never yield — the downstream fold keeps its initial value.
            // `pending()` parks forever without consuming CPU.
            std::future::pending::<()>().await;
            return;
        }

        let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&shared_config);

        let mut session_token: Option<String> = None;
        let mut last_value: Option<bool> = None;
        let mut next_interval = DEFAULT_POLL_INTERVAL;

        loop {
            // (Re)establish session if we don't have a token yet.
            if session_token.is_none() {
                match client
                    .start_configuration_session()
                    .application_identifier(application.clone())
                    .environment_identifier(environment.clone())
                    .configuration_profile_identifier(profile.clone())
                    .send()
                    .await
                {
                    Ok(resp) => {
                        session_token = resp.initial_configuration_token;
                    }
                    Err(err) => {
                        tracing::warn!(name: "appconfig_session_failed", ?err);
                        tokio::time::sleep(next_interval).await;
                        continue;
                    }
                }
            }

            let token = match session_token.clone() {
                Some(t) => t,
                None => {
                    tokio::time::sleep(next_interval).await;
                    continue;
                }
            };

            match client
                .get_latest_configuration()
                .configuration_token(token)
                .send()
                .await
            {
                Ok(resp) => {
                    // Roll the session token forward.
                    session_token = resp.next_poll_configuration_token;

                    if resp.next_poll_interval_in_seconds > 0 {
                        next_interval =
                            Duration::from_secs(resp.next_poll_interval_in_seconds as u64);
                    }

                    // `configuration` is Some(Blob) only when the profile
                    // changed since the last poll; otherwise it's empty
                    // bytes and we just re-yield last_value.
                    if let Some(blob) = resp.configuration {
                        let bytes = blob.into_inner();
                        if !bytes.is_empty() {
                            let text = String::from_utf8_lossy(&bytes);
                            let parsed = match text.trim() {
                                "true" => Some(true),
                                "false" => Some(false),
                                other => {
                                    tracing::warn!(name: "appconfig_bool_parse_failed", raw = %other);
                                    None
                                }
                            };
                            if let Some(v) = parsed {
                                last_value = Some(v);
                            }
                        }
                    }
                }
                Err(err) => {
                    // On bad-request-type errors the session token is
                    // invalidated — force re-StartConfigurationSession on
                    // the next tick.
                    tracing::warn!(name: "appconfig_poll_failed", ?err);
                    session_token = None;
                }
            }

            if let Some(v) = last_value {
                yield v;
            }
            tokio::time::sleep(next_interval).await;
        }
    })
}
