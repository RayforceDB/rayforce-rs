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

// The guard the three tests below exercise is a *thread* property, not a
// process one: the engine's VM and heap are both thread-local (`__VM` in the
// core's src/core/runtime.c, `ray_tl_heap` in src/mem/heap.c), while creating a
// runtime is what must stay unique process-wide.

#[test]
fn eval_off_the_runtime_thread_is_refused() {
    Runtime::scope(|rt| {
        // Control: the same call on the runtime's own thread works.
        assert_eq!(rt.eval("(+ 1 1)")?.as_i64()?, 2);
        // The value never crosses the join — `Value` is `!Send`, so returning
        // one would not compile. What is under test is the call, not the result.
        let off = std::thread::spawn(|| rayforce::eval("(+ 1 1)").map(|v| v.as_i64())).join();
        assert!(
            off.is_err(),
            "eval off the runtime's thread must be refused, not dispatched"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn construction_off_the_runtime_thread_is_refused() {
    Runtime::scope(|_rt| {
        // Control: on the runtime's thread the symbol interns and reads back.
        assert_eq!(Value::sym("hello").as_sym()?, "hello");
        // Off-thread this used to succeed, quietly allocating in a per-thread
        // heap that no `ray_runtime_destroy` will ever unmap.
        let off = std::thread::spawn(|| Value::sym("hello").as_sym()).join();
        assert!(
            off.is_err(),
            "constructing off the runtime's thread must be refused"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_second_thread_cannot_start_a_runtime() {
    // The invariant the thread-local guard must leave alone. `__RUNTIME` in the
    // core is an unguarded global, so a second `ray_runtime_create` would
    // overwrite the first; the compare-exchange in `Runtime::new` is the only
    // thing refusing it, and it stays process-wide for exactly this reason.
    Runtime::scope(|rt| {
        let nested = std::thread::spawn(|| Runtime::scope(|rt| rt.eval("1")?.as_i64()))
            .join()
            .expect("the second thread must be refused, not crash");
        assert!(nested.is_err(), "a second runtime must not be creatable");
        // The refusal left the first runtime intact.
        assert_eq!(rt.eval("(+ 2 3)")?.as_i64()?, 5);
        Ok(())
    })
    .unwrap();
}
