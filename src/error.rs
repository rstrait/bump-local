use std::fmt;

/// Reset is only allowed when single Bump reference exists
pub struct ResetError;

impl std::error::Error for ResetError {}

impl fmt::Display for ResetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("reset is only allowed when single Bump reference exists")
    }
}

impl fmt::Debug for ResetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}
