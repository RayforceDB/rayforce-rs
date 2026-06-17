//! Phase 2: scalar constructors, readers, conversions, null handling.

use rayforce::{eval, Guid, Runtime, Str, ToValue, Value};

#[test]
fn integer_roundtrips() {
    let _rt = Runtime::new().unwrap();
    assert_eq!(Value::i16(-12345).as_i16().unwrap(), -12345);
    assert_eq!(Value::i32(2_000_000_000).as_i32().unwrap(), 2_000_000_000);
    assert_eq!(Value::i64(i64::MAX).as_i64().unwrap(), i64::MAX);
    assert_eq!(Value::u8(255).as_u8().unwrap(), 255);
    assert!(Value::bool(true).as_bool().unwrap());
    assert!(!Value::bool(false).as_bool().unwrap());
}

#[test]
fn float_roundtrips() {
    let _rt = Runtime::new().unwrap();
    assert_eq!(Value::f64(3.5).as_f64().unwrap(), 3.5);
    assert_eq!(Value::f32(1.25).as_f32().unwrap(), 1.25);
    assert!(Value::f64(f64::NAN).is_atom_null());
}

#[test]
fn symbol_and_string() {
    let _rt = Runtime::new().unwrap();
    assert_eq!(Value::sym("hello").as_sym().unwrap(), "hello");
    assert_eq!(
        Value::string("a longer string value").as_string().unwrap(),
        "a longer string value"
    );
    // short (SSO) and empty
    assert_eq!(Value::string("hi").as_string().unwrap(), "hi");
    assert_eq!(Value::string("").as_string().unwrap(), "");
    assert!(Value::string("").is_atom_null());
}

#[test]
fn guid_roundtrip() {
    let _rt = Runtime::new().unwrap();
    let bytes = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];
    assert_eq!(Value::guid(&bytes).as_guid().unwrap(), bytes);
    assert!(Value::guid(&[0u8; 16]).is_atom_null());
}

#[test]
fn typed_nulls() {
    let _rt = Runtime::new().unwrap();
    assert!(Value::i64(i64::MIN).is_atom_null());
    assert!(Value::i32(i32::MIN).is_atom_null());
    assert!(Value::i16(i16::MIN).is_atom_null());
    assert!(!Value::i64(0).is_atom_null());
}

#[test]
fn temporal_raw_roundtrips() {
    let _rt = Runtime::new().unwrap();
    assert_eq!(Value::date_days(9000).as_date_days().unwrap(), 9000);
    assert_eq!(
        Value::time_millis(3_600_000).as_time_millis().unwrap(),
        3_600_000
    );
    assert_eq!(
        Value::timestamp_nanos(1_000_000_000)
            .as_timestamp_nanos()
            .unwrap(),
        1_000_000_000
    );
}

#[test]
fn type_mismatch_errors() {
    let _rt = Runtime::new().unwrap();
    assert!(Value::i64(5).as_f64().is_err());
    assert!(Value::sym("x").as_i64().is_err());
}

#[test]
fn to_from_value_traits() {
    let _rt = Runtime::new().unwrap();
    assert_eq!(42i64.to_value().extract::<i64>().unwrap(), 42);
    assert!(true.to_value().extract::<bool>().unwrap());
    // bare &str -> symbol; Str(..) -> string atom
    assert_eq!("sym".to_value().as_sym().unwrap(), "sym");
    assert_eq!(Str("str").to_value().as_string().unwrap(), "str");
    // Option<T> null handling
    let none: Option<i64> = None;
    assert!(none.to_value().is_null());
    assert_eq!(Value::i64(7).extract::<Option<i64>>().unwrap(), Some(7));
    assert_eq!(Value::i64(i64::MIN).extract::<Option<i64>>().unwrap(), None);
    // Guid wrapper
    let g = Guid([7u8; 16]);
    assert_eq!(g.to_value().extract::<Guid>().unwrap(), g);
}

#[test]
fn matches_engine_evaluation() {
    let _rt = Runtime::new().unwrap();
    // Our constructed atoms format identically to engine-produced ones.
    assert_eq!(Value::i64(2).format(), eval("(+ 1 1)").unwrap().format());
    assert_eq!(
        Value::f64(3.0).format(),
        eval("(* 1.5 2.0)").unwrap().format()
    );
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_roundtrips() {
    use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
    let _rt = Runtime::new().unwrap();

    let d = NaiveDate::from_ymd_opt(2021, 6, 15).unwrap();
    assert_eq!(d.to_value().extract::<NaiveDate>().unwrap(), d);

    let t = NaiveTime::from_hms_milli_opt(13, 30, 45, 250).unwrap();
    assert_eq!(t.to_value().extract::<NaiveTime>().unwrap(), t);

    let ts = Utc.with_ymd_and_hms(2021, 6, 15, 13, 30, 45).unwrap();
    assert_eq!(
        ts.to_value().extract::<chrono::DateTime<Utc>>().unwrap(),
        ts
    );
}
