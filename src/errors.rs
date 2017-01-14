// use std::error;
use std::fmt;
use std::io;
use std::iter;
use std::convert::From;

#[derive(Debug)]
pub enum DecodeError {
    IncompleteData {
        expected_size: usize,
        actual_size: usize,
    },
    InvalidHeader { line: String, position: usize },
    InvalidChecksum,
    IoError(io::Error),
}

impl From<io::Error> for DecodeError {
    fn from(error: io::Error) -> DecodeError {
        DecodeError::IoError(error)
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecodeError::IncompleteData { ref expected_size, ref actual_size } => {
                write!(f,
                       "Incomplete data: expected size {}, actual size {}",
                       expected_size,
                       actual_size)
            }
            DecodeError::InvalidHeader { ref line, position } => {
                write!(f,
                       "Invalid header: \n{}\n{}^",
                       line,
                       iter::repeat(" ").take(position).collect::<String>())
            }
            DecodeError::InvalidChecksum => write!(f, "Invalid checksum"),
            DecodeError::IoError(ref err) => write!(f, "IO error {}", err),
        }
    }
}

// impl error::Error for DecodeError {}
