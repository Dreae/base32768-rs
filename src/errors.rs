use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct Base32768Error {
    msg: String,
}

impl Error for Base32768Error {
    fn description(&self) -> &str {
        &self.msg
    }
}

impl fmt::Display for Base32768Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.msg, f)
    }
}

impl Base32768Error {
    pub fn new(msg: String) -> Base32768Error {
        Base32768Error {
            msg: msg,
        }
    }
}