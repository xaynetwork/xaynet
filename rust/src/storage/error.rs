use std::{error::Error, fmt};

#[derive(Debug)]
pub enum StoreError {
    Read,
    Convert,
}

// Allow the use of "{}" format specifier
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StoreError::Read => write!(f, "Read Error!",),
            StoreError::Convert => write!(f, "Convert Error!",),
        }
    }
}

// Allow this type to be treated like an error
impl Error for StoreError {
    fn description(&self) -> &str {
        match *self {
            StoreError::Read => "Read failed!",
            StoreError::Convert => "Convert failed!",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            StoreError::Read => None,
            StoreError::Convert => None,
        }
    }
}
