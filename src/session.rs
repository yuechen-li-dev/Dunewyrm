#![allow(non_snake_case)]

use crate::{
    DwBoard, DwBoardChunk, DwControl, DwControlSummary, DwFrameId, DwFrameRegistry, DwMailbox,
    DwMailboxChunk, DwMessage, DwPhase, DwSlotCollision,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DwRunStatus {
    Running,
    Waiting,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DwRuntimeFrame {
    pub Id: DwFrameId,
    pub Pc: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DwTickResult {
    pub DirtySlots: Vec<u32>,
    pub VisibleMailbox: Vec<DwMessage>,
    pub StagedMailbox: Vec<DwMessage>,
    pub Tick: u64,
    pub Status: DwRunStatus,
    pub Frame: Option<DwFrameId>,
    pub Pc: Option<u32>,
    pub Stack: [Option<DwRuntimeFrame>; 8],
    pub StackDepth: usize,
    pub Control: Option<DwControlSummary>,
    pub FailureReason: Option<&'static str>,
}

pub struct DwFrameCtx<'a> {
    Frame: DwFrameId,
    Pc: u32,
    Tick: u64,
    Board: &'a mut DwBoard,
    Mailbox: &'a mut DwMailbox,
}

impl<'a> DwFrameCtx<'a> {
    pub fn New(
        frame: DwFrameId,
        pc: u32,
        tick: u64,
        board: &'a mut DwBoard,
        mailbox: &'a mut DwMailbox,
    ) -> Self {
        Self {
            Frame: frame,
            Pc: pc,
            Tick: tick,
            Board: board,
            Mailbox: mailbox,
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
    pub fn Board(&self) -> &DwBoard {
        self.Board
    }
    pub fn BoardMut(&mut self) -> &mut DwBoard {
        self.Board
    }
    pub fn Mailbox(&self) -> &DwMailbox {
        self.Mailbox
    }
    pub fn MailboxMut(&mut self) -> &mut DwMailbox {
        self.Mailbox
    }
}

pub struct DwSession {
    Registry: DwFrameRegistry,
    Stack: Vec<DwRuntimeFrame>,
    Tick: u64,
    WaitRemaining: u32,
    WaitResumePc: Option<u32>,
    Status: DwRunStatus,
    FailureReason: Option<&'static str>,
    Board: DwBoard,
    Mailbox: DwMailbox,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DwWaitChunk {
    pub WaitRemaining: u32,
    pub WaitResumePc: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DwRuntimeChunk {
    pub Tick: u64,
    pub Status: DwRunStatus,
    pub FailureReason: Option<&'static str>,
    pub Wait: DwWaitChunk,
    pub Stack: Vec<DwRuntimeFrame>,
    pub Board: DwBoardChunk,
    pub Mailbox: DwMailboxChunk,
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
            Stack: vec![DwRuntimeFrame {
                Id: root,
                Pc: initial_pc,
            }],
            Tick: 0,
            WaitRemaining: 0,
            WaitResumePc: None,
            Status: DwRunStatus::Running,
            FailureReason: None,
            Board: DwBoard::New(),
            Mailbox: DwMailbox::New(),
        })
    }

    pub fn Tick(&mut self) -> Result<DwTickResult, &'static str> {
        let tick_now = self.Tick;
        if self.Status == DwRunStatus::Completed || self.Status == DwRunStatus::Failed {
            let r = self.BuildResult(tick_now, None);
            self.Tick += 1;
            return Ok(r);
        }

        self.Mailbox.BeginTick();

        if self.WaitRemaining > 0 {
            self.WaitRemaining -= 1;
            if self.WaitRemaining == 0 {
                if let Some(top) = self.Stack.last_mut() {
                    top.Pc = self.WaitResumePc.expect("wait resume pc should be set");
                }
                self.WaitResumePc = None;
                let r = self.BuildResult(tick_now, Some(DwControlSummary::WaitTicks { Ticks: 0 }));
                self.Tick += 1;
                return Ok(r);
            }
            let r = self.BuildResult(
                tick_now,
                Some(DwControlSummary::WaitTicks {
                    Ticks: self.WaitRemaining,
                }),
            );
            self.Tick += 1;
            return Ok(r);
        }

        self.Board.ClearDirty();

        let active = self.Stack.last().copied().ok_or("runtime stack empty")?;
        let frame = self
            .Registry
            .Find(active.Id)
            .ok_or("active frame missing")?;
        let mut ctx = DwFrameCtx::New(
            active.Id,
            active.Pc,
            tick_now,
            &mut self.Board,
            &mut self.Mailbox,
        );
        let control = (frame.Step)(&mut ctx);
        self.ApplyControl(control);
        let r = self.BuildResult(tick_now, Some(Self::Summarize(control)));
        self.Tick += 1;
        Ok(r)
    }

    pub fn ExportChunk(&self) -> DwRuntimeChunk {
        DwRuntimeChunk {
            Tick: self.Tick,
            Status: self.Status,
            FailureReason: self.FailureReason,
            Wait: DwWaitChunk {
                WaitRemaining: self.WaitRemaining,
                WaitResumePc: self.WaitResumePc,
            },
            Stack: self.Stack.clone(),
            Board: self.Board.ExportChunk(),
            Mailbox: self.Mailbox.ExportChunk(),
        }
    }

    pub fn FromChunk(
        registry: DwFrameRegistry,
        chunk: DwRuntimeChunk,
    ) -> Result<Self, &'static str> {
        for frame in &chunk.Stack {
            if registry.Find(frame.Id).is_none() {
                return Err("chunk stack frame not found in registry");
            }
        }

        Ok(Self {
            Registry: registry,
            Stack: chunk.Stack,
            Tick: chunk.Tick,
            WaitRemaining: chunk.Wait.WaitRemaining,
            WaitResumePc: chunk.Wait.WaitResumePc,
            Status: chunk.Status,
            FailureReason: chunk.FailureReason,
            Board: DwBoard::FromChunk(chunk.Board),
            Mailbox: DwMailbox::FromChunk(chunk.Mailbox),
        })
    }

    fn ApplyControl(&mut self, control: DwControl) {
        match control {
            DwControl::Continue { Pc } => {
                if let Some(top) = self.Stack.last_mut() {
                    top.Pc = Pc;
                }
            }
            DwControl::WaitTicks { Ticks, Pc } => {
                if Ticks == 0 {
                    if let Some(top) = self.Stack.last_mut() {
                        top.Pc = Pc;
                    }
                } else {
                    self.WaitRemaining = Ticks;
                    self.WaitResumePc = Some(Pc);
                    self.Status = DwRunStatus::Waiting;
                }
            }
            DwControl::Push { Target, ResumePc } => {
                if self.Registry.Find(Target).is_none() {
                    self.FailNow("push target frame not found");
                    return;
                }
                if let Some(top) = self.Stack.last_mut() {
                    top.Pc = ResumePc;
                }
                self.Stack.push(DwRuntimeFrame { Id: Target, Pc: 0 });
            }
            DwControl::Pop => {
                if self.Stack.len() == 1 {
                    self.FailNow("cannot pop root frame");
                    return;
                }
                self.Stack.pop();
            }
            DwControl::Replace { Target } => {
                if self.Registry.Find(Target).is_none() {
                    self.FailNow("replace target frame not found");
                    return;
                }
                self.Stack.pop();
                self.Stack.push(DwRuntimeFrame { Id: Target, Pc: 0 });
            }
            DwControl::Stay => {}
            DwControl::Complete => {
                if self.Stack.len() == 1 {
                    self.Status = DwRunStatus::Completed;
                } else {
                    self.Stack.pop();
                }
            }
            DwControl::Fail { Reason } => self.FailNow(Reason),
        }
    }

    fn FailNow(&mut self, reason: &'static str) {
        self.Status = DwRunStatus::Failed;
        self.FailureReason = Some(reason);
    }

    fn BuildResult(&mut self, tick_now: u64, control: Option<DwControlSummary>) -> DwTickResult {
        if self.Status == DwRunStatus::Waiting && self.WaitRemaining == 0 {
            self.Status = DwRunStatus::Running;
        }
        let mut snap = [None; 8];
        for (i, frame) in self.Stack.iter().take(8).enumerate() {
            snap[i] = Some(*frame);
        }
        let active = self.Stack.last().copied();
        DwTickResult {
            Tick: tick_now,
            Status: self.Status,
            Frame: active.map(|f| f.Id),
            Pc: active.map(|f| f.Pc),
            Stack: snap,
            StackDepth: self.Stack.len(),
            Control: control,
            FailureReason: self.FailureReason,
            DirtySlots: self.Board.DirtySlots(),
            VisibleMailbox: self.Mailbox.VisibleSnapshot(),
            StagedMailbox: self.Mailbox.StagedSnapshot(),
        }
    }

    fn Summarize(control: DwControl) -> DwControlSummary {
        match control {
            DwControl::Continue { .. } => DwControlSummary::Continue,
            DwControl::WaitTicks { Ticks, .. } => DwControlSummary::WaitTicks { Ticks },
            DwControl::Push { .. } => DwControlSummary::Push,
            DwControl::Pop => DwControlSummary::Pop,
            DwControl::Replace { .. } => DwControlSummary::Replace,
            DwControl::Stay => DwControlSummary::Stay,
            DwControl::Complete => DwControlSummary::Complete,
            DwControl::Fail { .. } => DwControlSummary::Fail,
        }
    }
}

impl DwSession {
    pub fn Board(&self) -> &DwBoard {
        &self.Board
    }

    pub fn BoardMut(&mut self) -> &mut DwBoard {
        &mut self.Board
    }

    pub fn LastBoardSlotCollision(&self) -> Option<DwSlotCollision> {
        self.Board.LastSlotCollision()
    }

    pub fn Mailbox(&self) -> &DwMailbox {
        &self.Mailbox
    }

    pub fn MailboxMut(&mut self) -> &mut DwMailbox {
        &mut self.Mailbox
    }
}
