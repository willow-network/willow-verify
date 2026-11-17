//! The npm client's suite (web/client/test), ported, on the same recorded fixtures: items and
//! state verified by the real verifiers, anchors by the mock anchor (the mock server can't sign
//! for Ethereum's validators) except in `anchor_real`, which runs the real light clients on the
//! saved anchors.

#![allow(clippy::type_complexity)]

mod support;

mod anchor_real;
mod blocks;
mod fees;
mod filters;
mod held;
mod items;
mod live;
mod logs;
mod lying;
mod modes;
mod policy;
mod regressions;
mod review2;
mod state;
mod trace_real;
mod traces;
mod transport;
mod tx;
