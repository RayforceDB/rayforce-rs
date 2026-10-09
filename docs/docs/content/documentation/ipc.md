# :material-network: IPC — Inter-Process Communication

`rayforce` ships a TCP **client** for talking to a running RayforceDB server over
RayforceDB's native IPC protocol. This lets you run queries on a remote (or
local) RayforceDB instance and exchange `Value`s with it.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, TcpClient, Value};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

## :material-server: Running a server

The client connects to a RayforceDB server process. Start one by running the
`rayforce` binary in server mode with `-p PORT`:

```sh
rayforce -p 5000
```

The server now listens for IPC connections on port `5000`.

!!! info "Serving from Rust"
    `Poll::serve_ipc(port)` serves this wire from a runtime, with a budget on
    each request; see [Serving IPC](#serving-ipc) below. The **Q** listener is
    there too: `Poll::serve_q(port)` makes a runtime accept q peers, answer what
    they send synchronously and dispatch what they push, which is what the
    `rayforce -q` binary does. See [Serving the Q wire](#serving-the-q-wire)
    below.

## :material-lan-connect: Connecting

Create a client with `TcpClient::connect(host, port, user, password)`. Pass empty
strings for `user` / `password` when the server requires no authentication. The
call returns `Result<TcpClient>`, so a failed connection is an error you handle.

```rust
use rayforce::{Runtime, TcpClient};
Runtime::scope(|_rt| {
    let client = TcpClient::connect("127.0.0.1", 5000, "", "")?;
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

A failure names its cause rather than reporting "failed":

| Message ends with | What happened |
| --- | --- |
| `connection refused` | Nothing is listening, or the core refused before reaching the peer |
| `authentication failed` | The server rejected the credentials |
| `wire version mismatch` | The peer speaks a serialization version this build would misparse |
| `timed out` | No answer within the 5s budget — including from a server that is up but busy inside a long evaluation |
| an OS error, e.g. `No route to host (os error 113)` | Anything else the socket layer reported; a host that fails to resolve arrives here |

The connection is **closed on drop** — when the `TcpClient` goes out of scope the
socket is released. You can also close it explicitly with `client.close()`.

## :material-database-search: Executing queries

`execute(query)` sends a Rayfall source string to the server, runs it there, and
returns the result as a `Value`.

```rust
use rayforce::{Runtime, TcpClient};
Runtime::scope(|_rt| {
    let client = TcpClient::connect("127.0.0.1", 5000, "", "")?;

    // A scalar result.
    let sum = client.execute("(+ 1 2)")?;
    assert_eq!(sum.as_i64()?, 3);

    // A vector result, read back zero-copy.
    let v = client.execute("(til 5)")?;
    assert_eq!(v.as_slice::<i64>()?, &[0, 1, 2, 3, 4]);
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

Server-side errors come back as a `Result::Err`:

```rust
use rayforce::{Runtime, TcpClient};
Runtime::scope(|_rt| {
    let client = TcpClient::connect("127.0.0.1", 5000, "", "")?;

    assert!(client.execute("(undefined_symbol_xyz)").is_err());
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

## :material-send: Sending values

To send a `Value` (rather than a source string), use `send`. It transmits the
value to the server, waits for the response, and returns it as a `Value`:

```rust
use rayforce::{Runtime, TcpClient, Value};
Runtime::scope(|_rt| {
    let client = TcpClient::connect("127.0.0.1", 5000, "", "")?;

    let payload = Value::vec(&[1i64, 2, 3]);
    let reply = client.send(&payload)?;
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

### :material-timer-outline: Bounding a request

`execute` and `send` wait as long as the server takes. To give up after a while,
use `execute_timeout` or `send_timeout` with a `Duration`. The deadline covers
the whole round trip, sending the request included:

```rust
use rayforce::{ErrorCode, Runtime, TcpClient};
use std::time::Duration;
Runtime::scope(|_rt| {
    let client = TcpClient::connect("127.0.0.1", 5000, "", "")?;

    match client.execute_timeout("(+ 1 2)", Duration::from_secs(2)) {
        Ok(v) => println!("{}", v.as_i64()?),
        Err(e) if e.code == ErrorCode::Io => println!("gave up: {e}"),
        Err(e) => return Err(e),
    }
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

When the deadline passes, the server is asked to cancel the query, as with
Ctrl-C, and **the connection is closed**. A reply carries no request id, so a
late one would otherwise be read as the answer to your next request. Every later
call on that client fails straight away; connect again to carry on. The same
goes for a server that goes away during any request. An error the server
*answers* with, such as an unknown symbol, leaves the connection open.

The timeout is rounded up to whole milliseconds. `Duration::ZERO` is refused,
because the core would read a zero timeout as no deadline at all.

### :material-send-outline: Fire and forget

For fire-and-forget messages where you do not need a reply, `send_async` sends a
value and returns immediately:

```rust
use rayforce::{Runtime, TcpClient, Value};
Runtime::scope(|_rt| {
    let client = TcpClient::connect("127.0.0.1", 5000, "", "")?;

    client.send_async(&Value::sym("ping"))?;   // returns () on success
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

## :material-code-tags: A full example

```rust
use rayforce::{Runtime, TcpClient};

fn main() -> rayforce::Result<()> {
    Runtime::scope(|_rt| {
        // Connect to a server started with: rayforce -p 5000
        let client = TcpClient::connect("127.0.0.1", 5000, "", "")?;

        // Run a query remotely.
        let result = client.execute("(+ 1 2)")?;
        println!("server says: {}", result.as_i64()?);

        // The connection closes automatically when `client` is dropped.
        Ok(())
    })
}
```

## :material-rss: Subscribing to a push feed

`TcpClient` and `QConnection` are both request/response: you ask, the server
answers. A *subscription* inverts that — you ask once, and the peer pushes for
the rest of the day. The blocking clients cannot express it, because a frame the
peer pushes unsolicited would be read as the answer to your next call.

The fix is to give the socket to the event loop. `Poll` is that loop; attaching
a connection to it means the loop owns the reads and routes each frame by
message type — a response wakes the call parked on it, and anything else is
dispatched to the function the publisher named. You bind that name.

```rust,no_run
use rayforce::{env, q::QConnection, Poll, Runtime, Value};

// The push handler. A kdb+ tickerplant sends `(`upd;`trade;tbl)` — two
// arguments — while a dict-form publisher sends `(`upd;dict)` — one. Binding
// it *vary* is what lets one build serve both: a fixed arity does not error,
// it silently drops every frame from the other kind of peer.
struct OnUpd;

impl env::VaryFn for OnUpd {
    fn call(args: env::Args<'_>) -> Value {
        println!("batch with {} argument(s)", args.len());
        Value::null()        // an async push expects no reply
    }
}

fn main() -> rayforce::Result<()> {
    Runtime::scope(|_rt| {
        let poll = Poll::install()?;
        env::bind_vary::<OnUpd>("upd")?;

        let sub = QConnection::connect("127.0.0.1", 5010)?.attach(&poll)?;
        sub.execute(".u.sub[`trade;`]")?;

        // A disconnect is not an error and does not interrupt the loop: the
        // peer simply stops resolving. Slice the loop and check between slices,
        // or a process whose connection was reaped sits there looking healthy
        // while receiving nothing.
        while sub.is_alive() {
            poll.run_for(200)?;   // `on_upd` fires from inside here
        }
        println!("peer disconnected");
        Ok(())
    })
}
```

!!! note "A handler is a type, not a closure"

    The core takes a bare C function pointer with no user-data argument, so
    there is nowhere to put a closure's captures. Binding a *type* sidesteps
    that — each handler gets its own monomorphized trampoline, so the function
    address carries the identity a data pointer normally would. State a handler
    needs lives in statics.

    The trampoline owns what is easy to get wrong: it borrows the argument
    array without taking ownership, and catches a panic before it can unwind
    into C, replying null instead. So one malformed frame cannot wedge the
    stream, and implementors write ordinary safe Rust.

!!! note "Teardown order"

    Closing a selector releases engine objects held for it, so a `Subscription`
    must die before the event loop it runs on. `Subscription` borrows its
    `Poll`, so the compiler enforces that half for you.

    The other half the scope enforces. The loop belongs to the `Runtime` and is
    torn down with it, and both `Poll` and `Subscription` are `!Send` — so
    `Runtime::scope`'s bound refuses to let either leave the closure. The
    `is_current()` check on every `Poll` method is for the one route the type
    system cannot see, a `thread_local!` stash: a handle that gets out that way
    stops working rather than touching a destroyed loop.

## :material-arrow-right: See also

- [:material-swap-horizontal: Serialization](serialization.md) — the same wire
  format used by IPC, available directly via `Value::serialize` /
  `Value::deserialize`.
- `q::decode_response` — decode a Q message your own transport already read,
  when you want the socket in a separate thread rather than on the event loop.

## :material-server-network: Serving the Q wire

`Poll::serve_q(port)` registers a Q-protocol listener on the runtime's event
loop. Each peer that connects is served for as long as the loop runs: a string
sent synchronously is evaluated as Rayfall and answered, and a call pushed
asynchronously, `(upd; payload)`, is dispatched to whatever `upd` names in the
global environment, a function defined in Rayfall or one bound with
`env::bind_vary`. That is an RDB a q publisher can write into:

```rust
use rayforce::{Poll, Runtime};

Runtime::scope(|rt| {
    rt.eval("(set upd (fn [p] (set last p)))")?;
    let poll = Poll::install()?;
    let _listener = poll.serve_q(5010)?;
    loop {
        poll.run_for(200)?;   // peers are accepted and served in here
    }
})?;
# Ok::<(), rayforce::RayError>(())
```

The `QListener` is a handle, not an owner: the socket belongs to the poll and
closes with the runtime, and dropping the handle does not stop serving. Port 0
is refused, so pick the port yourself.

## :material-shield-check: Serving IPC

`Poll::serve_ipc(port)` serves the native IPC, the wire `rayforce -p` serves and
`TcpClient` speaks, from a runtime, with every request guarded. The server
evaluates one request at a time, so a request that runs for minutes holds every
other client for minutes. The guard cancels a request that runs past the budget,
or whose client has closed its connection. A client's own cancel works as
before.

```rust
use std::time::Duration;
use rayforce::{Poll, Runtime};

Runtime::scope(|_rt| {
    let poll = Poll::install()?;
    let ipc = poll.serve_ipc(5011).budget(Duration::from_secs(10)).start()?;
    loop {
        poll.run_for(100)?;   // requests are served in here
        for end in ipc.drain() {
            println!("{:?} in {:?}: {}", end.ended, end.duration, end.text);
        }
    }
})?;
# Ok::<(), rayforce::RayError>(())
```

`drain` hands back each request that ended since the last call: its whole text,
where the core's own query log keeps the first 256 characters; how long it was
evaluated; and whether it was `Answered`, `Failed` with the core's error code, or
`Cancelled` by the budget, a hangup or its client.

- Nothing on the Q wire goes through the guard, so a writer there is never
  cancelled. It still waits: for the request being evaluated, and for any other
  queued ahead of it, because the core takes what is ready on the two wires in
  an order of its own. Each wait is at most a budget.
- The guard works through the core's `.ipc.on.sync` and `.ipc.on.async` hooks,
  which are one per process, so a second `serve_ipc` is refused. They are
  globals a client can redefine: serve clients in restricted mode to hold the
  budget against them.
- As with the Q listener, the socket closes only with the runtime. Dropping the
  `IpcServer` stops the guard, not the serving.
- A zero budget, which would cancel every request, and port 0 are refused.
  `clock` sets the clock the budget is measured on, for a test that moves time.
