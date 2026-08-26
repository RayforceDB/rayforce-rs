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

!!! info "Embedded server is planned"
    An embedded `TcpServer` you can run from within Rust is planned but **not yet
    available** in the bindings. For now, run the standalone `rayforce` binary as
    shown above.

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

## :material-arrow-right: See also

- [:material-swap-horizontal: Serialization](serialization.md) — the same wire
  format used by IPC, available directly via `Value::serialize` /
  `Value::deserialize`.
