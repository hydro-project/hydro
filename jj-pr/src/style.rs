use anstyle::{AnsiColor, Effects, Style};

const BOLD: Style = Style::new().effects(Effects::BOLD);
const DIM: Style = Style::new().effects(Effects::DIMMED);
const PR_NUM: Style = Style::new()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Cyan)))
    .effects(Effects::BOLD);
const BOOKMARK: Style = Style::new().effects(Effects::BOLD);
const DRAFT: Style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Yellow)));
const READY: Style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)));
const WARN: Style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Yellow)));

fn use_color() -> bool {
    anstream::AutoStream::choice(&std::io::stderr()) != anstream::ColorChoice::Never
}

fn osc8(url: &str, text: &str) -> String {
    if use_color() {
        format!("\x1b]8;;{url}\x07{text}\x1b]8;;\x07")
    } else {
        text.to_owned()
    }
}

fn styled(style: Style, text: &str) -> String {
    if use_color() {
        format!("{style}{text}{style:#}")
    } else {
        text.to_owned()
    }
}

/// Format a PR number as a clickable hyperlink (if URL provided) with bold cyan styling.
pub fn pr_num(number: u64, url: Option<&str>) -> String {
    let text = format!("PR #{number}");
    let linked = match url {
        Some(u) => osc8(u, &text),
        None => text,
    };
    styled(PR_NUM, &linked)
}

pub fn bookmark(name: &str) -> String {
    styled(BOOKMARK, name)
}

pub fn status(is_draft: bool) -> String {
    if is_draft {
        styled(DRAFT, "(draft)")
    } else {
        styled(READY, "(ready)")
    }
}

pub fn change_id(id: &str) -> String {
    let short = &id[..12.min(id.len())];
    styled(DIM, short)
}

pub fn bold(text: &str) -> String {
    styled(BOLD, text)
}

pub fn warn(text: &str) -> String {
    styled(WARN, text)
}
