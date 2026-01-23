//! # Telemetry
use tracing::Subscriber;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::{FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

struct Formatter;

impl<S, N> FormatEvent<S, N> for Formatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        use colored::Colorize;

        let metadata = event.metadata();

        if writer.has_ansi_escapes() {
            write!(
                &mut writer,
                "{} {} {}{} {} {}:{}: ",
                chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.f%:z")
                    .to_string()
                    .magenta()
                    .underline()
                    .on_white(),
                metadata.level().as_str().red(),
                std::thread::current()
                    .name()
                    .unwrap_or("unnamed-thread")
                    .blue(),
                format!("({:?})", std::thread::current().id()).blue(),
                // gettid::gettid(), TODO: can't get gettid to link properly.
                metadata.target().green(),
                metadata.file().unwrap_or("unknown-file").red(),
                format!("{}", metadata.line().unwrap_or(0)).red(),
            )?;
        } else {
            write!(
                &mut writer,
                "{} {} {}{:?} {} {}:{}: ",
                chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.f%:z"),
                metadata.level().as_str().red(),
                std::thread::current().name().unwrap_or("unnamed-thread"),
                std::thread::current().id(),
                // gettid::gettid(), TODO: can't get gettid to link properly.
                metadata.target(),
                metadata.file().unwrap_or("unknown-file"),
                metadata.line().unwrap_or(0),
            )?;
        }

        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                if writer.has_ansi_escapes() {
                    write!(writer, "{}", span.name().purple())?;
                } else {
                    write!(writer, "{}", span.name())?;
                }

                let ext = span.extensions();
                let fields = &ext.get::<FormattedFields<N>>().unwrap();

                if !fields.is_empty() {
                    if writer.has_ansi_escapes() {
                        write!(writer, "{{{}}}", fields.cyan())?;
                    } else {
                        write!(writer, "{{{}}}", fields)?;
                    }
                }

                write!(writer, ": ")?;
            }
        }

        if writer.has_ansi_escapes() {
            write!(writer, "{}: ", metadata.name().yellow().bold().underline())?;
        } else {
            write!(writer, "{}: ", metadata.name())?;
        }

        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

/// Initialize tracing using the above custom formatter with the default directive level of "ERROR", if RUST_LOG is not set.
pub fn initialize_tracing() {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|err| {
        match err {
            std::env::VarError::NotPresent => {
                // RUST_LOG not set, the user wants the default.
                "error".to_string()
            }
            std::env::VarError::NotUnicode(v) => {
                // Almost certainly there is a configuration issue.
                eprintln!(
                    "RUST_LOG is not unicode, defaulting to 'error' directive: {:?}",
                    v
                );
                "error".to_string()
            }
        }
    });

    let filter = EnvFilter::try_new(&rust_log).unwrap_or_else(|err| {
        // Configuration error.
        eprintln!("Failed to parse RUST_LOG: {}, err: {:?}", rust_log, err);
        "error".to_string().parse().unwrap()
    });

    initialize_tracing_with_filter(filter)
}

/// Initialize tracing using the above custom formatter, using the tracing directive.
/// something like "{level},{abc}={level},{xyz}={level}" where {level} is one of "tracing,debug,info,warn,error"
pub fn initialize_tracing_with_filter(filter: EnvFilter) {
    use tracing::subscriber::set_global_default;
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{Layer, fmt, registry};

    set_global_default(
        registry().with(
            fmt::layer()
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .event_format(Formatter)
                .with_filter(filter.clone()),
        ),
    )
    .unwrap();

    #[expect(
        non_snake_case,
        reason = "this variable represents an env var which is in all caps"
    )]
    let RUST_LOG = std::env::var("RUST_LOG");

    tracing::trace!(name: "Tracing Initialized", ?RUST_LOG, ?filter);
}
