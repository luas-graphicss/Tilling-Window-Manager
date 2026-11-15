use tilling_core::layout::{Gaps, LayoutKind};

use crate::diagnostics::Report;
use crate::limits::MAX_GAP;
use crate::schema::{RawLayout, RawLayoutKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutConfig {
    pub gaps: Gaps,
    pub default: LayoutKind,
}

pub(crate) fn validate(raw: &RawLayout, report: &mut Report) -> LayoutConfig {
    let gaps = [
        ("layout.inner_gap", raw.inner_gap),
        ("layout.outer_gap", raw.outer_gap),
    ];
    for (field, gap) in gaps {
        if gap > MAX_GAP {
            report.reject(field, format!("must not exceed {MAX_GAP}"));
        }
    }
    LayoutConfig {
        gaps: Gaps {
            inner: raw.inner_gap,
            outer: raw.outer_gap,
        },
        default: raw.default.into(),
    }
}

impl From<RawLayoutKind> for LayoutKind {
    fn from(raw: RawLayoutKind) -> Self {
        match raw {
            RawLayoutKind::Bsp => Self::Bsp,
            RawLayoutKind::MasterStack => Self::MasterStack,
            RawLayoutKind::Monocle => Self::Monocle,
        }
    }
}
