#[macro_use]
extern crate assert_json_diff;
#[macro_use]
extern crate serde_json;

#[test]
fn can_pass() {
    assert_json_include!(
        actual: json!({ "a": { "b": true }, "c": [true, null, 1] }),
        expected: json!({ "a": { "b": true }, "c": [true, null, 1] })
    );

    assert_json_include!(
        actual: json!({ "a": { "b": true } }),
        expected: json!({ "a": {} })
    );

    assert_json_include!(
        actual: json!({ "a": { "b": true } }),
        expected: json!({ "a": {} }),
    );

    assert_json_include!(
        expected: json!({ "a": {} }),
        actual: json!({ "a": { "b": true } }),
    );
}

#[test]
#[should_panic]
fn can_fail() {
    assert_json_include!(
        actual: json!({ "a": { "b": true }, "c": [true, null, 1] }),
        expected: json!({ "a": { "b": false }, "c": [false, null, {}] })
    );
}

#[test]
fn can_pass_with_exact_match() {
    assert_json_eq!(json!({ "a": { "b": true } }), json!({ "a": { "b": true } }));
    assert_json_eq!(json!({ "a": { "b": true } }), json!({ "a": { "b": true } }),);
}

#[test]
#[should_panic]
fn can_fail_with_exact_match() {
    assert_json_eq!(json!({ "a": { "b": true } }), json!({ "a": {} }));
}
