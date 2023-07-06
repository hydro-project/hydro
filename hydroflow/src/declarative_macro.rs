//! Hydroflow declarative macros.

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

/// Test helper, emits and checks snapshots for the mermaid and dot graphs.
#[doc(hidden)]
#[macro_export]
macro_rules! assert_graphvis_snapshots {
    ($df:ident) => {
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                insta::with_settings!({snapshot_suffix => "graphvis_mermaid"}, {
                    insta::assert_snapshot!($df.meta_graph().unwrap().to_mermaid());
                });
                insta::with_settings!({snapshot_suffix => "graphvis_dot"}, {
                    insta::assert_snapshot!($df.meta_graph().unwrap().to_dot());
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
