use nom::types::CompleteByteSlice;
use nom::{line_ending, not_line_ending, space0};

/// at least a newline
/// maybe more and comments
named!(pub line_sep<&[u8], ()>,
    preceded!(
      many1!(do_parse!(
        space0 >>
        opt!(preceded!(tag!("#"), not_line_ending)) >>
        line_ending >>
        ( () )
      )),
      map!(space0, |_| ())
    )
);

named!(pub opt_line_sep<&[u8], ()>,
    preceded!(
      many0!(do_parse!(
        space0 >>
        opt!(preceded!(tag!("#"), not_line_ending)) >>
        line_ending >>
        ( () )
      )),
      map!(space0, |_| ())
    )
);

named!(maybe_line_sep<CompleteByteSlice, ()>,
    preceded!(
      many0!(do_parse!(
        space0 >>
        opt!(preceded!(tag!("#"), not_line_ending)) >>
        line_ending >>
        ( () )
      )),
      map!(space0, |_| ())
    )
);

pub fn all_comments_or_space(bs: &[u8]) -> bool {
    if bs.len() == 0
        || String::from_utf8_lossy(bs)
            .chars()
            .all(|c| c.is_whitespace())
    {
        return true;
    }
    let c = CompleteByteSlice(bs);
    match maybe_line_sep(c) {
        Ok((r, _p)) => r.len() == 0,
        Err(e) => {
            eprintln!("error {:?}", e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::all_comments_or_space;
    use crate::util::BsDisp;
    #[test]
    fn test() {
        let comments = ["", "  ", " #lorem\n", "# ipsum\n#\n", " \n\n"];
        let not_comments = ["abcd"];
        for x in comments.iter().map(|x| x.as_bytes()) {
            let b = all_comments_or_space(x);
            assert!(
                b,
                "\"{}\" isn't correctly identified as comment",
                BsDisp::new(x)
            );
        }
        for x in not_comments.iter().map(|x| x.as_bytes()) {
            let b = all_comments_or_space(x);
            assert!(
                !b,
                "\"{}\" is incorrectly identified as comment",
                BsDisp::new(x)
            );
        }
    }
}
