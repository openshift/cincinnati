use std::iter::Peekable;

use proc_macro::{token_stream, TokenStream};

use crate::date::Date;
use crate::error::Error;
use crate::offset::Offset;
use crate::time::Time;
use crate::to_tokens::ToTokens;
use crate::{date, offset, time};

pub(crate) struct DateTime {
    date: Date,
    time: Time,
    offset: Option<Offset>,
}

pub(crate) fn parse(chars: &mut Peekable<token_stream::IntoIter>) -> Result<DateTime, Error> {
    let date = date::parse(chars)?;
    let time = time::parse(chars)?;
    #[allow(clippy::unnested_or_patterns)]
    let offset = match offset::parse(chars) {
        Ok(offset) => Some(offset),
        Err(Error::UnexpectedEndOfInput) | Err(Error::MissingComponent { name: "sign", .. }) => {
            None
        }
        Err(err) => return Err(err),
    };

    if let Some(token) = chars.peek() {
        return Err(Error::UnexpectedToken {
            tree: token.clone(),
        });
    }

    Ok(DateTime { date, time, offset })
}

impl ToTokens for DateTime {
    fn into_token_stream(self) -> TokenStream {
        let (type_name, maybe_offset) = match self.offset {
            Some(offset) => (quote!(OffsetDateTime), quote!(.assume_offset(#(offset)))),
            None => (quote!(PrimitiveDateTime), quote!()),
        };

        quote! {{
            const DATE_TIME: ::time::#(type_name) = ::time::PrimitiveDateTime::new(
                #(self.date),
                #(self.time),
            ) #(maybe_offset);
            DATE_TIME
        }}
    }
}
