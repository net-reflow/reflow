use nom::{ line_ending, multispace0, not_line_ending, space0};

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