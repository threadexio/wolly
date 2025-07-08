use std::iter::Peekable;

use miniarg::split_args::SplitArgs;

pub type ParseStream<'a> = Peekable<SplitArgs<'a>>;

pub fn parse_stream(s: &str) -> ParseStream<'_> {
    SplitArgs::new(s).peekable()
}

pub trait Parse {
    type Output;
    type Error;

    fn parse(stream: &mut ParseStream<'_>) -> Result<Self::Output, Self::Error>;
}
