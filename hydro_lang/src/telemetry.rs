//! # Telemetry
use tracing::Subscriber;
use tracing_subscriber::fmt::{FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

#[expect(
    missing_docs,
    reason = "This is internal code. This struct needs to be pub for some reason for the Formatter impl to work in staged code?"
)]
pub struct Formatter;

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

        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                write!(writer, "{}", span.name().purple())?;

                let ext = span.extensions();
                let fields = &ext.get::<FormattedFields<N>>().unwrap();

                if !fields.is_empty() {
                    write!(writer, "{{{}}}", fields.cyan())?;
                }

                write!(writer, ": ")?;
            }
        }

        write!(writer, "{}: ", metadata.name().yellow().bold().underline())?;

        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

/// Initialize tracing use the above custom formatter with the default "trace" directive, if RUST_LOG is not set.
pub fn initialize_tracing() {
    use tracing_subscriber::filter::EnvFilter;

    initialize_tracing_with_directive(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("trace"))
            .to_string(),
    );
}

/// Initialize tracing use the above custom formatter, using the tracing directive.
/// something like "{level},{abc}={level},{xyz}={level}" where {level} is one of "tracing,debug,info,warn,error"
pub fn initialize_tracing_with_directive(directive: impl AsRef<str>) {
    use tracing::subscriber::set_global_default;
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{Layer, fmt, registry};

    let filter = EnvFilter::new(directive.as_ref());

    set_global_default(
        registry().with(
            fmt::layer()
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .event_format(Formatter)
                .with_filter(filter),
        ),
    )
    .unwrap();

    tracing::trace!("Tracing Initialized");
}
