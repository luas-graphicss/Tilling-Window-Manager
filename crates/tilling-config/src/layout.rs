use tilling_core::layout::Gaps;

use crate::diagnostics::Report;
use crate::limits::MAX_GAP;
use crate::schema::RawLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutConfig {
    pub gaps: Gaps,
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
    }
}
