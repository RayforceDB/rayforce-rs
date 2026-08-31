//! Building a value requires a live `Runtime`.
//!
//! Its own test binary on purpose: this is about a process where no runtime was
//! ever created, which cannot be arranged in a file that also runs tests that
//! build one.
//!
//! Nothing here used to fail. `ray_alloc` lazily mmaps a thread-local heap when
//! none exists (core `heap.c:1386-1390`), so a constructor with no runtime
//! quietly allocated into an orphan heap instead of crashing — which is why the
//! hole survived so long.

#[test]
#[should_panic(expected = "requires a live Runtime")]
fn an_atom_cannot_be_built_without_a_runtime() {
    assert!(!rayforce::on_runtime_thread());
    let _ = rayforce::Value::i64(41);
}

#[test]
#[should_panic(expected = "requires a live Runtime")]
fn a_symbol_cannot_be_built_without_a_runtime() {
    // The sharp case: symbols are runtime-scoped, so with no symbol table to
    // intern into this returned an *empty* symbol — "hello" silently dropped on
    // the floor, no error anywhere.
    assert!(!rayforce::on_runtime_thread());
    let _ = rayforce::Value::sym("hello");
}
