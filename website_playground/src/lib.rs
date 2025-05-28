mod utils;

use dfir_lang::diagnostic::{Diagnostic, Level};
use dfir_lang::graph::{WriteConfig, build_hfcode};
use proc_macro2::{LineColumn, Span};
use quote::quote;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn init() {
    utils::set_panic_hook();
}

#[derive(Serialize, Deserialize)]
pub struct JsLineColumn {
    pub line: usize,
    pub column: usize,
}

impl From<LineColumn> for JsLineColumn {
    fn from(lc: LineColumn) -> Self {
        JsLineColumn {
            line: lc.line,
            column: lc.column,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsSpan {
    pub start: JsLineColumn,
    pub end: Option<JsLineColumn>,
}

impl From<Span> for JsSpan {
    fn from(span: Span) -> Self {
        #[cfg(procmacro2_semver_exempt)]
        let is_call_site = span.eq(&Span::call_site());

        #[cfg(not(procmacro2_semver_exempt))]
        let is_call_site = true;

        if is_call_site {
            JsSpan {
                start: JsLineColumn { line: 0, column: 0 },
                end: None,
            }
        } else {
            JsSpan {
                start: span.start().into(),
                end: Some(span.end().into()),
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsDiagnostic {
    pub span: JsSpan,
    pub message: String,
    pub is_error: bool,
}

impl From<Diagnostic> for JsDiagnostic {
    fn from(diag: Diagnostic) -> Self {
        JsDiagnostic {
            span: diag.span.into(),
            message: diag.message,
            is_error: diag.level == Level::Error,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DfirResult {
    pub output: Option<DfirOutput>,
    pub diagnostics: Vec<JsDiagnostic>,
}
#[derive(Serialize, Deserialize)]
pub struct DfirOutput {
    pub compiled: String,
    pub mermaid: String,
}

#[wasm_bindgen]
#[expect(
    clippy::too_many_arguments,
    reason = "Easier to expose `bool` to JS than a config struct. TODO(mingwei):"
)]
pub fn compile_dfir(
    program: String,
    no_subgraphs: bool,
    no_varnames: bool,
    no_pull_push: bool,
    no_handoffs: bool,
    no_references: bool,
    no_loops: bool,
    op_short_text: bool,
) -> JsValue {
    let write_config = WriteConfig {
        no_subgraphs,
        no_varnames,
        no_pull_push,
        no_handoffs,
        no_references,
        no_loops,
        op_short_text,
        op_text_no_imports: false,
    };

    let out = match syn::parse_str(&program) {
        Ok(input) => {
            let (graph_code_opt, diagnostics) = build_hfcode(input, &quote!(dfir_rs));
            let output = graph_code_opt.map(|(graph, code)| {
                let mermaid = graph.to_mermaid(&write_config);
                let file = syn::parse_quote! {
                    fn main() {
                        let mut df = #code;
                        df.run_available();
                    }
                };
                let compiled = prettyplease::unparse(&file);
                DfirOutput { mermaid, compiled }
            });
            DfirResult {
                output,
                diagnostics: diagnostics.into_iter().map(Into::into).collect(),
            }
        }
        Err(errors) => DfirResult {
            output: None,
            diagnostics: errors
                .into_iter()
                .map(|e| JsDiagnostic {
                    span: e.span().into(),
                    message: e.to_string(),
                    is_error: true,
                })
                .collect(),
        },
    };

    serde_wasm_bindgen::to_value(&out).unwrap()
}
