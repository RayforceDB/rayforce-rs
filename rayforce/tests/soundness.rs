//! A `Value` cannot outlive the runtime it was built in.
//!
//! Tearing the runtime down unmaps the engine heap: `ray_runtime_destroy`
//! munmaps every pool without consulting any object's reference count. A handle
//! that survived that pointed at unmapped address space, and its `Drop` wrote a
//! refcount into it — a segfault reachable from safe code, usually surfacing at
//! process exit far from its cause.
//!
//! `Runtime::scope` removes the shape rather than tracking it: the closure gets
//! a `&Runtime` it cannot drop, and the `Send` bounds reject a `Value` leaving
//! by return or by capture. Those rejections are `compile_fail` doctests on
//! `Runtime::scope` itself. What is left to check here is that the scope really
//! does tear down, on every path out.

use rayforce::{Runtime, TcpClient, Value};

#[test]
fn values_built_in_a_scope_are_dropped_with_it() {
    Runtime::scope(|_rt| {
        let vals: Vec<Value> = (0..64).map(Value::i64).collect();
        let list = Value::list(&vals);
        let vec = Value::vec(&[1i64, 2, 3]);
        let cloned = vec.clone();
        assert_eq!(list.len(), 64);
        assert_eq!(cloned.as_slice::<i64>().unwrap(), &[1, 2, 3]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn the_error_path_still_tears_down() {
    let r = Runtime::scope(|rt| Err::<(), _>(rt.eval("(undefined_name_xyz)").unwrap_err()));
    assert!(r.is_err());
    // If the guard had leaked, this would fail with "already live".
    assert_eq!(Runtime::scope(|rt| rt.eval("1")?.as_i64()).unwrap(), 1);
}

#[test]
fn a_panic_in_the_closure_still_tears_down() {
    let caught = std::panic::catch_unwind(|| {
        Runtime::scope(|_rt| -> rayforce::Result<()> {
            panic!("deliberate");
        })
    });
    assert!(caught.is_err(), "the panic must propagate");
    // The guard's `Drop` ran during the unwind, so the next scope can start.
    assert_eq!(Runtime::scope(|rt| rt.eval("2")?.as_i64()).unwrap(), 2);
}

#[test]
fn value_is_one_pointer_wide() {
    // No generation tag, and no heap-handle bookkeeping either: the scope bounds
    // a value's life, so there is nothing for the value itself to carry.
    assert_eq!(
        std::mem::size_of::<Value>(),
        std::mem::size_of::<*mut ()>(),
        "Value grew a field — did a liveness tag creep back?"
    );
}

#[test]
fn a_refused_connection_leaves_the_scope_usable() {
    // Nothing is listening on port 1. A refused connection is routine, so it
    // must leave nothing behind: the same scope keeps working, and the next one
    // starts. The failing path returns before a `TcpClient` exists, so its
    // `Drop` — which calls `ray_ipc_close` — must not run.
    Runtime::scope(|rt| {
        assert!(TcpClient::connect("127.0.0.1", 1, "", "").is_err());
        assert_eq!(rt.eval("(+ 1 1)")?.as_i64()?, 2);
        Ok(())
    })
    .unwrap();
    Runtime::scope(|rt| rt.eval("1")?.as_i64()).unwrap();
}
