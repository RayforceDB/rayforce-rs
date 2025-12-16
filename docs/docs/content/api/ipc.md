# IPC (Inter-Process Communication)

The IPC module enables communication with remote RayforceDB instances.

## Overview

IPC allows you to:

- Connect to remote RayforceDB servers
- Execute queries remotely
- Transfer data between processes
- Build distributed systems

## Connection

### Opening a Connection

```rust
use rayforce::ipc::{Connection, hopen};

// Connect to remote server
let conn = hopen("localhost", 5000)?;

// With timeout (milliseconds)
let conn = hopen_timeout("localhost", 5000, 5000)?;
```

### Connection Management

```rust
// Check if connected
if conn.is_connected() {
    println!("Connected!");
}

// Close connection
conn.close()?;
```

## Executing Queries

### Send and Receive

```rust
use rayforce::ipc::Connection;

let conn = hopen("localhost", 5000)?;

// Execute query
let result = conn.execute("(+ 1 2 3)")?;
println!("Result: {}", result);

// Execute with parameters
let result = conn.execute("(select { from: trades })")?;
```

### Async Execution

```rust
// Send without waiting for response
conn.send_async("(long_running_task)")?;
```

## Data Transfer

### Sending Data

```rust
use rayforce::{RayObj, RayVector};
use rayforce::ipc::Connection;

let conn = hopen("localhost", 5000)?;

// Send vector
let prices: RayVector<i64> = RayVector::from_iter([100, 200, 300]);
conn.write(&prices)?;

// Send with assignment
conn.execute("(set remote_prices prices)")?;
```

### Receiving Data

```rust
// Execute and receive result
let table = conn.execute("trades")?;
println!("Received: {}", table);

// Receive specific columns
let prices = conn.execute("(. trades 'price)")?;
```

## Connection Pool

For high-throughput applications:

```rust
use rayforce::ipc::ConnectionPool;

// Create connection pool
let pool = ConnectionPool::new("localhost", 5000, 10)?;

// Get connection from pool
let conn = pool.get()?;

// Use connection
let result = conn.execute("(+ 1 2)")?;

// Connection returns to pool when dropped
```

## Error Handling

### Connection Errors

```rust
use rayforce::ipc::{hopen, IPCError};

match hopen("localhost", 5000) {
    Ok(conn) => println!("Connected!"),
    Err(IPCError::ConnectionFailed(msg)) => {
        eprintln!("Connection failed: {}", msg);
    }
    Err(IPCError::Timeout) => {
        eprintln!("Connection timed out");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Query Errors

```rust
match conn.execute("(invalid_query") {
    Ok(result) => println!("{}", result),
    Err(e) => eprintln!("Query failed: {}", e),
}
```

## Complete Example

```rust
use rayforce::{Rayforce, RayVector};
use rayforce::ipc::hopen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to remote RayforceDB
    let conn = hopen("localhost", 5000)?;
    
    println!("Connected to RayforceDB server");
    
    // Check server version
    let version = conn.execute("(version)")?;
    println!("Server version: {}", version);
    
    // Create table on server
    conn.execute(r#"
        (set trades (table [sym price qty time]
            (list
                ['AAPL 'MSFT 'AAPL]
                [150.0 300.0 151.0]
                [100 50 200]
                [09:30:00 09:31:00 09:32:00])))
    "#)?;
    
    // Query data
    let result = conn.execute(r#"
        (select {
            sym: sym
            total_value: (sum (* price qty))
            from: trades
            by: sym})
    "#)?;
    
    println!("Trade summary:\n{}", result);
    
    // Close connection
    conn.close()?;
    
    Ok(())
}
```

## Server Setup

To run a RayforceDB server:

```bash
# Start server on port 5000
rayforce -p 5000
```

Or programmatically:

```rust
use rayforce::Rayforce;

let ray = Rayforce::new()?;

// Start listening (blocking)
ray.eval("(listen 5000)")?;
```

## Protocol

RayforceDB IPC uses a binary protocol:

1. **Message header**: Type + length
2. **Payload**: Serialized RayforceDB objects
3. **Response**: Result or error

## Security Considerations

!!! warning "Network Security"
    RayforceDB IPC does not include built-in encryption. Use SSH tunnels or VPNs for secure connections over untrusted networks.

### SSH Tunnel

```bash
# Create SSH tunnel
ssh -L 5000:localhost:5000 user@remote-server

# Connect via tunnel
let conn = hopen("localhost", 5000)?;
```

### Firewall Rules

```bash
# Allow only local connections
iptables -A INPUT -p tcp --dport 5000 -s 127.0.0.1 -j ACCEPT
iptables -A INPUT -p tcp --dport 5000 -j DROP
```

## Performance Tips

### Keep Connections Open

Reuse connections instead of reconnecting:

```rust
// Good: reuse connection
let conn = hopen("localhost", 5000)?;
for query in queries {
    conn.execute(&query)?;
}

// Bad: reconnect each time
for query in queries {
    let conn = hopen("localhost", 5000)?;  // Overhead
    conn.execute(&query)?;
}
```

### Batch Queries

Combine multiple queries when possible:

```rust
// Good: single round-trip
let result = conn.execute(r#"
    (list
        (select { from: table1 })
        (select { from: table2 })
        (select { from: table3 }))
"#)?;

// Bad: multiple round-trips
let r1 = conn.execute("(select { from: table1 })")?;
let r2 = conn.execute("(select { from: table2 })")?;
let r3 = conn.execute("(select { from: table3 })")?;
```

### Compress Large Data

For large transfers:

```rust
// Enable compression (if supported)
conn.set_compression(true)?;
```

## Next Steps

- **[FFI](ffi.md)** - Low-level bindings
- **[API Overview](overview.md)** - Full API reference
- **[Examples](../examples/index.md)** - More code examples

