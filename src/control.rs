#![allow(non_snake_case)]

use crate::{DwFrameId, DwPhase};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DwControl {
    Continue { Pc: u32 },
    WaitTicks { Ticks: u32, Pc: u32 },
    Push { Target: DwFrameId, ResumePc: u32 },
    Pop,
    Replace { Target: DwFrameId },
    Stay,
    Complete,
    Fail { Reason: &'static str },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DwControlSummary {
    Continue,
    WaitTicks { Ticks: u32 },
    Push,
    Pop,
    Replace,
    Stay,
    Complete,
    Fail,
}

pub mod Dw {
    use super::{DwControl, DwFrameId, DwPhase};

    pub fn Continue<P: DwPhase>(phase: P) -> DwControl {
        DwControl::Continue { Pc: phase.ToPc() }
    }

    pub fn WaitTicks<P: DwPhase>(ticks: u32, phase: P) -> DwControl {
        DwControl::WaitTicks {
            Ticks: ticks,
            Pc: phase.ToPc(),
        }
    }

    pub fn Push<P: DwPhase>(target: DwFrameId, resume_phase: P) -> DwControl {
        DwControl::Push {
            Target: target,
            ResumePc: resume_phase.ToPc(),
        }
    }

    pub fn Pop() -> DwControl {
        DwControl::Pop
    }

    pub fn Replace(target: DwFrameId) -> DwControl {
        DwControl::Replace { Target: target }
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
