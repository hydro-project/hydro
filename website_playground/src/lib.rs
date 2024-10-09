mod utils;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::task::{Context, Poll};
use std::thread_local;

use hydroflow::datalog;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow_datalog_core::gen_hydroflow_graph;
use hydroflow_lang::diagnostic::{Diagnostic, Level};
use hydroflow_lang::graph::{build_hfcode, partition_graph, WriteConfig};
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
pub struct HydroflowResult {
    pub output: Option<HydroflowOutput>,
    pub diagnostics: Vec<JsDiagnostic>,
}
#[derive(Serialize, Deserialize)]
pub struct HydroflowOutput {
    pub compiled: String,
    pub mermaid: String,
}

#[wasm_bindgen]
pub fn compile_hydroflow(
    program: String,
    no_subgraphs: bool,
    no_varnames: bool,
    no_pull_push: bool,
    no_handoffs: bool,
    no_references: bool,
    op_short_text: bool,
) -> JsValue {
    let write_config = WriteConfig {
        no_subgraphs,
        no_varnames,
        no_pull_push,
        no_handoffs,
        no_references,
        op_short_text,
        op_text_no_imports: false,
    };

    let out = match syn::parse_str(&program) {
        Ok(input) => {
            let (graph_code_opt, diagnostics) =
                build_hfcode(input, &quote!(hydroflow), PathBuf::default());
            let output = graph_code_opt.map(|(graph, code)| {
                let mermaid = graph.to_mermaid(&write_config);
                let file = syn::parse_quote! {
                    fn main() {
                        let mut df = #code;
                        df.run_available();
                    }
                };
                let compiled = prettyplease::unparse(&file);
                HydroflowOutput { mermaid, compiled }
            });
            HydroflowResult {
                output,
                diagnostics: diagnostics.into_iter().map(Into::into).collect(),
            }
        }
        Err(errors) => HydroflowResult {
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

#[wasm_bindgen]
pub fn compile_datalog(
    program: String,
    no_subgraphs: bool,
    no_varnames: bool,
    no_pull_push: bool,
    no_handoffs: bool,
    no_references: bool,
    op_short_text: bool,
) -> JsValue {
    let write_config = WriteConfig {
        no_subgraphs,
        no_varnames,
        no_pull_push,
        no_handoffs,
        no_references,
        op_short_text,
        op_text_no_imports: false,
    };

    let wrapped = format!("r#\"{}\"#", program);
    let out = match syn::parse_str(&wrapped) {
        Ok(input) => match gen_hydroflow_graph(input) {
            Ok(flat_graph) => {
                let mut diagnostics = Vec::new();
                let output = match partition_graph(flat_graph) {
                    Ok(part_graph) => {
                        let out = part_graph.as_code(
                            &quote!(hydroflow),
                            true,
                            quote!(),
                            &mut diagnostics,
                        );
                        let file: syn::File = syn::parse_quote! {
                            fn main() {
                                #out
                            }
                        };

                        Some(HydroflowOutput {
                            compiled: prettyplease::unparse(&file),
                            mermaid: part_graph.to_mermaid(&write_config),
                        })
                    }
                    Err(diagnostic) => {
                        diagnostics.push(diagnostic);
                        None
                    }
                };
                HydroflowResult {
                    output,
                    diagnostics: diagnostics.into_iter().map(Into::into).collect(),
                }
            }
            Err(diagnostics) => HydroflowResult {
                output: None,
                diagnostics: diagnostics.into_iter().map(Into::into).collect(),
            },
        },
        Err(err) => HydroflowResult {
            output: None,
            diagnostics: vec![Diagnostic {
                span: Span::call_site(),
                level: Level::Error,
                message: format!("Error: Could not parse input: {}", err),
            }
            .into()],
        },
    };

    serde_wasm_bindgen::to_value(&out).unwrap()
}

struct HydroflowInstance<'a, In, Out> {
    hydroflow: Hydroflow<'a>,
    input: tokio::sync::mpsc::UnboundedSender<In>,
    output: tokio::sync::mpsc::UnboundedReceiver<Out>,
}

type DatalogBooleanDemoInstance = HydroflowInstance<'static, (i32,), (i32,)>;

thread_local! {
    static DATALOG_BOOLEAN_DEMO_INSTANCES: RefCell<HashMap<String, DatalogBooleanDemoInstance>> =
        RefCell::new(HashMap::new());
}

#[wasm_bindgen]
pub fn init_datalog_boolean_demo(instance_name: &str) {
    DATALOG_BOOLEAN_DEMO_INSTANCES.with(|map| {
        let (in_send, input) = hydroflow::util::unbounded_channel::<(i32,)>();
        let (out, out_recv) = hydroflow::util::unbounded_channel::<(i32,)>();
        let hydroflow = datalog!(
            r#"
              .input ints `source_stream(input)`
              .output result `for_each(|v| out.send(v).unwrap())`

              result(a) :- ints(a), ( a >= 0 ).
            "#
        );

        map.borrow_mut().insert(
            instance_name.into(),
            DatalogBooleanDemoInstance {
                hydroflow,
                input: in_send,
                output: out_recv.into_inner(),
            },
        );
    })
}

#[wasm_bindgen]
pub fn send_datalog_boolean_demo(instance_name: &str, input: i32) -> Option<i32> {
    DATALOG_BOOLEAN_DEMO_INSTANCES.with(|map| -> Option<i32> {
        let mut map = map.borrow_mut();
        let instance = map.get_mut(instance_name)?;
        instance.input.send((input,)).unwrap();
        instance.hydroflow.run_tick();
        match instance
            .output
            .poll_recv(&mut Context::from_waker(futures::task::noop_waker_ref()))
        {
            Poll::Pending => None,
            Poll::Ready(opt) => Some(opt?.0),
        }
    })
}
