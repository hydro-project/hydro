//! TODO:
use tracing::Subscriber;
use tracing_subscriber::fmt::{FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

/// TODO:
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
        let metadata = event.metadata();

        write!(
            &mut writer,
            "{} {} {:?} {} {}:{} {}: ",
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.f%:z"),
            metadata.level(),
            std::thread::current().id(),
            std::thread::current().name().unwrap_or("unnamed-thread"),
            // gettid::gettid(), TODO: can't get gettid to link properly.
            metadata.file().unwrap_or("unknown-file"),
            metadata.line().unwrap_or(0),
            metadata.target(),
        )?;

        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                write!(writer, "{}", span.name())?;

                let ext = span.extensions();
                let fields = &ext.get::<FormattedFields<N>>().unwrap();

                if !fields.is_empty() {
                    write!(writer, "{{{}}}", fields)?;
                }

                write!(writer, ": ")?;
            }
        }

        write!(writer, "{}: ", metadata.name())?;

        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}
