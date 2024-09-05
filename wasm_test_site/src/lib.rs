use wasm_bindgen::prelude::*;

fn log(string: impl AsRef<str>) {
    web_sys::console::log_1(&JsValue::from_str(string.as_ref()));
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn writeToDom(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    console_error_panic_hook::set_once();

    log("Hello, wasm_test_site!");
}

#[wasm_bindgen]
pub fn test_hydroflow() -> web_sys::js_sys::Promise {
    console_error_panic_hook::set_once();

    let mut df = hydroflow::hydroflow_syntax! {
        // https://hydro.run/docs/hydroflow/quickstart/example_1_simplest
        source_iter(0..10) -> for_each(|n| writeToDom(&format!("Hello {}", n)));
    };

    wasm_bindgen_futures::future_to_promise(async move {
        let work_done = df.run_available_async().await;
        Ok(if work_done {
            JsValue::TRUE
        } else {
            JsValue::FALSE
        })
    })
}
