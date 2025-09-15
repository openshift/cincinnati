#![allow(missing_docs, dead_code, clippy::from_over_into)]

struct MyNewTypeStruct(String);

impl_more::impl_as_ref!(MyNewTypeStruct => String);
impl_more::impl_as_mut!(MyNewTypeStruct => String);

impl_more::impl_deref!(MyNewTypeStruct => String);
impl_more::impl_deref_mut!(MyNewTypeStruct);
// or, to deref through String too:
// impl_more::forward_deref_and_mut!(MyNewTypeStruct, ref str);

impl_more::impl_from!(String => MyNewTypeStruct);
impl_more::impl_into!(MyNewTypeStruct => String);

enum MyEnum {
    Bar,
    Qux,
}

impl_more::impl_display_enum!(MyEnum, Bar => "bar", Qux => "qux");

enum Coords {
    Xy(i64, i64),
    Xyz(i64, i64, i64),
}

impl_more::impl_display_enum!(
    Coords,
    Xy(x, y) => "{x}, {y}",
    Xyz(x, y, z) => "{x}, {y}, {z}",
);

#[derive(Debug)]
enum Err {
    Io(std::io::Error),
    Generic(String),
}

impl_more::impl_display_enum!(Err, Io(err) => "{err}", Generic(msg) => "{msg}");
impl_more::impl_error_enum!(Err, Io(err) => err);

#[test]
fn forward_error_newtype() {
    use std::error::Error as _;

    #[derive(Debug)]
    struct MyError(eyre::Report);

    impl_more::forward_display!(MyError);
    impl_more::forward_error!(MyError);

    let err = MyError(eyre::eyre!("something went wrong"));
    assert_eq!(err.source().unwrap().to_string(), "something went wrong");
}

#[test]
fn forward_error_field() {
    use std::error::Error as _;

    #[derive(Debug)]
    struct MyError {
        cause: eyre::Report,
    }

    impl_more::forward_display!(MyError => cause);
    impl_more::forward_error!(MyError => cause);

    let err = MyError {
        cause: eyre::eyre!("something went wrong"),
    };
    assert_eq!(err.source().unwrap().to_string(), "something went wrong");
}
