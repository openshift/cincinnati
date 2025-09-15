/// Implement [`AsRef`] for a struct.
///
/// The first argument is that of the struct to create the impl for and the second is the type to
/// produce a reference for.
///
/// # Examples
/// With a newtype struct:
/// ```
/// use impl_more::impl_as_ref;
///
/// struct Foo(String);
/// impl_as_ref!(Foo => String);
/// let foo = Foo("bar".to_owned());
/// assert_eq!(foo.as_ref(), "bar");
/// ```
///
/// With a named field struct and type parameters:
/// ```
/// use impl_more::impl_as_ref;
///
/// struct Foo<T> { inner: T }
/// impl_as_ref!(Foo<T> => inner: T);
/// let foo = Foo { inner: "bar".to_owned() };
/// assert_eq!(foo.as_ref().as_str(), "bar");
/// ```
#[macro_export]
macro_rules! impl_as_ref {
    ($this:ident $(<$($generic:ident),+>)? => $inner:ty) => {
        impl $(<$($generic),+>)? ::core::convert::AsRef<$inner> for $this $(<$($generic),+>)? {
            fn as_ref(&self) -> &$inner {
                &self.0
            }
        }
    };

    ($this:ident $(<$($generic:ident),+>)? => $field:ident : $inner:ty) => {
        impl $(<$($generic),+>)? ::core::convert::AsRef<$inner> for $this $(<$($generic),+>)? {
            fn as_ref(&self) -> &$inner {
                &self.$field
            }
        }
    };
}

/// Implement [`AsMut`] for a struct.
///
/// The first argument is that of the struct to create the impl for and the second is the type to
/// produce a reference for.
///
/// # Examples
/// With a newtype struct:
/// ```
/// use impl_more::{impl_as_ref, impl_as_mut};
///
/// struct Foo(String);
///
/// impl_as_ref!(Foo => String);
/// impl_as_mut!(Foo => String);
///
/// let mut foo = Foo("bar".to_owned());
/// foo.as_mut().push('!');
///
/// assert_eq!(foo.as_ref(), "bar!");
/// ```
///
/// With a named field struct and type parameters:
/// ```
/// use impl_more::{impl_as_ref, impl_as_mut};
///
/// struct Foo<T> { inner: T }
///
/// impl_as_ref!(Foo<T> => inner: T);
/// impl_as_mut!(Foo<T> => inner: T);
///
/// let mut foo = Foo { inner: "bar".to_owned() };
/// foo.as_mut().push('!');
///
/// assert_eq!(foo.as_ref(), "bar!");
/// ```
#[macro_export]
macro_rules! impl_as_mut {
    ($this:ident $(<$($generic:ident),+>)? => $inner:ty) => {
        impl $(<$($generic),+>)? ::core::convert::AsMut<$inner> for $this $(<$($generic),+>)? {
            fn as_mut(&mut self) -> &mut $inner {
                &mut self.0
            }
        }
    };

    ($this:ident $(<$($generic:ident),+>)? => $field:ident : $inner:ty) => {
        impl $(<$($generic),+>)? ::core::convert::AsMut<$inner> for $this $(<$($generic),+>)? {
            fn as_mut(&mut self) -> &mut $inner {
                &mut self.$field
            }
        }
    };
}
