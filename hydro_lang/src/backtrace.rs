use std::cell::RefCell;
use std::fmt::Debug;

#[derive(Clone)]
pub struct Backtrace {
    // TODO(shadaj): figure out how to make these not pub in Stageleft
    #[cfg(feature = "build")]
    pub skip_count: usize,
    #[cfg(feature = "build")]
    pub inner: RefCell<backtrace::Backtrace>,
    #[cfg(feature = "build")]
    pub resolved: RefCell<Option<Vec<BacktraceElement>>>,
}

impl Backtrace {
    #[cfg(feature = "build")]
    pub fn elements(&self) -> Vec<BacktraceElement> {
        self.resolved
            .borrow_mut()
            .get_or_insert_with(|| {
                let mut inner_borrow = self.inner.borrow_mut();
                inner_borrow.resolve();
                inner_borrow
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
                            addr: symbol.addr().map(|a| a as usize),
                        }
                    })
                    .collect()
            })
            .clone()
    }
}

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
pub(crate) fn get_backtrace(skip_count: usize) -> Backtrace {
    let backtrace = backtrace::Backtrace::new_unresolved();
    Backtrace {
        skip_count,
        inner: RefCell::new(backtrace),
        resolved: RefCell::new(None),
    }
}

#[cfg(not(feature = "build"))]
pub(crate) fn get_backtrace(_skip_count: usize) -> Backtrace {
    panic!();
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_backtrace() {
        let backtrace = get_backtrace(0);
        let elements = backtrace.elements();
        insta::assert_debug_snapshot!(elements);
    }
}
