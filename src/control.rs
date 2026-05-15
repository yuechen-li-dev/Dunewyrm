#![allow(non_snake_case)]

use crate::DwPhase;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DwControl {
    Continue { Pc: u32 },
    WaitTicks { Ticks: u32, Pc: u32 },
    Stay,
    Complete,
    Fail { Reason: &'static str },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DwControlSummary {
    Continue,
    WaitTicks { Ticks: u32 },
    Stay,
    Complete,
    Fail,
}

pub mod Dw {
    use super::{DwControl, DwPhase};

    pub fn Continue<P: DwPhase>(phase: P) -> DwControl {
        DwControl::Continue { Pc: phase.ToPc() }
    }

    pub fn WaitTicks<P: DwPhase>(ticks: u32, phase: P) -> DwControl {
        DwControl::WaitTicks {
            Ticks: ticks,
            Pc: phase.ToPc(),
        }
    }

    pub fn Stay() -> DwControl {
        DwControl::Stay
    }

    pub fn Complete() -> DwControl {
        DwControl::Complete
    }

    pub fn Fail(reason: &'static str) -> DwControl {
        DwControl::Fail { Reason: reason }
    }
}
