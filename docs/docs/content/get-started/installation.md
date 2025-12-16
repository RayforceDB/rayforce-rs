# Installation

## Prerequisites

Before installing rayforce-rs, ensure you have:

- **Rust** 1.70 or later (`rustup update stable`)
- **C Compiler** (gcc or clang)
- **Git** (for cloning dependencies)
- **Make** (for building the C library)

## Add to Cargo.toml

Add rayforce-rs to your project:

```toml
[dependencies]
rayforce = "0.1"
```

Or use cargo add:

```bash
cargo add rayforce
```

## Build from Source

If you want to build from the latest source:

```bash
# Clone the repository
git clone https://github.com/RayforceDB/rayforce-rs.git
cd rayforce-rs

# Build the library
cargo build --release

# Run tests
cargo test

# Run the example
cargo run --example basic
```

## How the Build Works

The build process automatically:

1. **Clones** the RayforceDB C library from GitHub
2. **Compiles** the C library as a static library
3. **Generates** Rust bindings using `bindgen`
4. **Links** everything together

!!! note "First Build"
    The first build will take longer as it downloads and compiles the C library. Subsequent builds are much faster.

## Verifying Installation

Create a simple test program:

```rust
// src/main.rs
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    println!("RayforceDB version: {}", ray.version());
    
    let result = ray.eval("(+ 1 2)")?;
    println!("1 + 2 = {}", result);
    
    Ok(())
}
```

Run it:

```bash
cargo run
```

Expected output:

```
RayforceDB version: 0.x.x
1 + 2 = 3
```

## Platform Notes

### Linux

Works out of the box on most distributions. Ensure you have:

```bash
# Ubuntu/Debian
sudo apt-get install build-essential git

# Fedora/RHEL
sudo dnf install gcc make git
```

### macOS

Requires Xcode Command Line Tools:

```bash
xcode-select --install
```

### Windows

Currently requires WSL2 (Windows Subsystem for Linux). Native Windows support is planned.

## Troubleshooting

### bindgen fails

Ensure you have LLVM/Clang installed:

```bash
# Ubuntu/Debian
sudo apt-get install llvm-dev libclang-dev clang

# macOS (via Homebrew)
brew install llvm
```

### Linker errors

Make sure the C library built successfully. Check the `tmp/rayforce-c` directory in your project root.

### Version mismatch

Clear the build cache and rebuild:

```bash
cargo clean
cargo build
```

## What's Next?

Now that you have rayforce-rs installed, check out the [Quick Start](quickstart.md) guide to build your first application!

