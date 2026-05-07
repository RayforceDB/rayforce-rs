# IPC Client

A blocking client for talking to a remote Rayforce server. Wraps
the public `ray_ipc_*` C symbols
(`ray_ipc_connect` / `ray_ipc_close` / `ray_ipc_send` /
`ray_ipc_send_async` / `ray_ipc_send_verbose`).

```rust
use rayforce::Connection;

let conn = Connection::connect("127.0.0.1", 5000)?;
let result = conn.execute("(select {from: trades take: 10})")?;
println!("{result}");
// connection is closed automatically when `conn` is dropped
```

## Connecting

```rust
use rayforce::Connection;

// Without auth.
let conn = Connection::connect("127.0.0.1", 5000)?;

// With auth.
let conn = Connection::connect_with_auth(
    "rayforce.example.com",
    5000,
    Some("alice"),    // username (optional)
    "s3cret",         // password
)?;
```

`connect` returns a [`RayforceError::IoError`] on:

| Condition | Message |
|---|---|
| Server not reachable / refused | `ipc connect failed (host:port)` |
| Auth required, no credentials | `auth required but no credentials provided` |
| Auth rejected | `auth rejected` |
| Wire-version mismatch | `wire version mismatch` |

These map back from the negative return codes of the C
`ray_ipc_connect` (`-1` / `-2` / `-3` / `-4`).

## Sending requests

The message can be any `RayObj`. The most common forms are a string
of Rayfall source for the server to evaluate, or a list expression
that the server will execute as-is.

```rust
use rayforce::{Connection, RayObj};

let conn = Connection::connect("127.0.0.1", 5000)?;

// Synchronous request/response with a Rayfall source string.
let result = conn.execute("(select {from: trades take: 10})")?;

// Same path, but with a pre-built RayObj message.
let msg = RayObj::from("1 + 2");
let r2 = conn.send(&msg)?;
assert_eq!(i64::try_from(r2)?, 3);
```

| Method | Returns | Use case |
|---|---|---|
| `send(&RayObj)` | `Result<RayObj>` | Synchronous request/response. |
| `send_verbose(&RayObj)` | `Result<RayObj>` | Same wire path, but the response is `[server_stdout, result]` — useful for REPL-style integrations. |
| `send_async(&RayObj)` | `Result<()>` | Fire-and-forget; no response is read. |
| `execute(&str)` | `Result<RayObj>` | Convenience: wraps the source string and calls `send`. |
| `close()` | `()` | Close the underlying socket; idempotent. Called automatically on drop. |
| `is_closed()` | `bool` | True after `close` (or after the connection has been dropped). |
| `handle()` | `i64` | Raw process-local handle slot, for embedders that want to call the C API directly. |

## Engine errors

If the server raises an engine-side error (parse error, unknown
column, type mismatch, …), the response comes back as a Rayforce
error object. The wrapper translates it into
`RayforceError::Ray { code, message, kind }` so you can match on the
short tag:

```rust
match conn.execute("nope") {
    Ok(v) => println!("got {v}"),
    Err(rayforce::RayforceError::Ray { code, .. }) if code == "parse" => {
        eprintln!("rayfall syntax error");
    }
    Err(e) => return Err(e.into()),
}
```

## Lifecycle

`Connection` implements `Drop`; closing is automatic. You can also
close explicitly if you want to release the slot before the binding
goes out of scope:

```rust
let mut conn = Connection::connect("127.0.0.1", 5000)?;
conn.execute("(set ready 1)")?;
conn.close();   // idempotent; subsequent send/execute will return IoError
assert!(conn.is_closed());
```

A failed `connect` does not consume a slot, so retrying is cheap.

## Thread safety

`Connection` is `Send` but not `Sync`. The IPC API is blocking and
holds per-handle state on the calling thread; share a connection by
moving it across threads, not by reference. For many concurrent
clients open one connection per worker thread.

## Limitations relative to 1.0

- **No timeout parameter.** The 1.0 module exposed
  `hopen_timeout(host, port, ms)`. The 2.x C entry point
  (`ray_ipc_connect`) uses a fixed internal connect timeout and
  doesn't accept one from callers; we can't surface a timeout from
  Rust until the upstream API gains one.
- **No server-side helpers.** The C engine has internal
  `ray_ipc_server_*` functions but they're not part of the public
  `rayforce.h` surface, so the Rust crate offers no equivalent.

## Next steps

- **[Eval & FFI](ffi.md)** — what `Rayforce::eval` does locally.
- **[Query Builder](query.md)** — build the queries you'll send
  remotely with `Connection::execute`.
