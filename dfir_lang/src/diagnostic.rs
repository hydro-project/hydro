//! Compatibility for `proc_macro` diagnostics, which are missing from [`proc_macro2`].

extern crate proc_macro;

use std::hash::{Hash, Hasher};

use itertools::Itertools;
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
    /// Iterator of all levels from most to least severe.
    pub fn iter() -> std::array::IntoIter<Self, 4> {
        [Self::Error, Self::Warning, Self::Note, Self::Help].into_iter()
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

/// A basic wrapper around [`Vec<Diagnostic>`] with pretty debug printing and utility methods.
pub struct Diagnostics<S = Span> {
    diagnostics: Vec<Diagnostic<S>>,
}

impl<S> Default for Diagnostics<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Diagnostics<S> {
    /// Creates a new empty `Diagnostics`.
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// Retains only diagnostics as severe as `level` or more severe.
    pub fn retain_level(&mut self, level: Level) {
        self.diagnostics.retain(|d| d.level <= level);
    }

    /// Returns if any errors exist in this collection.
    pub fn has_error(&self) -> bool {
        self.diagnostics.iter().any(|d| Level::Error == d.level)
    }

    /// Adds a diagnostic to this collection.
    pub fn push(&mut self, diagnostic: Diagnostic<S>) {
        self.diagnostics.push(diagnostic);
    }

    /// Returns an iterator over the diagnostics.
    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic<S>> {
        self.diagnostics.iter()
    }

    /// Returns the number of diagnostics in this collection.
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Returns if this collection is empty.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

impl Diagnostics {
    /// Emits all if possible, otherwise returns `Err` containing a [`TokenStream`] of code to spanned to emit each
    /// error and warning indirectly.
    pub fn try_emit_all(&self) -> Result<(), TokenStream> {
        if let Some(tokens) = self
            .diagnostics
            .iter()
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
}

impl<S> Extend<Diagnostic<S>> for Diagnostics<S> {
    fn extend<T: IntoIterator<Item = Diagnostic<S>>>(&mut self, iter: T) {
        self.diagnostics.extend(iter);
    }
}

impl<S> std::fmt::Debug for Diagnostics<S>
where
    Diagnostic<S>: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.diagnostics.is_empty() {
            write!(f, "Diagnostics (empty)")?;
        } else {
            write!(f, "Diagnostics (")?;
            let groups = self.diagnostics.iter().into_group_map_by(|d| d.level);
            for (level, count) in
                Level::iter().filter_map(|level| groups.get(&level).map(|vec| (level, vec.len())))
            {
                write!(f, "{level:?}: {count}, ")?;
            }
            writeln!(f, "):")?;
            for diagnostic in Level::iter()
                .filter_map(|level| groups.get(&level))
                .flatten()
            {
                writeln!(f, "{diagnostic}")?;
            }
        }
        Ok(())
    }
}

impl<S> FromIterator<Diagnostic<S>> for Diagnostics<S> {
    fn from_iter<T: IntoIterator<Item = Diagnostic<S>>>(iter: T) -> Self {
        Self {
            diagnostics: Vec::from_iter(iter),
        }
    }
}

impl<S> IntoIterator for Diagnostics<S> {
    type Item = Diagnostic<S>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.diagnostics.into_iter()
    }
}
