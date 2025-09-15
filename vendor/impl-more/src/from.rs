/// Implement [`From`] for a struct.
///
/// # Examples
/// With a newtype struct:
/// ```
/// use impl_more::impl_from;
///
/// struct Foo(String);
/// impl_from!(String => Foo);
///
/// let foo = Foo::from("bar".to_owned());
/// ```
///
/// With a named field struct with type parameters:
/// ```
/// use std::rc::Rc;
/// use impl_more::impl_from;
///
/// struct Foo<T> { inner: Rc<T> }
/// impl_from!(<T> in Rc<T> => Foo<T> : inner);
///
/// let foo = Foo::from(Rc::new("bar".to_owned()));
/// ```
#[macro_export]
macro_rules! impl_from {
    (<$($generic:ident),+> in $from:ty => $this:ty $(,)?) => {
        impl <$($generic),+> ::core::convert::From<$from> for $this {
            fn from(from: $from) -> Self {
                Self(from)
            }
        }
    };

    (<$($generic:ident),+> in $from:ty => $this:ty : $field:ident $(,)?) => {
        impl <$($generic),+> ::core::convert::From<$from> for $this {
            fn from(from: $from) -> Self {
                Self { $field: from }
            }
        }
    };

    ($from:ty => $this:ty $(,)?) => {
        impl ::core::convert::From<$from> for $this {
            fn from(from: $from) -> Self {
                Self(from)
            }
        }
    };


    ($from:ty => $this:ty : $field:ident $(,)?) => {
        impl ::core::convert::From<$from> for $this {
            fn from(from: $from) -> Self {
                Self { $field : from }
            }
        }
    };
}

/// Implement [`From`] for a primitive.
///
/// # Examples
/// With a newtype struct:
/// ```
/// use impl_more::impl_from_for_primitive;
///
/// struct Checked(bool);
/// impl_from_for_primitive!(bool; from Checked);
///
/// let foo = bool::from(Checked(true));
/// ```
#[macro_export]
macro_rules! impl_from_for_primitive {
    ($this:ty; from $from:ty $(,)?) => {
        impl ::core::convert::From<$from> for $this {
            fn from(from: $from) -> $this {
                <$this as ::core::convert::From<_>>::from(from.0)
            }
        }
    };
}

/// Implement [`Into`] for a struct.
///
/// # Examples
/// With a newtype struct:
/// ```
/// use impl_more::impl_into;
///
/// struct Foo(String);
///
/// impl_into!(Foo => String);
///
/// let foo = Foo("bar".to_owned());
/// let foo_str: String = foo.into();
/// ```
///
/// With a named field struct with type parameters:
/// ```
/// use std::rc::Rc;
/// use impl_more::impl_into;
///
/// struct Foo<T> { inner: Rc<T> }
/// impl_into!(<T> in Foo<T> => Rc<T> : inner);
///
/// let foo = Foo { inner: Rc::new("bar".to_owned()) };
/// let _: Rc<String> = foo.into();
/// ```
#[macro_export]
macro_rules! impl_into {
    (<$($generic:ident),+> in $this:ty => $inner:ty : $field:ident) => {
        impl <$($generic),+> ::core::convert::Into<$inner> for $this {
            fn into(self) -> $inner {
                self.$field
            }
        }
    };

    (<$($generic:ident),+> in $this:ty => $inner:ty) => {
        impl <$($generic),+> ::core::convert::Into<$inner> for $this {
            fn into(self) -> $inner {
                self.0
            }
        }
    };

    ($this:ty => $inner:ty) => {
        impl ::core::convert::Into<$inner> for $this {
            fn into(self) -> $inner {
                self.0
            }
        }
    };

    ($this:ty => $inner:ty : $field:ident) => {
        impl ::core::convert::Into<$inner> for $this {
            fn into(self) -> $inner {
                self.$field
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #![allow(clippy::from_over_into)]

    use alloc::rc::Rc;

    #[test]
    fn newtype() {
        struct Foo(usize);
        impl_from!(usize => Foo);
        impl_into!(Foo => usize);

        static_assertions::assert_impl_all!(Foo: From<usize>, Into<usize>);

        let foo = Foo::from(42);
        assert_eq!(foo.0, 42);
    }

    #[test]
    fn newtype_primitive() {
        struct Foo(usize);
        impl_from!(usize => Foo);
        impl_from_for_primitive!(usize; from Foo);

        static_assertions::assert_impl_all!(Foo: From<usize>);
        static_assertions::assert_impl_all!(usize: From<Foo>);

        let foo = Foo::from(42);
        assert_eq!(foo.0, 42);
    }

    #[test]
    fn newtype_generic() {
        struct Foo<T>(Rc<T>);
        impl_from!(<T> in Rc<T> => Foo<T>);
        impl_into!(<T> in Foo<T> => Rc<T>);

        let foo = Foo::from(Rc::new(42_usize));
        assert_eq!(*foo.0, 42);
    }

    #[test]
    fn named_field() {
        struct Foo {
            inner: usize,
        }
        impl_from!(usize => Foo : inner);
        impl_into!(Foo => usize : inner);

        static_assertions::assert_impl_all!(Foo: From<usize>, Into<usize>);

        let foo = Foo::from(42);
        assert_eq!(foo.inner, 42);

        struct MultiFoo {
            small: u8,
            big: u64,
        }
        impl_into!(MultiFoo => u8 : small);
        impl_into!(MultiFoo => u64 : big);

        static_assertions::assert_impl_all!(MultiFoo: Into<u8>, Into<u64>);

        let foo = MultiFoo { small: 4, big: 42 };
        assert_eq!(foo.small, 4);
        assert_eq!(foo.big, 42);
    }

    #[test]
    fn named_field_generic() {
        struct Foo<T> {
            inner: Rc<T>,
        }
        impl_from!(<T> in Rc<T> => Foo<T> : inner);
        impl_into!(<T> in Foo<T> => Rc<T> : inner);

        let foo = Foo::from(Rc::new(42_usize));
        assert_eq!(*foo.inner, 42);
    }
}
