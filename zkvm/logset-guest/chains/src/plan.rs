//! **Fast-chain segment and publish planner.** Pure and deterministic.
//!
//! Three layers, all aligned to ABSOLUTE block numbers:
//!
//! ```text
//!   leaf segments  [kS, (k+1)S)          proved at the tip, once, by the frozen leaf
//!   L1 index aggs  [jQ, (j+1)Q)          Q/S segments each, the sorted index scope
//!   statement      [Q-aligned s, A]      folded (L2) and published; A = an ANCHOR block
//! ```
//!
//! An L2 chain's block hash is provable on Ethereum only at its anchors: a BoLD
//! assertion's block (Orbit) or a valid dispute game's block (OP stack). So a
//! published statement must END on an anchor block `A`, which is arbitrary, while
//! the tip proves fixed aligned segments without knowing where the next anchor
//! will fall. The fold's alignment rule (every child spans exactly `2^k` blocks
//! except the last) is satisfied by:
//!
//! - starting every statement on a `Q` boundary, so its full children are the
//!   tip's own segments and L1 aggregates, reused as proved;
//! - ending it at `A` with ONE partial segment `[⌊A/S⌋·S, A]` and ONE partial L1
//!   `[⌊A/Q⌋·Q, A]`, proved for this statement only (the "tail");
//! - starting the NEXT statement at `⌊(A+1)/Q⌋·Q`, i.e. re-covering the partial
//!   scope in full. Consecutive statements therefore overlap by fewer than `Q`
//!   blocks; the registry keys statements by hash, so an overlap is harmless and
//!   coverage never has a gap.
//!
//! The extra work per publish is one partial segment plus one partial L1, against
//! days of chain: noise.

/// An inclusive block range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

impl Range {
    pub fn len(&self) -> u64 {
        self.end - self.start + 1
    }
    pub fn is_empty(&self) -> bool {
        false
    }
}

pub fn align_down(block: u64, log2: u8) -> u64 {
    (block >> log2) << log2
}

/// The aligned `2^log2`-block pieces covering `[start, end]`; `start` must be
/// aligned. Every piece is full except possibly the last.
pub fn pieces(start: u64, end: u64, log2: u8) -> Vec<Range> {
    assert_eq!(align_down(start, log2), start, "start must be aligned");
    assert!(end >= start);
    let w = 1u64 << log2;
    let mut out = Vec::new();
    let mut s = start;
    while s <= end {
        let e = (s + w - 1).min(end);
        out.push(Range { start: s, end: e });
        s = match e.checked_add(1) {
            Some(x) => x,
            None => break,
        };
    }
    out
}

/// One anchored statement and the work it needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredStatement {
    /// `[s, A]`, `s` Q-aligned, `A` the anchor block.
    pub range: Range,
    /// The L1 index aggregates the fold takes, in order. All are full `Q`
    /// scopes except possibly the last.
    pub l1: Vec<Range>,
    /// The partial last leaf segment `[⌊A/S⌋·S, A]`, if `A` does not close a
    /// segment. It is NOT a tip segment: it is proved for this statement.
    pub tail_segment: Option<Range>,
    /// The partial last L1 `[⌊A/Q⌋·Q, A]`, if `A` does not close a scope.
    pub tail_l1: Option<Range>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The anchor lies before the next statement's start (already covered).
    StaleAnchor { anchor: u64, next_start: u64 },
    /// `lane_start` is not Q-aligned.
    UnalignedStart { start: u64, index_log2: u8 },
    /// `S > Q` or `Q/S > 64`.
    Geometry { seg_log2: u8, index_log2: u8 },
}

/// Statement planner for one chain's on-chain lane.
#[derive(Clone, Debug)]
pub struct Planner {
    seg_log2: u8,
    index_log2: u8,
    next_start: u64,
}

impl Planner {
    pub fn new(seg_log2: u8, index_log2: u8, lane_start: u64) -> Result<Self, PlanError> {
        if seg_log2 > index_log2 || index_log2 - seg_log2 > crate::MAX_L1_CHILDREN_LOG2 {
            return Err(PlanError::Geometry {
                seg_log2,
                index_log2,
            });
        }
        if align_down(lane_start, index_log2) != lane_start {
            return Err(PlanError::UnalignedStart {
                start: lane_start,
                index_log2,
            });
        }
        Ok(Self {
            seg_log2,
            index_log2,
            next_start: lane_start,
        })
    }

    pub fn next_start(&self) -> u64 {
        self.next_start
    }

    /// The statement ending at `anchor`, and advance past it.
    pub fn statement_to(&mut self, anchor: u64) -> Result<AnchoredStatement, PlanError> {
        if anchor < self.next_start {
            return Err(PlanError::StaleAnchor {
                anchor,
                next_start: self.next_start,
            });
        }
        let s = self.next_start;
        let l1 = pieces(s, anchor, self.index_log2);
        let last = *l1.last().expect("non-empty");
        let tail_l1 = (last.len() < 1u64 << self.index_log2).then_some(last);
        let seg_start = align_down(anchor, self.seg_log2);
        let tail_segment = (anchor - seg_start + 1 < 1u64 << self.seg_log2).then_some(Range {
            start: seg_start,
            end: anchor,
        });
        self.next_start = align_down(anchor + 1, self.index_log2);
        Ok(AnchoredStatement {
            range: Range {
                start: s,
                end: anchor,
            },
            l1,
            tail_segment,
            tail_l1,
        })
    }
}

/// An anchorable L2 block: its number and its header timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorPoint {
    pub block: u64,
    pub timestamp: u64,
}

/// Daily cadence: one statement per UTC day, ending on that day's LAST anchor
/// (by the anchor block's own timestamp). The anchor already lags the chain by
/// days, so one more day of batching costs nothing a consumer can see and
/// amortizes the fold, the Groth16 wrap and the gas over the whole day.
#[derive(Clone, Debug, Default)]
pub struct DailyAnchors {
    held: Option<AnchorPoint>,
}

pub const DAY_SECS: u64 = 86_400;

impl DailyAnchors {
    /// Feed anchors in increasing block order (duplicates and regressions are
    /// ignored). Returns the anchors that close a day, in order.
    pub fn push(&mut self, points: &[AnchorPoint]) -> Vec<AnchorPoint> {
        let mut out = Vec::new();
        for p in points {
            match self.held {
                Some(h) if p.block <= h.block => continue,
                Some(h) if p.timestamp / DAY_SECS > h.timestamp / DAY_SECS => {
                    out.push(h);
                    self.held = Some(*p);
                }
                _ => self.held = Some(*p),
            }
        }
        out
    }

    /// The anchor that would close the current day, if one has been seen.
    pub fn pending(&self) -> Option<AnchorPoint> {
        self.held
    }
}
