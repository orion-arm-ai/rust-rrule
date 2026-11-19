use chrono::Local;

use super::IanaTz;

/// A wrapper around [`IanaTz`] that is able to represent `Local` timezone also.
///
/// # Usage
///
/// ```
/// use rrule::{IanaTz, Tz};
///
/// let utc = Tz::UTC;
/// let local = Tz::LOCAL;
/// let berlin: Tz = "Europe/Berlin".parse().unwrap();
/// // From `IanaTz`
/// let berlin: Tz = IanaTz::get("Europe/Berlin").unwrap().into();
/// ```
#[derive(Clone, Copy)]
pub enum Tz {
    /// Local timezone
    Local(Local),
    /// IANA timezone, backed by the bundled tz database (see [`IanaTz`])
    Tz(IanaTz),
}

impl Tz {
    /// Name of timezone
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Local(_) => "Local",
            Self::Tz(tz) => tz.name(),
        }
    }

    /// Check if timezone is the Local timezone
    #[must_use]
    pub fn is_local(&self) -> bool {
        match self {
            Self::Local(_) => true,
            Self::Tz(_) => false,
        }
    }

    /// Local timezone
    #[allow(non_upper_case_globals)]
    pub const LOCAL: Self = Self::Local(Local);

    /// UTC timezone
    pub const UTC: Self = Self::Tz(IanaTz::UTC);
}
