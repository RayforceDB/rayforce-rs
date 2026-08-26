//! Phase 7: serialization round-trips.

use rayforce::{Runtime, Table, Value};

fn roundtrip(v: &Value) -> Value {
    let bytes = v.serialize().unwrap();
    assert!(!bytes.is_empty());
    Value::deserialize(&bytes).unwrap()
}

#[test]
fn scalar_roundtrips() {
    Runtime::scope(|_rt| {
        assert_eq!(
            roundtrip(&Value::i64(123456789)).as_i64().unwrap(),
            123456789
        );
        assert_eq!(roundtrip(&Value::f64(123.456)).as_f64().unwrap(), 123.456);
        assert_eq!(roundtrip(&Value::sym("hello")).as_sym().unwrap(), "hello");
        assert_eq!(
            roundtrip(&Value::string("a string value"))
                .as_string()
                .unwrap(),
            "a string value"
        );
        assert!(roundtrip(&Value::bool(true)).as_bool().unwrap());
        Ok(())
    })
    .unwrap();
}

#[test]
fn vector_roundtrips() {
    Runtime::scope(|_rt| {
        let v = Value::vec(&[1i64, 2, 3, 4, 5]);
        assert_eq!(roundtrip(&v).as_slice::<i64>().unwrap(), &[1, 2, 3, 4, 5]);

        let f = Value::vec(&[1.5f64, 2.5, 3.5]);
        assert_eq!(roundtrip(&f).as_slice::<f64>().unwrap(), &[1.5, 2.5, 3.5]);

        let syms = Value::sym_vec(&["a", "bb", "ccc"]);
        let back = roundtrip(&syms);
        assert_eq!(back.len(), 3);
        assert_eq!(back.get(1).unwrap().as_sym().unwrap(), "bb");
        Ok(())
    })
    .unwrap();
}

#[test]
fn list_roundtrip() {
    Runtime::scope(|_rt| {
        let l = Value::list(&[Value::i64(1), Value::sym("two"), Value::f64(3.0)]);
        let back = roundtrip(&l);
        assert_eq!(back.len(), 3);
        assert_eq!(back.get(0).unwrap().as_i64().unwrap(), 1);
        assert_eq!(back.get(1).unwrap().as_sym().unwrap(), "two");
        Ok(())
    })
    .unwrap();
}

#[test]
fn dict_roundtrip() {
    Runtime::scope(|_rt| {
        let d = Value::dict(Value::sym_vec(&["x", "y"]), Value::vec(&[10i64, 20]));
        let back = roundtrip(&d);
        assert!(back.is_dict());
        assert_eq!(back.dict_len().unwrap(), 2);
        assert_eq!(
            back.dict_get(&Value::sym("y"))
                .unwrap()
                .unwrap()
                .as_i64()
                .unwrap(),
            20
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn table_roundtrip() {
    Runtime::scope(|_rt| {
        let t = Table::new(
            &["sym", "px"],
            &[
                Value::sym_vec(&["AAPL", "MSFT"]),
                Value::vec(&[100.0f64, 200.0]),
            ],
        )
        .unwrap();
        let back = roundtrip(t.as_value());
        assert!(back.is_table());
        let bt = back.as_table().unwrap();
        assert_eq!(bt.shape(), (2, 2));
        assert_eq!(
            bt.column("px").unwrap().as_slice::<f64>().unwrap(),
            &[100.0, 200.0]
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn roundtrip_preserves_formatting() {
    Runtime::scope(|_rt| {
        let v = Value::vec(&[7i64, 8, 9]);
        assert_eq!(v.format(), roundtrip(&v).format());
        Ok(())
    })
    .unwrap();
}

#[test]
fn deserialize_garbage_errors() {
    Runtime::scope(|_rt| {
        // Random bytes are not a valid wire payload.
        let bad = [0u8, 1, 2, 3, 4, 5, 6, 7];
        assert!(Value::deserialize(&bad).is_err());
        Ok(())
    })
    .unwrap();
}
