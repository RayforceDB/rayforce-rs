//! Демо: как работают лямбды (`Fn`) в rayforce-rs.
//! Запуск: `cargo run -p rayforce --example lambda_demo`

use rayforce::{col, sum, Fn, Runtime, Table, Value};

fn main() {
    // Нужен живой рантайм (один на процесс).
    Runtime::scope(|_rt| {
        // 1. Создаём лямбду из исходника Rayfall.
        let square = Fn::new("(fn [x] (* x x))").unwrap();
        println!("лямбда:        {square}");

        // 2. Прямой вызов на скаляре — режим `call` (немедленное вычисление).
        let r = square.call(&[Value::i64(5)]).unwrap();
        println!("square(5)    = {}", r.as_i64().unwrap());

        // 3. Прямой вызов на векторе (лямбда применяется поэлементно).
        let r = square.call(&[Value::vec(&[2i64, 3, 4])]).unwrap();
        println!("square([2 3 4]) = {:?}", r.as_slice::<i64>().unwrap());

        // 4. Несколько аргументов.
        let add = Fn::new("(fn [x y] (+ x y))").unwrap();
        let r = add.call(&[Value::i64(10), Value::i64(32)]).unwrap();
        println!("add(10, 32)  = {}", r.as_i64().unwrap());

        // 5. Применение внутри запроса — режим `apply`.
        //    Лямбда биндится в глобальный env под уникальным именем, чтобы
        //    DAG-компилятор запроса мог её раскрыть по имени.
        let table = Table::new(
            &["id", "value"],
            &[Value::sym_vec(&["a", "b", "c"]), Value::vec(&[2i64, 3, 4])],
        )
        .unwrap();

        let out = table
            .select()
            .col("id")
            .agg("squared", square.apply([col("value")]).unwrap())
            .execute()
            .unwrap();
        println!("\nselect id, squared = square(value):\n{}", out.as_value());

        // 6. apply можно оборачивать в агрегаты — как обычное выражение.
        let out = table
            .select()
            .agg("sum_sq", sum(square.apply([col("value")]).unwrap()))
            .execute()
            .unwrap();
        println!(
            "\nsum of squares = {}",
            out.column("sum_sq")
                .unwrap()
                .get(0)
                .unwrap()
                .as_i64()
                .unwrap()
        );
        Ok(())
    })
    .unwrap();
}
