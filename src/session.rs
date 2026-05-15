#![allow(non_snake_case)]

use std::fmt::Write;

use crate::{
    DwBoard, DwBoardChunk, DwControl, DwControlSummary, DwFrameId, DwFrameRegistry, DwMailbox,
    DwMailboxChunk, DwMessage, DwPhase,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DwTickTraceEntry {
    pub Tick: u64,
    pub Status: DwRunStatus,
    pub Frame: Option<DwFrameId>,
    pub Pc: Option<u32>,
    pub Stack: Vec<DwRuntimeFrame>,
    pub Control: Option<DwControlSummary>,
    pub DirtySlots: Vec<u32>,
    pub VisibleMailbox: Vec<DwMessage>,
    pub StagedMailbox: Vec<DwMessage>,
    pub FailureReason: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DwTraceComparison {
    pub Matches: bool,
    pub FirstMismatchIndex: Option<usize>,
    pub Reason: Option<String>,
    pub Expected: Option<DwTickTraceEntry>,
    pub Actual: Option<DwTickTraceEntry>,
}

pub fn CompareTrace(
    expected: &[DwTickTraceEntry],
    actual: &[DwTickTraceEntry],
) -> DwTraceComparison {
    let shared_len = expected.len().min(actual.len());
    for index in 0..shared_len {
        if expected[index] != actual[index] {
            return DwTraceComparison {
                Matches: false,
                FirstMismatchIndex: Some(index),
                Reason: Some(format!("trace entry mismatch at tick index {index}")),
                Expected: Some(expected[index].clone()),
                Actual: Some(actual[index].clone()),
            };
        }
    }

    if expected.len() != actual.len() {
        return DwTraceComparison {
            Matches: false,
            FirstMismatchIndex: Some(shared_len),
            Reason: Some(format!(
                "trace length mismatch expected={} actual={}",
                expected.len(),
                actual.len()
            )),
            Expected: expected.get(shared_len).cloned(),
            Actual: actual.get(shared_len).cloned(),
        };
    }

    DwTraceComparison {
        Matches: true,
        FirstMismatchIndex: None,
        Reason: None,
        Expected: None,
        Actual: None,
    }
}

pub fn FormatFrameId(frame: DwFrameId) -> String {
    format!("{}:{}", frame.Domain, frame.Local)
}

pub fn FormatTraceEntry(entry: &DwTickTraceEntry) -> String {
    let mut output = String::new();
    let _ = write!(
        output,
        "tick={} status={:?} frame={} pc={} control={:?} dirty={:?}",
        entry.Tick,
        entry.Status,
        entry
            .Frame
            .map(FormatFrameId)
            .unwrap_or_else(|| "none".to_string()),
        entry
            .Pc
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string()),
        entry.Control,
        entry.DirtySlots
    );

    let stack = entry
        .Stack
        .iter()
        .map(|frame| format!("{}@{}", FormatFrameId(frame.Id), frame.Pc))
        .collect::<Vec<_>>()
        .join(" -> ");
    let _ = write!(output, " stack=[{}]", stack);
    let _ = write!(
        output,
        " visible={:?} staged={:?}",
        entry.VisibleMailbox, entry.StagedMailbox
    );

    if let Some(reason) = entry.FailureReason {
        let _ = write!(output, " failure={reason}");
    }

    output
}

pub fn FormatTrace(trace: &[DwTickTraceEntry]) -> String {
    trace
        .iter()
        .map(FormatTraceEntry)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn FormatComparison(comparison: &DwTraceComparison) -> String {
    if comparison.Matches {
        return "trace comparison matches".to_string();
    }

    let mut output = String::new();
    let _ = write!(
        output,
        "trace comparison mismatch index={:?} reason={}",
        comparison.FirstMismatchIndex,
        comparison.Reason.clone().unwrap_or_default()
    );
    if let Some(expected) = &comparison.Expected {
        let _ = write!(output, "\nexpected: {}", FormatTraceEntry(expected));
    }
    if let Some(actual) = &comparison.Actual {
        let _ = write!(output, "\nactual: {}", FormatTraceEntry(actual));
    }
    output
}

pub struct DwFrameCtx<'a> {
    /* unchanged */
    Frame: DwFrameId,
    Pc: u32,
    Tick: u64,
    Board: &'a mut DwBoard,
    Mailbox: &'a mut DwMailbox,
}
impl<'a> DwFrameCtx<'a> {
    /* methods */
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
    Trace: Vec<DwTickTraceEntry>,
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
            Trace: Vec::new(),
        })
    }
    pub fn Tick(&mut self) -> Result<DwTickResult, &'static str> {
        let tick_now = self.Tick;
        let result = if self.Status == DwRunStatus::Completed || self.Status == DwRunStatus::Failed
        {
            self.BuildResult(tick_now, None)
        } else {
            self.Mailbox.BeginTick();
            if self.WaitRemaining > 0 {
                self.WaitRemaining -= 1;
                if self.WaitRemaining == 0 {
                    if let Some(top) = self.Stack.last_mut() {
                        top.Pc = self.WaitResumePc.expect("wait resume pc should be set");
                    }
                    self.WaitResumePc = None;
                    self.BuildResult(tick_now, Some(DwControlSummary::WaitTicks { Ticks: 0 }))
                } else {
                    self.BuildResult(
                        tick_now,
                        Some(DwControlSummary::WaitTicks {
                            Ticks: self.WaitRemaining,
                        }),
                    )
                }
            } else {
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
                self.BuildResult(tick_now, Some(Self::Summarize(control)))
            }
        };
        self.Trace.push(Self::TraceFromResult(&result));
        self.Tick += 1;
        Ok(result)
    }
    fn TraceFromResult(result: &DwTickResult) -> DwTickTraceEntry {
        DwTickTraceEntry {
            Tick: result.Tick,
            Status: result.Status,
            Frame: result.Frame,
            Pc: result.Pc,
            Stack: result.Stack.iter().flatten().copied().collect(),
            Control: result.Control,
            DirtySlots: result.DirtySlots.clone(),
            VisibleMailbox: result.VisibleMailbox.clone(),
            StagedMailbox: result.StagedMailbox.clone(),
            FailureReason: result.FailureReason,
        }
    }
    pub fn Trace(&self) -> &[DwTickTraceEntry] {
        &self.Trace
    }
    pub fn Board(&self) -> &DwBoard {
        &self.Board
    }
    pub fn Mailbox(&self) -> &DwMailbox {
        &self.Mailbox
    }
    pub fn MailboxMut(&mut self) -> &mut DwMailbox {
        &mut self.Mailbox
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
            Trace: Vec::new(),
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
