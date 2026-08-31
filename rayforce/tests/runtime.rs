//! Phase 1: runtime lifecycle, eval, and Value basics.
//!
//! These run serialized (`RUST_TEST_THREADS=1`); each test opens its own
//! `Runtime::scope`, which also exercises create → destroy → recreate within one
//! process — the property the whole test suite relies on.

use rayforce::{eval, Runtime};

#[test]
fn eval_arithmetic() {
    Runtime::scope(|_rt| {
        let v = eval("(+ 1 1)").unwrap();
        assert_eq!(v.format(), "2");
        Ok(())
    })
    .unwrap();
}

#[test]
fn recreate_runtime_after_scope() {
    assert!(!rayforce::on_runtime_thread());
    Runtime::scope(|_rt| {
        assert!(rayforce::on_runtime_thread());
        assert_eq!(eval("(* 6 7)").unwrap().format(), "42");
        Ok(())
    })
    .unwrap();
    assert!(!rayforce::on_runtime_thread());
    // A second runtime in the same process must work (tests depend on this).
    Runtime::scope(|_rt| {
        assert_eq!(eval("(- 10 3)").unwrap().format(), "7");
        Ok(())
    })
    .unwrap();
}

#[test]
fn only_one_live_runtime() {
    // The core permits one live runtime at a time, so a nested scope must
    // refuse rather than tear the outer one's heap out from under it.
    let err = Runtime::scope(|_rt| match Runtime::scope(|_inner| Ok(())) {
        Ok(()) => panic!("a nested scope must not start a second runtime"),
        Err(e) => Ok(e),
    })
    .unwrap();
    assert!(
        err.message.contains("cannot be nested"),
        "expected a nesting error, got: {}",
        err.message
    );
}

#[test]
fn eval_error_is_surfaced() {
    Runtime::scope(|_rt| {
        let err = eval("(undefined_name_xyz)").unwrap_err();
        // Should be a categorized error, not a panic.
        assert!(!err.code_str.is_empty() || !err.message.is_empty());
        Ok(())
    })
    .unwrap();
}

#[test]
fn value_type_inspection() {
    Runtime::scope(|_rt| {
        let v = eval("(+ 2 3)").unwrap();
        assert!(v.is_atom());
        assert!(!v.is_vec());
        assert!(!v.is_null());
        Ok(())
    })
    .unwrap();
}
