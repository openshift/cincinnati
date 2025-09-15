//! Concise, declarative trait implementation macros.
//!
//! # `#[no_std]`
//!
//! Where possible, these macros emit `#[no_std]`-compatible code.

#![cfg_attr(not(docsrs), no_std)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

#[cfg(test)]
extern crate alloc;
#[cfg(test)]
extern crate std;

#[macro_use]
mod as_ref;
#[macro_use]
mod deref;
#[macro_use]
mod display;
#[macro_use]
mod error;
#[macro_use]
mod from;

#[cfg(test)]
mod tests {
    #![allow(dead_code, clippy::from_over_into)]

    use alloc::string::{String, ToString as _};
    use core::ops::{Deref, DerefMut};

    #[derive(Debug)]
    struct Foo(String);

    crate::impl_as_ref!(Foo => String);
    crate::impl_as_mut!(Foo => String);

    crate::impl_deref!(Foo => String);
    crate::impl_deref_mut!(Foo);

    static_assertions::assert_impl_all!(
        Foo:
        // impls
        AsRef<String>,
        AsMut<String>,
        Deref<Target = String>,
        DerefMut,
    );

    #[derive(Debug)]
    struct Bar {
        foo: Foo,
    }

    crate::impl_as_ref!(Bar => foo: Foo);
    crate::impl_as_mut!(Bar => foo: Foo);

    crate::impl_deref!(Bar => foo: String);
    crate::impl_deref_mut!(Bar => foo);

    static_assertions::assert_impl_all!(
        Bar:
        // impls
        AsRef<Foo>,
        AsMut<Foo>,
        Deref<Target = String>,
        DerefMut,
    );

    #[allow(dead_code)]
    #[test]
    fn impl_display() {
        enum Foo {
            Bar,
            Qux,
        }
        crate::impl_display_enum!(Foo, Bar => "bar", Qux => "qux");
        assert_eq!(Foo::Bar.to_string(), "bar");
        assert_eq!(Foo::Qux.to_string(), "qux");

        enum FooComma {
            Bar,
            Qux,
        }
        crate::impl_display_enum!(FooComma, Bar => "bar", Qux => "qux",);

        enum FooContents {
            Bar(u64, u64),
        }
        crate::impl_display_enum!(FooContents, Bar (x, y) => "x: {x}; y: {y}");
        assert_eq!(FooContents::Bar(4, 2).to_string(), "x: 4; y: 2");

        enum FooContents2 {
            Qux { msg: &'static str },
        }
        crate::impl_display_enum!(FooContents2, Qux { msg } => "msg: {msg}");
        assert_eq!(FooContents2::Qux { msg: "foo" }.to_string(), "msg: foo");

        // not supported yet
        // enum FooContents3 {
        //     Bar(u64, u64),
        //     Qux { msg: &'static str },
        // }
        // impl_display_enum!(FooContents3, Bar (x, y) => "x: {x}; y: {y}", Qux { msg } => "{msg}");
        // assert_eq!(FooContents3::Bar(4, 2).to_string(), "x: 4; y: 2");
        // assert_eq!(FooContents3::Qux { msg: "foo" }.to_string(), "x: 4; y: 2");
    }
}
