use std::fmt::Debug;

#[derive(Clone)]
pub struct BacktraceElement {
    pub fn_name: String,
    pub filename: Option<String>,
    pub lineno: Option<u32>,
}

impl Debug for BacktraceElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.fn_name)?;
        if let Some(filename) = &self.filename {
            f.write_str("(")?;
            f.write_str(filename)?;
            if let Some(lineno) = self.lineno {
                f.write_str(":")?;
                f.write_fmt(format_args!("{}", lineno))?;
            }
            f.write_str(")")?;
        } else {
            f.write_str(" (unknown location)")?;
        }

        Ok(())
    }
}

#[cfg(feature = "build")]
#[inline(never)]
pub(crate) fn get_backtrace(skip_count: usize) -> Vec<BacktraceElement> {
    let backtrace = backtrace::Backtrace::new();
    backtrace
        .frames()
        .iter()
        .flat_map(|frame| frame.symbols())
        .skip(6 + skip_count)
        .take_while(|s| {
            !s.name()
                .is_some_and(|n| n.as_str().unwrap().contains("__rust_begin_short_backtrace"))
        })
        .map(|symbol| BacktraceElement {
            fn_name: symbol.name().unwrap().to_string(),
            filename: symbol.filename().map(|f| f.display().to_string()),
            lineno: symbol.lineno(),
        })
        .collect()
}

#[cfg(not(feature = "build"))]
pub(crate) fn get_backtrace() -> Vec<BacktraceElement> {
    panic!();
}
