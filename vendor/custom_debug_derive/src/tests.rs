use super::custom_debug_derive;
use synstructure::test_derive;

#[test]
fn test_default_struct() {
    test_derive! {
        custom_debug_derive {
            struct Point {
                x: f32,
                y: f32,
            }
        }

        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_core_fmt_Debug_FOR_Point: () = {
                impl ::core::fmt::Debug for Point {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        match self {
                            Point { x: ref __binding_0, y: ref __binding_1, } => {
                                let mut s = f.debug_struct("Point");
                                s.field("x", __binding_0);
                                s.field("y", __binding_1);
                                s.finish()
                            }
                        }
                    }
                }
            };
        }
    }
}

#[test]
fn test_format() {
    test_derive! {
        custom_debug_derive {
            struct Point {
                #[debug(format = "{:.02}")]
                x: f32,
                y: f32,
            }
        }

        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_core_fmt_Debug_FOR_Point: () = {
                impl ::core::fmt::Debug for Point {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        match self {
                            Point { x: ref __binding_0, y: ref __binding_1, } => {
                                let mut s = f.debug_struct("Point");
                                s.field("x", &format_args!("{:.02}", __binding_0));
                                s.field("y", __binding_1);
                                s.finish()
                            }
                        }
                    }
                }
            };
        }

        no_build
    }
}

#[test]
fn test_with() {
    test_derive! {
        custom_debug_derive {
            struct Point {
                #[debug(with = "my_fmt")]
                x: f32,
                y: f32,
            }
        }

        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_core_fmt_Debug_FOR_Point: () = {
                impl ::core::fmt::Debug for Point {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        match self {
                            Point { x: ref __binding_0, y: ref __binding_1, } => {
                                let mut s = f.debug_struct("Point");
                                s.field("x", &{
                                    struct DebugWith<'a, T: 'a> {
                                        data: &'a T,
                                        fmt: fn(&T, &mut ::core::fmt::Formatter) -> ::core::fmt::Result,
                                    }

                                    impl<'a, T: 'a> ::core::fmt::Debug for DebugWith<'a, T> {
                                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                                            (self.fmt)(self.data, f)
                                        }
                                    }

                                    DebugWith {
                                        data: __binding_0,
                                        fmt: my_fmt,
                                    }
                                });
                                s.field("y", __binding_1);
                                s.finish()
                            }
                        }
                    }
                }
            };
        }

        no_build
    }
}

#[test]
fn test_skip() {
    test_derive! {
        custom_debug_derive {
            struct Point {
                x: f32,
                #[debug(skip)]
                y: f32,
                z: f32,
            }
        }

        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_core_fmt_Debug_FOR_Point: () = {
                impl ::core::fmt::Debug for Point {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        match self {
                            Point { x: ref __binding_0, z: ref __binding_2, .. } => {
                                let mut s = f.debug_struct("Point");
                                s.field("x", __binding_0);
                                s.field("z", __binding_2);
                                s.finish()
                            }
                        }
                    }
                }
            };
        }

        no_build
    }
}

#[test]
fn test_enum() {
    test_derive! {
        custom_debug_derive {
            enum Foo {
                Bar(#[debug(format = "{}i32")] i32, String),
                Quux { x: f32, y: f32 },
            }
        }

        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_core_fmt_Debug_FOR_Foo: () = {
                impl ::core::fmt::Debug for Foo {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        match self {
                            Foo::Bar(ref __binding_0, ref __binding_1,) => {
                                let mut s = f.debug_tuple("Bar");
                                s.field(&format_args!("{}i32", __binding_0));
                                s.field(__binding_1);
                                s.finish()
                            }
                            Foo::Quux { x: ref __binding_0, y: ref __binding_1, } => {
                                let mut s = f.debug_struct("Quux");
                                s.field("x", __binding_0);
                                s.field("y", __binding_1);
                                s.finish()
                            }
                        }
                    }
                }
            };
        }

        no_build
    }
}

#[test]
fn test_bounds_on_skipped() {
    #![allow(dead_code)]

    use std::{fmt::*, marker::PhantomData};

    struct NoDebug;
    struct TemplatedType<T> {
        _phantom: PhantomData<T>,
    };
    impl<T> Debug for TemplatedType<T>
    where
        T: Debug,
    {
        fn fmt(&self, f: &mut Formatter) -> Result {
            write!(f, "TemplatedType")
        }
    }

    test_derive! {
        custom_debug_derive {
            struct WantDebug<T> {
                foo: TemplatedType<T>,
                #[debug(skip)]
                bar: Debug,
            }
        }

        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_core_fmt_Debug_FOR_WantDebug: () = {
                impl<T> ::core::fmt::Debug for WantDebug<T>
                    where
                        TemplatedType<T>: ::core::fmt::Debug
                {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        match self {
                            WantDebug { foo: ref __binding_0, .. } => {
                                let mut s = f.debug_struct("WantDebug");
                                s.field("foo", __binding_0);
                                s.finish()
                            }
                        }
                    }
                }
            };
        }

        no_build
    }
}

#[test]
fn test_bounds_on_fields_only() {
    #![allow(dead_code)]

    use std::marker::PhantomData;

    struct NoDebug;
    struct TemplatedType<T> {
        _phantom: PhantomData<T>,
    };

    test_derive! {
        custom_debug_derive {
            struct WantDebug<T> {
                foo: TemplatedType<T>,
                bar: TemplatedType<NoDebug>,
                needs_debug: T,
            }
        }

        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_core_fmt_Debug_FOR_WantDebug: () = {
                impl<T> ::core::fmt::Debug for WantDebug<T>
                    where
                        TemplatedType<T>: ::core::fmt::Debug,
                        T: ::core::fmt::Debug
                {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        match self {
                            WantDebug { foo: ref __binding_0, bar: ref __binding_1, needs_debug: ref __binding_2, } => {
                                let mut s = f.debug_struct("WantDebug");
                                s.field("foo", __binding_0);
                                s.field("bar", __binding_1);
                                s.field("needs_debug", __binding_2);
                                s.finish()
                            }
                        }
                    }
                }
            };
        }

        no_build
    }
}
