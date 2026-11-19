use chrono::{FixedOffset, Local, Offset, TimeZone, Utc};

use std::str::FromStr;

use super::{IanaOffset, IanaTz, Tz};
use crate::ParseError;

impl PartialEq for Tz {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Local(_), Self::Local(_)) => true,
            (Self::Tz(l0), Self::Tz(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl From<Local> for Tz {
    fn from(tz: Local) -> Self {
        Self::Local(tz)
    }
}

impl From<Utc> for Tz {
    fn from(_tz: Utc) -> Self {
        Self::UTC
    }
}

impl From<IanaTz> for Tz {
    fn from(tz: IanaTz) -> Self {
        Self::Tz(tz)
    }
}

impl FromStr for Tz {
    type Err = ParseError;

    /// Parses an IANA timezone name (case-sensitive), e.g. `Europe/Berlin`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        IanaTz::get(s)
            .map(Self::Tz)
            .ok_or_else(|| ParseError::InvalidTimezone(s.into()))
    }
}

impl std::fmt::Debug for Tz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(tz) => tz.fmt(f),
            Self::Tz(tz) => tz.fmt(f),
        }
    }
}

impl std::fmt::Display for Tz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(_tz) => write!(f, "Local"),
            Self::Tz(tz) => tz.fmt(f),
        }
    }
}

#[derive(Clone, Copy)]
pub enum RRuleOffset {
    Local(FixedOffset),
    Tz(IanaOffset),
}

impl std::fmt::Debug for RRuleOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(offset) => offset.fmt(f),
            Self::Tz(offset) => offset.fmt(f),
        }
    }
}

impl std::fmt::Display for RRuleOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(offset) => offset.fmt(f),
            Self::Tz(offset) => offset.fmt(f),
        }
    }
}

impl Offset for RRuleOffset {
    fn fix(&self) -> FixedOffset {
        match self {
            Self::Local(tz) => tz.fix(),
            Self::Tz(tz) => tz.fix(),
        }
    }
}

impl TimeZone for Tz {
    type Offset = RRuleOffset;

    fn from_offset(offset: &Self::Offset) -> Self {
        match offset {
            RRuleOffset::Local(offset) => Self::Local(Local::from_offset(offset)),
            RRuleOffset::Tz(offset) => Self::Tz(IanaTz::from_offset(offset)),
        }
    }

    #[allow(deprecated)]
    fn offset_from_local_date(
        &self,
        local: &chrono::NaiveDate,
    ) -> chrono::LocalResult<Self::Offset> {
        match self {
            Self::Local(tz) => tz
                .from_local_date(local)
                .map(|date| RRuleOffset::Local(*date.offset())),
            Self::Tz(tz) => tz
                .from_local_date(local)
                .map(|date| RRuleOffset::Tz(*date.offset())),
        }
    }

    fn offset_from_local_datetime(
        &self,
        local: &chrono::NaiveDateTime,
    ) -> chrono::LocalResult<Self::Offset> {
        match self {
            Self::Local(tz) => tz
                .from_local_datetime(local)
                .map(|date| RRuleOffset::Local(*date.offset())),
            Self::Tz(tz) => tz
                .from_local_datetime(local)
                .map(|date| RRuleOffset::Tz(*date.offset())),
        }
    }

    #[allow(deprecated)]
    fn offset_from_utc_date(&self, utc: &chrono::NaiveDate) -> Self::Offset {
        match self {
            Self::Local(tz) => RRuleOffset::Local(*tz.from_utc_date(utc).offset()),
            Self::Tz(tz) => RRuleOffset::Tz(*tz.from_utc_date(utc).offset()),
        }
    }

    fn offset_from_utc_datetime(&self, utc: &chrono::NaiveDateTime) -> Self::Offset {
        match self {
            Self::Local(tz) => RRuleOffset::Local(*tz.from_utc_datetime(utc).offset()),
            Self::Tz(tz) => RRuleOffset::Tz(*tz.from_utc_datetime(utc).offset()),
        }
    }
}
