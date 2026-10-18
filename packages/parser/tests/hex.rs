use parser::Hex;

#[test]
fn encodes_and_decodes_round_trip() {
    let hex = Hex::from("hi\n");
    assert_eq!(hex.to_string(), "68690a");
    let bytes: Vec<u8> = hex.try_into().unwrap();
    assert_eq!(bytes, b"hi\n");
}

#[test]
fn rejects_malformed_hex() {
    assert!("abc".parse::<Hex>().is_err());
    assert!("zz".parse::<Hex>().is_err());
}
