//! Basic example demonstrating Rayforce Rust bindings.
//!
//! This example shows how to:
//! - Initialize the Rayforce runtime
//! - Create and manipulate various data types
//! - Build and query tables
//! - Use the expression builder for queries

use rayforce::{
    RayDict, RayF64, RayI64, RayList, RayString, RaySymbol, RayType, RayVector, Rayforce, Result,
};

fn main() -> Result<()> {
    // Initialize the Rayforce runtime
    println!("Initializing Rayforce runtime...");
    let rf = Rayforce::new()?;
    println!("Rayforce version: {}", rf.version());

    // === Scalar Types ===
    println!("\n=== Scalar Types ===");

    let i = RayI64::new(42);
    println!("RayI64: {}", i);

    let f = RayF64::new(3.14159);
    println!("RayF64: {}", f);

    let s = RaySymbol::new("hello");
    println!("RaySymbol: {}", s);

    let str_val = RayString::new("Hello, Rayforce!");
    println!("RayString: {}", str_val);

    // === Vectors ===
    println!("\n=== Vectors ===");

    let int_vec = RayVector::<i64>::from_iter([1i64, 2, 3, 4, 5]);
    println!("RayVector<i64>: {:?}", int_vec.as_slice());

    let float_vec = RayVector::<f64>::from_iter([1.1, 2.2, 3.3, 4.4, 5.5]);
    println!("RayVector<f64>: {:?}", float_vec.as_slice());

    let sym_vec = RayVector::<RaySymbol>::from_iter(["apple", "banana", "cherry"]);
    println!("RayVector<RaySymbol>: {} items", sym_vec.len());

    // === Lists ===
    println!("\n=== Lists ===");

    let mut list = RayList::new();
    list.push(1i64);
    list.push("hello");
    list.push(3.14f64);
    println!("Mixed RayList: {} items", list.len());

    // === Dictionaries ===
    println!("\n=== Dictionaries ===");

    let dict = RayDict::from_pairs([
        ("name", RayString::new("Alice").ptr().clone()),
        ("age", RayI64::new(30).ptr().clone()),
    ])?;
    println!("RayDict: {:?}", dict);

    // === Eval ===
    println!("\n=== Eval ===");

    // Basic evaluation examples
    let eval_result = rf.eval("42")?;
    println!("Eval '42': {}", eval_result);

    let string_result = rf.eval("\"hello world\"")?;
    println!("Eval '\"hello world\"': {}", string_result);

    println!("\nDone!");
    Ok(())
}
