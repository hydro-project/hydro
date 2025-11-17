//! Platform-independent interface for collecting backtraces, used in the Hydro IR to
//! trace the origin of each node.

#[cfg(feature = "build")]
use std::cell::RefCell;
#[cfg(feature = "build")]
use std::fmt::Debug;

#[cfg(not(feature = "build"))]
/// A dummy backtrace element with no data. Enable the `build` feature to collect backtraces.
#[derive(Clone)]
pub struct Backtrace;

#[cfg(feature = "build")]
/// Captures an entire backtrace, whose elements will be lazily resolved. See
/// [`Backtrace::elements`] for more information.
#[derive(Clone)]
pub struct Backtrace {
    skip_count: usize,
    col_offset: usize, // whether this is from `sliced!` which requires an offset
    inner: RefCell<backtrace::Backtrace>,
    resolved: RefCell<Option<Vec<BacktraceElement>>>,
}

#[cfg(stageleft_runtime)]
#[cfg(feature = "build")]
#[doc(hidden)]
pub fn __macro_get_backtrace(col_offset: usize) -> Backtrace {
    let mut out = Backtrace::get_backtrace(1);
    out.col_offset = col_offset;
    out
}

#[cfg(not(feature = "build"))]
#[doc(hidden)]
pub fn __macro_get_backtrace(_col_offset: usize) -> Backtrace {
    panic!();
}

impl Backtrace {
    #[cfg(feature = "build")]
    #[inline(never)]
    pub(crate) fn get_backtrace(skip_count: usize) -> Backtrace {
        let backtrace = backtrace::Backtrace::new_unresolved();
        Backtrace {
            skip_count,
            col_offset: 0,
            inner: RefCell::new(backtrace),
            resolved: RefCell::new(None),
        }
    }

    #[cfg(not(feature = "build"))]
    pub(crate) fn get_backtrace(_skip_count: usize) -> Backtrace {
        panic!();
    }

    /// Gets the elements of the backtrace including inlined frames.
    ///
    /// Excludes all backtrace elements up to the original `get_backtrace` call as
    /// well as additional skipped frames from that call. Also drops the suffix
    /// of frames from `__rust_begin_short_backtrace` onwards.
    #[cfg(feature = "build")]
    pub fn elements(&self) -> Vec<BacktraceElement> {
        self.resolved
            .borrow_mut()
            .get_or_insert_with(|| {
                let mut inner_borrow = self.inner.borrow_mut();
                inner_borrow.resolve();
                let mut collected: Vec<_> = inner_borrow
                    .frames()
                    .iter()
                    .skip_while(|f| {
                        !(std::ptr::eq(f.symbol_address(), Backtrace::get_backtrace as _)
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
                    .skip(self.skip_count)
                    .map(|symbol| {
                        let full_fn_name = symbol.name().unwrap().to_string();
                        BacktraceElement {
                            fn_name: full_fn_name
                                .rfind("::")
                                .map(|idx| full_fn_name.split_at(idx).0.to_string())
                                .unwrap_or(full_fn_name),
                            filename: symbol.filename().map(|f| f.display().to_string()),
                            lineno: symbol.lineno(),
                            colno: symbol.colno(),
                            addr: symbol.addr().map(|a| a as usize),
                        }
                    })
                    .collect();

                if self.col_offset > 0
                    && let Some(first) = collected.first_mut()
                {
                    first.colno = first
                        .colno
                        .map(|c| c.saturating_sub(self.col_offset as u32));
                }

                collected
            })
            .clone()
    }
}

#[cfg(feature = "build")]
/// A single frame of a backtrace, corresponding to a single function call.
#[derive(Clone)]
pub struct BacktraceElement {
    /// The name of the function that was called.
    pub fn_name: String,
    /// The path to the file where this call occured.
    pub filename: Option<String>,
    /// The line number of the function call.
    pub lineno: Option<u32>,
    /// The column number of the function call.
    pub colno: Option<u32>,
    /// The address of the instruction corresponding to this function call.
    pub addr: Option<usize>,
}

#[cfg(feature = "build")]
impl Debug for BacktraceElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // filename / addr is unstable across platforms so we drop it
        f.debug_struct("BacktraceElement")
            .field("fn_name", &self.fn_name)
            .field("lineno", &self.lineno)
            .field("colno", &self.colno)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "build")]
    use super::*;

    #[cfg(feature = "build")]
    #[test]
    fn test_backtrace() {
        if cfg!(not(target_os = "linux")) && std::env::var_os("GITHUB_ACTIONS").is_some() {
            eprintln!("Backtrace tests fail on non-linux Github Actions runners, skipping.");
            return;
        }

        let backtrace = Backtrace::get_backtrace(0);
        let elements = backtrace.elements();

        hydro_build_utils::assert_debug_snapshot!(elements);
    }
}
