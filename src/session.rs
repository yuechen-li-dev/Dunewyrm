#![allow(non_snake_case)]

use crate::{DwControl, DwControlSummary, DwFrameId, DwFrameRegistry, DwPhase};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DwRunStatus {
    Running,
    Waiting,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DwTickResult {
    pub Tick: u64,
    pub Status: DwRunStatus,
    pub Frame: DwFrameId,
    pub Pc: u32,
    pub Control: Option<DwControlSummary>,
    pub FailureReason: Option<&'static str>,
}

pub struct DwFrameCtx {
    Frame: DwFrameId,
    Pc: u32,
    Tick: u64,
}

impl DwFrameCtx {
    pub fn New(frame: DwFrameId, pc: u32, tick: u64) -> Self {
        Self {
            Frame: frame,
            Pc: pc,
            Tick: tick,
        }
    }

    pub fn Frame(&self) -> DwFrameId {
        self.Frame
    }

    pub fn Pc(&self) -> u32 {
        self.Pc
    }

    pub fn Phase<P: DwPhase>(&self) -> Option<P> {
        P::FromPc(self.Pc)
    }

    pub fn Tick(&self) -> u64 {
        self.Tick
    }
}

pub struct DwSession {
    Registry: DwFrameRegistry,
    ActiveFrame: DwFrameId,
    Pc: u32,
    Tick: u64,
    WaitRemaining: u32,
    WaitResumePc: Option<u32>,
    Status: DwRunStatus,
    FailureReason: Option<&'static str>,
}

impl DwSession {
    pub fn New(
        registry: DwFrameRegistry,
        root: DwFrameId,
        initial_pc: u32,
    ) -> Result<Self, &'static str> {
        if registry.Find(root).is_none() {
            return Err("root frame not found");
        }

        Ok(Self {
            Registry: registry,
            ActiveFrame: root,
            Pc: initial_pc,
            Tick: 0,
            WaitRemaining: 0,
            WaitResumePc: None,
            Status: DwRunStatus::Running,
            FailureReason: None,
        })
    }

    pub fn Tick(&mut self) -> Result<DwTickResult, &'static str> {
        let tick_now = self.Tick;

        if self.Status == DwRunStatus::Completed || self.Status == DwRunStatus::Failed {
            let result = self.BuildResult(tick_now, None);
            self.Tick += 1;
            return Ok(result);
        }

        if self.WaitRemaining > 0 {
            self.WaitRemaining -= 1;
            if self.WaitRemaining == 0 {
                self.Pc = self.WaitResumePc.expect("wait resume pc should be set");
                self.WaitResumePc = None;
                let result =
                    self.BuildResult(tick_now, Some(DwControlSummary::WaitTicks { Ticks: 0 }));
                self.Tick += 1;
                return Ok(result);
            }

            let result = self.BuildResult(
                tick_now,
                Some(DwControlSummary::WaitTicks {
                    Ticks: self.WaitRemaining,
                }),
            );
            self.Tick += 1;
            return Ok(result);
        }

        let frame = self
            .Registry
            .Find(self.ActiveFrame)
            .ok_or("active frame missing")?;
        let mut ctx = DwFrameCtx::New(self.ActiveFrame, self.Pc, tick_now);
        let control = (frame.Step)(&mut ctx);
        self.ApplyControl(control);

        let result = self.BuildResult(tick_now, Some(Self::Summarize(control)));
        self.Tick += 1;
        Ok(result)
    }

    fn ApplyControl(&mut self, control: DwControl) {
        match control {
            DwControl::Continue { Pc } => {
                self.Pc = Pc;
            }
            DwControl::WaitTicks { Ticks, Pc } => {
                if Ticks == 0 {
                    self.Pc = Pc;
                } else {
                    self.WaitRemaining = Ticks;
                    self.WaitResumePc = Some(Pc);
                    self.Status = DwRunStatus::Waiting;
                }
            }
            DwControl::Stay => {}
            DwControl::Complete => {
                self.Status = DwRunStatus::Completed;
            }
            DwControl::Fail { Reason } => {
                self.Status = DwRunStatus::Failed;
                self.FailureReason = Some(Reason);
            }
        }
    }

    fn BuildResult(&mut self, tick_now: u64, control: Option<DwControlSummary>) -> DwTickResult {
        if self.Status == DwRunStatus::Waiting && self.WaitRemaining == 0 {
            self.Status = DwRunStatus::Running;
        }

        DwTickResult {
            Tick: tick_now,
            Status: self.Status,
            Frame: self.ActiveFrame,
            Pc: self.Pc,
            Control: control,
            FailureReason: self.FailureReason,
        }
    }

    fn Summarize(control: DwControl) -> DwControlSummary {
        match control {
            DwControl::Continue { .. } => DwControlSummary::Continue,
            DwControl::WaitTicks { Ticks, .. } => DwControlSummary::WaitTicks { Ticks },
            DwControl::Stay => DwControlSummary::Stay,
            DwControl::Complete => DwControlSummary::Complete,
            DwControl::Fail { .. } => DwControlSummary::Fail,
        }
    }
}
