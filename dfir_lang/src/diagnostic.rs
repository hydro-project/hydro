//! Compatibility for `proc_macro` diagnostics, which are missing from [`proc_macro2`].

extern crate proc_macro;

use std::hash::{Hash, Hasher};

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote_spanned;
use serde::{Deserialize, Serialize};

use crate::pretty_span::{PrettySpan, make_source_path_relative};

/// Diagnostic reporting level.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Level {
    /// An error.
    ///
    /// The most severe and important diagnostic. Errors will prevent compilation.
    Error,
    /// A warning.
    ///
    /// The second most severe diagnostic. Will not stop compilation.
    Warning,
    /// A note.
    ///
    /// The third most severe, or second least severe diagnostic.
    Note,
    /// A help message.
    ///
    /// The least severe and important diagnostic.
    Help,
}
impl Level {
    /// If this level is [`Level::Error`].
    pub fn is_error(&self) -> bool {
        self <= &Self::Error
    }
}

/// Diagnostic. A warning or error (or lower [`Level`]) with a message and span. Shown by IDEs
/// usually as a squiggly red or yellow underline.
///
/// Diagnostics must be emitted via [`Diagnostic::try_emit`], [`Diagnostic::to_tokens`], or
/// [`Diagnostic::try_emit_all`] for diagnostics to show up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic<S = Span> {
    /// Span (source code location).
    pub span: S,
    /// Severity level.
    pub level: Level,
    /// Human-readable message.
    pub message: String,
}
impl<S> Diagnostic<S> {
    /// If this diagnostic's level is [`Level::Error`].
    pub fn is_error(&self) -> bool {
        self.level.is_error()
    }
}
impl Diagnostic {
    /// Create a new diagnostic from the given span, level, and message.
    pub fn spanned(span: Span, level: Level, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            span,
            level,
            message,
        }
    }

    /// Emit if possible, otherwise return `Err` containing a [`TokenStream`] of a
    /// `compile_error!(...)` call.
    pub fn try_emit(&self) -> Result<(), TokenStream> {
        #[cfg(nightly)]
        if proc_macro::is_available() {
            let pm_diag = match self.level {
                Level::Error => self.span.unwrap().error(&*self.message),
                Level::Warning => self.span.unwrap().warning(&*self.message),
                Level::Note => self.span.unwrap().note(&*self.message),
                Level::Help => self.span.unwrap().help(&*self.message),
            };
            pm_diag.emit();
            return Ok(());
        }
        Err(self.to_tokens())
    }

    /// Emits all if possible, otherwise returns `Err` containing a [`TokenStream`] of
    /// `compile_error!(...)` calls.
    pub fn try_emit_all<'a>(
        diagnostics: impl IntoIterator<Item = &'a Self>,
    ) -> Result<(), TokenStream> {
        if let Some(tokens) = diagnostics
            .into_iter()
            .filter_map(|diag| diag.try_emit().err())
            .reduce(|mut tokens, next| {
                tokens.extend(next);
                tokens
            })
        {
            Err(tokens)
        } else {
            Ok(())
        }
    }

    /// Used to emulate `proc_macro::Diagnostic::emit` by turning this diagnostic into a properly spanned [`TokenStream`]
    /// that emits an error via `compile_error!(...)` with this diagnostic's message.
    pub fn to_tokens(&self) -> TokenStream {
        let msg_lit: Literal = Literal::string(&self.message);
        let unique_ident = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.level.hash(&mut hasher);
            self.message.hash(&mut hasher);
            let hash = hasher.finish();
            Ident::new(&format!("diagnostic_{}", hash), self.span)
        };

        if Level::Error == self.level {
            quote_spanned! {self.span=>
                {
                    ::core::compile_error!(#msg_lit);
                }
            }
        } else {
            // Emit as a `#[deprecated]` warning message.
            let level_ident = Ident::new(&format!("{:?}", self.level), self.span);
            quote_spanned! {self.span=>
                {
                    #[allow(dead_code, non_snake_case)]
                    fn #unique_ident() {
                        #[deprecated = #msg_lit]
                        struct #level_ident {}
                        #[warn(deprecated)]
                        #level_ident {};
                    }
                }
            }
        }
    }

    /// Converts this into a serializable and deserializable Diagnostic. Span information is
    /// converted into [`SerdeSpan`] which keeps the span info but cannot be plugged into or
    /// emitted through the Rust compiler's diagnostic system.
    pub fn to_serde(&self) -> Diagnostic<SerdeSpan> {
        let Self {
            span,
            level,
            message,
        } = self;
        Diagnostic {
            span: (*span).into(),
            level: *level,
            message: message.clone(),
        }
    }
}
impl From<syn::Error> for Diagnostic {
    fn from(value: syn::Error) -> Self {
        Self::spanned(value.span(), Level::Error, value.to_string())
    }
}
impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}: {}", self.level, self.message)?;
        write!(f, "  --> {}", PrettySpan(self.span))?;
        Ok(())
    }
}
impl std::fmt::Display for Diagnostic<SerdeSpan> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}: {}", self.level, self.message)?;
        write!(f, "  --> {}", self.span)?;
        Ok(())
    }
}

/// A serializable and deserializable version of [`Span`]. Cannot be plugged into the Rust
/// compiler's diagnostic system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerdeSpan {
    /// The source file path.
    pub file: Option<String>,
    /// Line number, one-indexed.
    pub line: usize,
    /// Column number, one-indexed.
    pub column: usize,
}
impl From<Span> for SerdeSpan {
    fn from(span: Span) -> Self {
        #[cfg_attr(
            not(nightly),
            expect(unused_labels, reason = "conditional compilation")
        )]
        let file = 'a: {
            #[cfg(nightly)]
            if proc_macro::is_available() {
                break 'a Some(span.unwrap().file());
            }

            None
        };

        Self {
            file,
            line: span.start().line,
            column: span.start().column,
        }
    }
}
impl std::fmt::Display for SerdeSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.file
                .as_ref()
                .map(make_source_path_relative)
                .map(|path| path.display().to_string())
                .as_deref()
                .unwrap_or("unknown"),
            self.line,
            self.column
        )
    }
}
