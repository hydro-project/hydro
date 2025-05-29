//! Declarative macros.

/// [`assert!`] but returns a [`Result<(), String>`] instead of panicking.
#[macro_export]
macro_rules! rassert {
    ($cond:expr $(,)?) => {
        $crate::rassert!($cond, "assertion failed: `{}`", stringify!($cond))
    };
    ($cond:expr, $fmt:literal) => {
        $crate::rassert!($cond, $fmt,)
    };
    ($cond:expr, $fmt:literal, $($arg:tt)*) => {
        {
            if $cond {
                Ok(())
            }
            else {
                Err(format!($fmt, $($arg)*))
            }
        }
    };
}

/// [`assert_eq!`] but returns a [`Result<(), String>`] instead of panicking.
#[macro_export]
macro_rules! rassert_eq {
    ($a:expr, $b:expr) => {
        $crate::rassert!($a == $b,)
    };
    ($a:expr, $b:expr, $($arg:tt)*) => {
        $crate::rassert!($a == $b, $($arg)*)
    };
}

/// Asserts that the variable's type implements the given traits.
#[macro_export]
macro_rules! assert_var_impl {
    ($var:ident: $($trait:path),+ $(,)?) => {
        let _ = || {
            // Only callable when `$var` implements all traits in `$($trait)+`.
            fn assert_var_impl<T: ?Sized $(+ $trait)+>(_x: &T) {}
            assert_var_impl(& $var);
        };
    };
}

/// Tests that the given warnings are emitted by the dfir macro invocation.
///
/// For example usage, see `dfir/tests/surface_warnings.rs`.
#[macro_export]
macro_rules! dfir_expect_warnings {
    (
        $hf:tt,
        $( $msg:literal ),*
        $( , )?
    ) => {
        {
            fn emit(msg: impl ::std::convert::AsRef<str>) {
                if Ok("ignore") == ::std::env::var("DFIR_EXPECT_WARNINGS").as_deref() {
                    eprintln!("{}", msg.as_ref());
                } else {
                    panic!("{}", msg.as_ref());
                }
            }

            let __file = ::std::file!();
            let __line = ::std::line!() as usize;
            let __hf = $crate::dfir_syntax_noemit! $hf;

            let actuals = __hf.diagnostics().expect("Expected `diagnostics()` to be set.");
            let actuals_len = actuals.len();
            let actuals = ::std::collections::BTreeSet::from_iter(actuals.iter().cloned().map(|mut actual| {
                actual.span.line = actual.span.line.saturating_sub(__line);
                ::std::borrow::Cow::<'static, str>::Owned(actual.to_string().replace(__file, "$FILE"))
            }));

            let expecteds = [
                $(
                    ::std::borrow::Cow::Borrowed( $msg ),
                )*
            ];
            let expecteds_len = expecteds.len();
            let expecteds = ::std::collections::BTreeSet::from(expecteds);

            let missing_errs = expecteds.difference(&actuals).map(|missing| {
                format!("Expected diagnostic `{}` was not emitted.", missing)
            });
            let extra_errs = actuals.difference(&expecteds).map(|extra| {
                format!("Unexpected extra diagnostic `{}` was emitted", extra)
            });
            let all_errs: ::std::vec::Vec<_> = missing_errs.chain(extra_errs).collect();
            if !all_errs.is_empty() {
                emit(all_errs.join("\n"));
            }

            if actuals_len != expecteds_len {
                emit(format!(
                    "Number of expected warnings ({}) does not match number of actual warnings ({}), were there duplicates?",
                    expecteds_len,
                    actuals_len
                ));
            }

            __hf
        }
    };
}

/// Test helper, emits and checks snapshots for the mermaid and dot graphs.
#[doc(hidden)]
#[macro_export]
macro_rules! assert_graphvis_snapshots {
    ($df:ident) => {
        $crate::assert_graphvis_snapshots!($df, &Default::default())
    };
    ($df:ident, $cfg:expr) => {
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let cfg = $cfg;
                insta::with_settings!({snapshot_suffix => "graphvis_mermaid"}, {
                    insta::assert_snapshot!($df.meta_graph().unwrap().to_mermaid(cfg));
                });
                insta::with_settings!({snapshot_suffix => "graphvis_dot"}, {
                    insta::assert_snapshot!($df.meta_graph().unwrap().to_dot(cfg));
                });
            }
        }
    }
}

#[doc(hidden)]
#[macro_export]
#[cfg(feature = "python")]
macro_rules! __python_feature_gate {
    (
        {
            $( $ypy:tt )*
        },
        {
            $( $npy:tt )*
        }
    ) => {
        $( $ypy )*
    };
}

#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "python"))]
macro_rules! __python_feature_gate {
    (
        {
            $( $ypy:tt )*
        },
        {
            $( $npy:tt )*
        }
    ) => {
        $( $npy )*
    };
}
