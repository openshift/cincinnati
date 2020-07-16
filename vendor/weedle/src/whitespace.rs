use {CompleteStr, IResult};

pub fn sp(input: CompleteStr) -> IResult<CompleteStr, CompleteStr> {
    recognize!(
        input,
        many0!(
            alt!(
                do_parse!(tag!("//") >> take_until!("\n") >> char!('\n') >> (()))
                |
                map!(
                    take_while1!(|c| c == '\t' || c == '\n' || c == '\r' || c == ' '),
                    |_| ()
                )
                |
                do_parse!(
                    tag!("/*") >>
                    take_until!("*/") >>
                    tag!("*/") >>
                    (())
                )
            )
        )
    )
}

/// ws! also ignores line & block comments
macro_rules! ws (
    ($i:expr, $($args:tt)*) => ({
        use $crate::whitespace::sp;

        do_parse!($i,
            sp >>
            s: $($args)* >>
            sp >>
            (s)
        )
    });
);
