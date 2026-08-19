use std::io::{Error, ErrorKind};

pub fn not_found() -> Box<dyn std::error::Error> {
    Box::new(Error::new(
        ErrorKind::NotFound,
        "Task not found, run `list` to check current numbers",
    ))
}

pub fn invalid_input_noting_change() -> Box<dyn std::error::Error> {
    Box::new(Error::new(
        ErrorKind::InvalidInput,
        "Nothing to change, Please enter one or more subcommands",
    ))
}

pub fn invalid_input_time() -> Box<dyn std::error::Error> {
    Box::new(Error::new(
        ErrorKind::InvalidInput,
        "The date format must be {%Y-%m-%d} or {%Y-%m-%dT%h:%M:%S}",
    ))
}
