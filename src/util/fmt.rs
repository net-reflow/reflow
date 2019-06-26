use std::fmt;

/// Since large amount of byte strings are in fact
/// ASCII strings or contain a lot of ASCII strings (e. g. HTTP),
/// it is convenient to print strings as ASCII when possible.
///
/// copied from the bytes crate
pub struct BsDisp<'a>(pub &'a [u8]);

impl<'a> BsDisp<'a> {
    pub fn new(bs: &'a [u8]) -> BsDisp<'a> {
        BsDisp(bs)
    }
}

impl<'a> fmt::Display for BsDisp<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for &c in self.0 {
            // https://doc.rust-lang.org/reference.html#byte-escapes
            if c == b'\n' {
                write!(fmt, "\\n")?;
            } else if c == b'\r' {
                write!(fmt, "\\r")?;
            } else if c == b'\t' {
                write!(fmt, "\\t")?;
            } else if c == b'\\' || c == b'"' {
                write!(fmt, "\\{}", c as char)?;
            } else if c == b'\0' {
                write!(fmt, "\\0")?;
            // ASCII printable
            } else if c >= 0x20 && c < 0x7f {
                write!(fmt, "{}", c as char)?;
            } else {
                write!(fmt, "\\x{:02x}", c)?;
            }
        }
        Ok(())
    }
}
