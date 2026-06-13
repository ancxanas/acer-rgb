use std::fmt;
use std::io;

#[derive(Debug)]
pub enum KbdError {
    Io(io::Error),
    Json(serde_json::Error),
    NoLeds,
    NoFrames,
    InvalidProfileName(String),
}

impl fmt::Display for KbdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KbdError::Io(e) => write!(f, "io: {}", e),
            KbdError::Json(e) => write!(f, "json: {}", e),
            KbdError::NoLeds => write!(f, "no LED entries in profile"),
            KbdError::NoFrames => write!(f, "no frames generated"),
            KbdError::InvalidProfileName(n) => write!(f, "invalid profile name: '{}'", n),
        }
    }
}

impl std::error::Error for KbdError {}

impl From<io::Error> for KbdError {
    fn from(e: io::Error) -> KbdError {
        KbdError::Io(e)
    }
}

impl From<serde_json::Error> for KbdError {
    fn from(e: serde_json::Error) -> KbdError {
        KbdError::Json(e)
    }
}

pub type Result<T> = std::result::Result<T, KbdError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kbd_error_display() {
        let e = KbdError::NoLeds;
        assert_eq!(format!("{}", e), "no LED entries in profile");
        let e = KbdError::NoFrames;
        assert_eq!(format!("{}", e), "no frames generated");
        let e = KbdError::InvalidProfileName("../foo".into());
        assert_eq!(format!("{}", e), "invalid profile name: '../foo'");
    }

    #[test]
    fn test_kbd_error_impl_std_error() {
        let e = KbdError::NoLeds;
        let _: &dyn std::error::Error = &e;
    }
}
