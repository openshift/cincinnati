/// Implement [`Deref`] for a struct.
///
/// The first argument is that of the newtype struct to create the impl for and the second is the
/// deref target type. The third argument is required for non-newtype structs and is the name of the
/// field to deref to. Type parameters require special handling, see examples.
///
/// Also see [`impl_deref_mut`], [`impl_deref_and_mut`], and [`forward_deref_and_mut`].
///
/// # Examples
/// With a newtype struct:
/// ```
/// struct Foo(String);
/// impl_more::impl_deref!(Foo => String);
///
/// let mut foo = Foo("bar".to_owned());
/// assert_eq!(foo.len(), 3);
/// ```
///
/// With a named field struct and type parameter:
/// ```
/// struct MyStruct<T> { msg: T };
/// impl_more::impl_deref!(<T> in MyStruct<T> => msg: T);
///
/// let foo = MyStruct { msg: "two".to_owned() };
/// assert_eq!(foo.len(), 3);
/// ```
///
/// [`Deref`]: core::ops::Deref
/// [`impl_deref_mut`]: crate::impl_deref_mut
/// [`impl_deref_and_mut`]: crate::impl_deref_and_mut
/// [`forward_deref_and_mut`]: crate::forward_deref_and_mut
#[macro_export]
macro_rules! impl_deref {
    (<$($generic:ident),+> in $this:ty => $target:ty) => {
        impl <$($generic),+> ::core::ops::Deref for $this {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };

    (<$($generic:ident),+> in $this:ty => $field:ident : $target:ty) => {
        impl <$($generic),+> ::core::ops::Deref for $this {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$field
            }
        }
    };

    ($this:ty => $target:ty) => {
        impl ::core::ops::Deref for $this {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };

    ($this:ty => $field:ident : $target:ty) => {
        impl ::core::ops::Deref for $this {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$field
            }
        }
    };
}

/// Implement [`DerefMut`] for a struct.
///
/// The first argument is that of the struct to create the impl for and this type must also
/// implement [`Deref`]. The second argument is required for non-newtype structs and is the field
/// to deref to.
///
/// This macro has the same type parameter support and format as [`impl_deref`].
///
/// Also see [`impl_deref`], [`impl_deref_and_mut`], and [`forward_deref_and_mut`].
///
/// # Examples
/// With a newtype struct:
/// ```
/// use impl_more::{impl_deref, impl_deref_mut};
///
/// struct Foo(String);
///
/// impl_deref!(Foo => String);
/// impl_deref_mut!(Foo);
///
/// let mut foo = Foo("bar".to_owned());
/// foo.push('!');
///
/// assert_eq!(*foo, "bar!");
/// ```
///
/// With a named field struct and type parameter:
/// ```
/// struct Foo<T> { msg: T };
/// impl_more::impl_deref!(<T> in Foo<T> => msg: T);
/// impl_more::impl_deref_mut!(<T> in Foo<T> => msg);
///
/// let mut foo = Foo { msg: "bar".to_owned() };
/// foo.push('!');
///
/// assert_eq!(*foo, "bar!");
/// ```
///
/// [`Deref`]: core::ops::Deref
/// [`DerefMut`]: core::ops::DerefMut
/// [`impl_deref`]: crate::impl_deref
/// [`impl_deref_and_mut`]: crate::impl_deref_and_mut
/// [`forward_deref_and_mut`]: crate::forward_deref_and_mut
#[macro_export]
macro_rules! impl_deref_mut {
    (<$($generic:ident),+> in $this:ty) => {
        impl <$($generic),+> ::core::ops::DerefMut for $this {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };

    (<$($generic:ident),+> in $this:ty => $field:ident) => {
        impl <$($generic),+> ::core::ops::DerefMut for $this {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$field
            }
        }
    };

    ($this:ty) => {
        impl ::core::ops::DerefMut for $this {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };

    ($this:ty => $field:ident) => {
        impl ::core::ops::DerefMut for $this {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$field
            }
        }
    };
}

/// Implements [`Deref`] and [`DerefMut`] by forwarding through an inner field's implementation.
///
/// Use the `ref <type>` form for deref-ing to types with lifetimes like `&str`. For newtype
/// structs, only the struct name and deref target type is necessary.
///
/// This macro has the same type parameter support and format as [`impl_deref`].
///
/// Also see [`forward_deref_and_mut`].
///
/// # Examples
/// ```
/// struct MyNewTypeStruct(String);
/// impl_more::impl_deref_and_mut!(MyNewTypeStruct => String);
///
/// let foo = MyNewTypeStruct("one".to_owned());
/// let foo_ref: &String = &foo;
///
/// // Unlike `forward_deref_and_mut`, this macro will not forward the deref implementation
/// // through the named type. Even so, in some cases Rust will be able to support these cases.
///
/// let foo_ref: &str = &foo;
///
/// fn accepts_string_slice(_: &str) {}
/// accepts_string_slice(&foo);
/// ```
///
/// [`Deref`]: core::ops::Deref
/// [`DerefMut`]: core::ops::DerefMut
/// [`impl_deref`]: crate::impl_deref
/// [`forward_deref_and_mut`]: crate::forward_deref_and_mut
#[macro_export]
macro_rules! impl_deref_and_mut {
    (<$($generic:ident),+> in $this:ty => $target:ty) => {
        impl <$($generic),+> ::core::ops::Deref for $this {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl <$($generic),+> ::core::ops::DerefMut for $this {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };

    (<$($generic:ident),+> in $this:ty => $field:ident : $target:ty) => {
        impl <$($generic),+> ::core::ops::Deref for $this {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$field
            }
        }

        impl <$($generic),+> ::core::ops::DerefMut for $this {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$field
            }
        }
    };

    ($this:ty => $target:ty) => {
        impl ::core::ops::Deref for $this {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::core::ops::DerefMut for $this {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };

    ($this:ty => $field:ident : $target:ty) => {
        impl ::core::ops::Deref for $this {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$field
            }
        }

        impl ::core::ops::DerefMut for $this {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$field
            }
        }
    };
}

/// Implements [`Deref`] and [`DerefMut`] by forwarding through an inner field's implementation.
///
/// Use the `ref <type>` form for deref-ing to types with lifetimes like `&str`. For newtype
/// structs, only the struct name and deref target type is necessary.
///
/// Also see [`impl_deref_and_mut`].
///
/// # Examples
/// With a newtype struct:
/// ```
/// # fn accepts_string_slice(_: &str) {}
/// # fn accepts_mut_string_slice(_: &mut str) {}
/// #
/// struct MyNewTypeStruct(String);
/// impl_more::forward_deref_and_mut!(MyNewTypeStruct => ref str);
/// let mut foo = MyNewTypeStruct("one".to_owned());
/// let foo_ref: &str = &foo;
/// accepts_string_slice(&foo);
/// accepts_mut_string_slice(&mut foo);
/// ```
///
/// [`impl_deref_and_mut`]: crate::impl_deref_and_mut
/// [`Deref`]: core::ops::Deref
/// [`DerefMut`]: core::ops::DerefMut
#[macro_export]
macro_rules! forward_deref_and_mut {
    ($ty:ty => $target:ty) => {
        impl ::core::ops::Deref for $ty {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                ::core::ops::Deref::deref(&self.0)
            }
        }

        impl ::core::ops::DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                ::core::ops::DerefMut::deref_mut(&mut self.0)
            }
        }
    };

    ($ty:ty => ref $target:ty) => {
        impl<'__impl_more_a> ::core::ops::Deref for $ty {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                ::core::ops::Deref::deref(&self.0)
            }
        }

        impl<'__impl_more_a> ::core::ops::DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                ::core::ops::DerefMut::deref_mut(&mut self.0)
            }
        }
    };

    ($ty:ty => $field:ident : $target:ty) => {
        impl ::core::ops::Deref for $ty {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                ::core::ops::Deref::deref(&self.$field)
            }
        }

        impl ::core::ops::DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                ::core::ops::DerefMut::deref_mut(&mut self.$field)
            }
        }
    };

    ($ty:ty => $field:ident : ref $target:ty) => {
        impl<'__impl_more_a> ::core::ops::Deref for $ty {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                ::core::ops::Deref::deref(&self.$field)
            }
        }

        impl<'__impl_more_a> ::core::ops::DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                ::core::ops::DerefMut::deref_mut(&mut self.$field)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use alloc::{borrow::ToOwned as _, string::String};
    use core::ops::{Deref, DerefMut};

    fn accepts_string_slice(_: &str) {}
    fn accepts_mut_string_slice(_: &mut str) {}

    struct Foo1(String);
    impl_deref_and_mut!(Foo1 => String);
    static_assertions::assert_impl_all!(
        Foo1:
        // impls
        Deref<Target = String>,
        DerefMut<Target = String>,
    );

    struct Foo2(String);
    forward_deref_and_mut!(Foo2 => ref str);
    static_assertions::assert_impl_all!(
        Foo2:
        // impls
        Deref,
        DerefMut,
    );

    struct SingleGeneric<T>(T);
    impl_deref!(<T> in SingleGeneric<T> => T);
    impl_deref_mut!(<T> in SingleGeneric<T>);
    static_assertions::assert_impl_all!(
        SingleGeneric<usize>:
        // impls
        Deref<Target = usize>,
        DerefMut<Target = usize>,
    );

    struct SingleGenericCombined<T>(T);
    impl_deref_and_mut!(<T> in SingleGenericCombined<T> => T);
    static_assertions::assert_impl_all!(
        SingleGenericCombined<usize>:
        // impls
        Deref<Target = usize>,
        DerefMut<Target = usize>,
    );

    struct DoubleGeneric<T, U> {
        t: T,
        _u: U,
    }
    impl_deref!(<T, U> in DoubleGeneric<T, U> => t: T);
    impl_deref_mut!(<T, U> in DoubleGeneric<T, U> => t);
    static_assertions::assert_impl_all!(
        DoubleGeneric<usize, u32>:
        // impls
        Deref<Target = usize>,
        DerefMut<Target = usize>,
    );

    struct DoubleGenericCombined<T, U> {
        t: T,
        _u: U,
    }
    impl_deref_and_mut!(<T, U> in DoubleGenericCombined<T, U> => t: T);
    static_assertions::assert_impl_all!(
        DoubleGenericCombined<usize, u32>:
        // impls
        Deref<Target = usize>,
        DerefMut<Target = usize>,
    );

    #[test]
    fn foo2_impls() {
        let mut foo = Foo2("".to_owned());
        accepts_string_slice(&foo);
        accepts_mut_string_slice(&mut foo);
    }

    struct Foo3 {
        msg: String,
    }
    forward_deref_and_mut!(Foo3 => msg: ref str);
    static_assertions::assert_impl_all!(
        Foo3:
        // impls
        Deref,
        DerefMut,
    );

    #[test]
    fn foo3_impls() {
        let mut foo = Foo3 { msg: "".to_owned() };
        accepts_string_slice(&foo);
        accepts_mut_string_slice(&mut foo);
    }
}
