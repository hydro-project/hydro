use std::fmt::Debug;

#[derive(Clone)]
pub struct BacktraceElement {
    pub fn_name: String,
    pub filename: Option<String>,
    pub lineno: Option<u32>,
    pub addr: Option<usize>,
}

impl Debug for BacktraceElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.fn_name)
    }
}

#[cfg(feature = "build")]
#[inline(never)]
pub(crate) fn get_backtrace(skip_count: usize) -> Vec<BacktraceElement> {
    let backtrace = backtrace::Backtrace::new();
    backtrace
        .frames()
        .iter()
        .skip_while(|f| {
            !(f.symbol_address() as usize == get_backtrace as usize
                || f.symbols()
                    .first()
                    .and_then(|s| s.name())
                    .and_then(|n| n.as_str())
                    .is_some_and(|n| n.contains("get_backtrace")))
        })
        .skip(1)
        .take_while(|f| {
            !f.symbols()
                .last()
                .and_then(|s| s.name())
                .and_then(|n| n.as_str())
                .is_some_and(|n| n.contains("__rust_begin_short_backtrace"))
        })
        .flat_map(|frame| frame.symbols())
        .skip(skip_count)
        .map(|symbol| {
            let full_fn_name = symbol.name().unwrap().to_string();
            BacktraceElement {
                fn_name: full_fn_name
                    .rfind("::")
                    .map(|idx| full_fn_name.split_at(idx).0.to_string())
                    .unwrap_or(full_fn_name),
                filename: symbol.filename().map(|f| f.display().to_string()),
                lineno: symbol.lineno(),
                addr: symbol.addr().map(|a| a as usize),
            }
        })
        .collect()
}

#[cfg(not(feature = "build"))]
pub(crate) fn get_backtrace(_skip_count: usize) -> Vec<BacktraceElement> {
    panic!();
}
