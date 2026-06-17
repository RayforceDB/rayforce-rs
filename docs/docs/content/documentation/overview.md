# :material-file-document: Documentation

This is the reference for working with `rayforce` in depth. Everything builds on
a single uniform handle — [`Value`](data-types/values.md) — and runs inside one
live [`Runtime`](../get-started/overview.md). From there you assemble typed
columns into [tables](table/overview.md), shape them with a fluent
[query DSL](query-guide/overview.md), and move them across the wire with
[serialization](serialization.md) and the [IPC client](ipc.md).

!!! note "Assume a live runtime"
    Every runtime-dependent example in these pages assumes you have started a
    runtime first:

    ```rust
    use rayforce::{Runtime, Value};
    let _rt = Runtime::new()?;
    ```

## :material-map-outline: Map of the documentation

### :octicons-database-16: [Data Types](data-types/overview.md)

The value model: atoms, vectors, lists, dicts, and tables, plus `ToValue` /
`FromValue` conversions and optional `chrono` temporal support. Start with
[Values & Conversions](data-types/values.md).

### :material-table: [Table](table/overview.md)

Build columnar tables with [`Table::new`](table/create.md), read columns and rows
back, transform them, and round-trip them through
[CSV and splayed/parted on-disk formats](table/save-and-fetch.md).

### :material-database-eye-outline: [Query Guide](query-guide/overview.md)

The fluent query DSL: [expressions](query-guide/expressions.md) built with
`col(..)` and operator overloads, [`select`](query-guide/select.md) with
[filtering](query-guide/where.md), [grouping](query-guide/group-by.md), and
[ordering](query-guide/order-by.md), plus [`update`](query-guide/update.md),
[inserts/upserts](query-guide/insert.md), and [joins](query-guide/joins.md).

### :material-network: [IPC](ipc.md)

Connect to a running RayforceDB server with `TcpClient`, execute queries, and
exchange `Value`s over RayforceDB's native protocol.

### :material-swap-horizontal: [Serialization](serialization.md)

Turn any `Value` into bytes with `Value::serialize()` and back with
`Value::deserialize()` — the same wire format the IPC layer uses.

## :material-arrow-right: Where to start

If you are new, read [Data Types](data-types/overview.md) and the
[Query Guide](query-guide/overview.md) in order. If you already have a
RayforceDB server running, jump straight to [IPC](ipc.md).
