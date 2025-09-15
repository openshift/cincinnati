use proc_macro2::TokenStream;
use syn::{parse_str, Fields, Ident, Lit, Meta, NestedMeta, Path};
use synstructure::{decl_derive, quote, AddBounds, BindingInfo, Structure};

#[cfg(test)]
mod tests;

decl_derive!([Debug, attributes(debug)] => custom_debug_derive);

fn custom_debug_derive(mut s: Structure) -> TokenStream {
    fn get_metas<'a>(b: &BindingInfo<'a>) -> impl Iterator<Item = NestedMeta> + 'a {
        let debug_attr = parse_str::<Path>("debug").unwrap();

        b.ast()
            .attrs
            .iter()
            .filter(move |attr| attr.path == debug_attr)
            .flat_map(|attr| attr.parse_meta())
            .flat_map(|meta| match meta {
                Meta::List(list) => list.nested,
                _ => panic!("Invalid debug attribute"),
            })
    }

    s.add_bounds(AddBounds::Fields);

    let skip_ident: Ident = parse_str("skip").unwrap();
    s.filter(|b| {
        for meta in get_metas(b) {
            if let NestedMeta::Meta(Meta::Path(ref path)) = meta {
                if path.get_ident().map(|i| i == &skip_ident).unwrap_or(false) {
                    return false;
                }
            }
        }
        true
    });

    let variants = s.each_variant(|variant| {
        let name = variant.ast().ident.to_string();
        let debug_helper = match variant.ast().fields {
            | Fields::Named(_)
            | Fields::Unit => quote! { debug_struct },
            | Fields::Unnamed(_) => quote! { debug_tuple },
        };

        let variant_body = variant.bindings().iter().map(|b| {
            let mut format = None;

            for meta in get_metas(b) {
                match meta {
                    NestedMeta::Meta(Meta::NameValue(nv)) => {
                        let value = nv.lit;
                        let ident = nv.path.get_ident().map(|i| i.to_string());
                        let ident_ref = ident.as_ref().map(|s| -> &str { s });
                        format = Some(match ident_ref {
                            Some("format") => quote! { &format_args!(#value, #b) },
                            Some("with") => match value {
                                Lit::Str(fun) => {
                                    let fun = fun.parse::<Path>().unwrap();
                                    quote! {
                                        &{
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
                                                data: #b,
                                                fmt: #fun,
                                            }
                                        }
                                    }
                                },
                                _ => panic!("Invalid 'with' value"),
                            },
                            _ => panic!("Unknown key '{}'", quote!(nv.path)),
                        })
                    },
                    _ => panic!("Invalid debug attribute"),
                }
            }

            let format = format.unwrap_or_else(|| quote! { #b });

            if let Some(ref name) = b.ast().ident.as_ref().map(<_>::to_string) {
                quote! {
                    s.field(#name, #format);
                }
            } else {
                quote! {
                    s.field(#format);
                }
            }
        });

        quote! {
            let mut s = f.#debug_helper(#name);
            #(#variant_body)*
            s.finish()
        }
    });

    s.gen_impl(quote! {
        gen impl ::core::fmt::Debug for @Self {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    #variants
                }
            }
        }
    })
}
