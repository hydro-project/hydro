//! Test suite for Node.
//!
//! TODO(mingwei): update this test!

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use hydroflow_demo::{init_datalog_boolean_demo, send_datalog_boolean_demo};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_datalog_boolean_demo() {
    init_datalog_boolean_demo("test");
    assert_eq!(Some(0), send_datalog_boolean_demo("test", 0));
    assert_eq!(Some(1), send_datalog_boolean_demo("test", 1));
    assert_eq!(None, send_datalog_boolean_demo("test", -1));
}
