#![cfg(feature = "python")]

use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;
use pyo3::prelude::*;

#[multiplatform_test(test)]
pub fn test_python_basic() {
    let mut hf = dfir_syntax! {
        source_iter(0..10)
            -> map(|x| (x,))
            -> py_udf("
def fib(n):
    if n < 2:
        return n
    else:
        return fib(n - 2) + fib(n - 1)
", "fib")
            -> map(|x: PyResult<Py<PyAny>>| Python::with_gil(|py| {
                x.unwrap().extract::<usize>(py).unwrap()
            }))
            -> assert_eq([0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
    };
    assert_graphvis_snapshots!(hf);

    hf.run_available();
}

#[multiplatform_test(test)]
pub fn test_python_too_many_args() {
    let mut hf = dfir_syntax! {
        source_iter([(5,)])
            -> py_udf("
def add(a, b):
    return a + b
", "add")
            -> map(PyResult::<Py<PyAny>>::unwrap_err)
            -> map(|py_err| py_err.to_string())
            -> assert_eq(["TypeError: add() missing 1 required positional argument: 'b'"]);
    };
    assert_graphvis_snapshots!(hf);

    hf.run_available();
}

#[multiplatform_test(test)]
pub fn test_python_two_args() {
    let mut hf = dfir_syntax! {
        source_iter([(5,1)])
            -> py_udf("
def add(a, b):
    return a + b
", "add")
            -> map(|x: PyResult<Py<PyAny>>| Python::with_gil(|py| {
                x.unwrap().extract::<usize>(py).unwrap()
            }))
            -> assert_eq([6]);
    };
    assert_graphvis_snapshots!(hf);

    hf.run_available();
}
