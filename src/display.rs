use std::fmt::{self, Display, Formatter, Result};

use owo_colors::OwoColorize;

pub mod style {
    use super::*;

    macro_rules! style {
        ($name:ident: $($style:tt)*) => {
            pub fn $name<'a, T>(x: &'a T) -> impl Display + 'a
            where
                T: Display
            {
                struct D<'b, U>(&'b U);

                impl<'b, U> Display for D<'b, U>
                where
                    U: Display
                {
                    fn fmt(&self, f: &mut Formatter<'_>)  -> Result {
                        self.0 $($style)* .fmt(f)
                    }
                }

                D(x)
            }
        };
    }

    style!(error: .bright_red());
    style!(path: .bright_blue());
    style!(address: .bright_yellow());
    style!(time: .dimmed());
}

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

    (@impl $t:ty as $style:ident) => {
        impl std::fmt::Display for $crate::display::_display<'_, $t> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                $crate::display::style::$style(self.0).fmt(f)
            }
        }
    };
}

display!(@impl std::io::Error as error);
display!(@impl std::net::IpAddr as address);
display!(@impl std::net::SocketAddr as address);

impl Display for _display<'_, std::time::Duration> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self::style::time(&DebugDisplay(self.0)).fmt(f)
    }
}

impl Display for _display<'_, std::path::Path> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self::style::path(&self.0.display()).fmt(f)
    }
}

impl Display for _display<'_, std::path::PathBuf> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self::style::path(&self.0.display()).fmt(f)
    }
}
