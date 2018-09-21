//! parse the configuration file

use nom::{be_u8,be_u16, be_u24, be_u32, ErrorKind};

named!(get_greeting<&str, &str>,
    do_parse!(
        tag_s!("Content-Type: text/x-reflow-decision-tree\nTree-Format: reflow 0.1") >>
        ( "" )
    )
);

