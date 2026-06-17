//! Phase 1: runtime lifecycle, eval, and Value basics.
//!
//! These run serialized (`RUST_TEST_THREADS=1`); each test owns a short-lived
//! `Runtime`, which also exercises create → destroy → recreate within one
//! process — the property the whole test suite relies on.

use rayforce::{eval, Runtime};

#[test]
fn eval_arithmetic() {
    let _rt = Runtime::new().unwrap();
    let v = eval("(+ 1 1)").unwrap();
    assert_eq!(v.format(), "2");
}

#[test]
fn recreate_runtime_after_drop() {
    {
        let _rt = Runtime::new().unwrap();
        assert!(rayforce::is_live());
        assert_eq!(eval("(* 6 7)").unwrap().format(), "42");
    }
    assert!(!rayforce::is_live());
    // A second runtime in the same process must work (tests depend on this).
    {
        let _rt = Runtime::new().unwrap();
        assert_eq!(eval("(- 10 3)").unwrap().format(), "7");
    }
}

#[test]
fn only_one_live_runtime() {
    let _rt = Runtime::new().unwrap();
    assert!(Runtime::new().is_err(), "second runtime should be rejected");
}

#[test]
fn eval_error_is_surfaced() {
    let _rt = Runtime::new().unwrap();
    let err = eval("(undefined_name_xyz)").unwrap_err();
    // Should be a categorized error, not a panic.
    assert!(!err.code_str.is_empty() || !err.message.is_empty());
}

#[test]
fn value_type_inspection() {
    let _rt = Runtime::new().unwrap();
    let v = eval("(+ 2 3)").unwrap();
    assert!(v.is_atom());
    assert!(!v.is_vec());
    assert!(!v.is_null());
}
