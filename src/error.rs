use std::io::Error;

pub fn not_found() -> Box<dyn std::error::Error> {
    Box::new(Error::new(
        std::io::ErrorKind::NotFound,
        "Task not found, run -- list to check current numbers",
    ))
}

pub fn invalid_input() -> Box<dyn std::error::Error> {
    Box::new(Error::new(
        std::io::ErrorKind::InvalidInput,
        "Nothing to change",
    ))
}
