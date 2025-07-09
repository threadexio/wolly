use std::fmt::{self, Display, Formatter, Result};

use owo_colors::OwoColorize;

#[doc(hidden)]
pub struct DebugDisplay<T>(T);

impl<T> Display for DebugDisplay<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

#[doc(hidden)]
#[expect(non_camel_case_types)]
pub struct _display<'a, T: ?Sized>(pub &'a T);

impl<'a, T> Display for _display<'a, &T>
where
    _display<'a, T>: Display,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        _display(*self.0).fmt(f)
    }
}

macro_rules! display {
    ($x:expr) => {{ $crate::display::_display(&$x) }};
}

impl Display for _display<'_, std::io::Error> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.bright_red().fmt(f)
    }
}

impl Display for _display<'_, std::time::Duration> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        DebugDisplay(self.0).dimmed().fmt(f)
    }
}

impl Display for _display<'_, std::net::IpAddr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.bright_yellow().fmt(f)
    }
}

impl Display for _display<'_, std::net::SocketAddr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.bright_yellow().fmt(f)
    }
}

impl Display for _display<'_, crate::config::address::Address> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.bright_yellow().fmt(f)
    }
}
