#![allow(non_snake_case)]

mod control;
mod ids;
mod phase;
mod registry;
mod session;

pub use control::{Dw, DwControl, DwControlSummary};
pub use ids::DwFrameId;
pub use phase::DwPhase;
pub use registry::{DwFrameDef, DwFrameFn, DwFrameRegistry};
pub use session::{DwFrameCtx, DwRunStatus, DwRuntimeFrame, DwSession, DwTickResult};

pub fn ProjectName() -> &'static str {
    "Dunewyrm"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum RootPhase {
        Start,
        Wait,
        Done,
    }
    impl DwPhase for RootPhase {
        fn ToPc(self) -> u32 {
            match self {
                RootPhase::Start => 0,
                RootPhase::Wait => 1,
                RootPhase::Done => 2,
            }
        }
        fn FromPc(pc: u32) -> Option<Self> {
            match pc {
                0 => Some(RootPhase::Start),
                1 => Some(RootPhase::Wait),
                2 => Some(RootPhase::Done),
                _ => None,
            }
        }
    }
    fn Root(ctx: &mut DwFrameCtx) -> DwControl {
        match ctx.Phase::<RootPhase>() {
            Some(RootPhase::Start) => Dw::Continue(RootPhase::Wait),
            Some(RootPhase::Wait) => Dw::WaitTicks(1, RootPhase::Done),
            Some(RootPhase::Done) => Dw::Complete(),
            None => Dw::Fail("bad phase"),
        }
    }

    #[test]
    fn ProjectNameMatches() {
        assert_eq!(ProjectName(), "Dunewyrm");
    }

    #[test]
    fn StackPushPopAndChildCompleteWorkDeterministically() {
        #[derive(Clone, Copy)]
        enum R {
            Start,
            After,
        }
        impl DwPhase for R {
            fn ToPc(self) -> u32 {
                match self {
                    R::Start => 0,
                    R::After => 1,
                }
            }
            fn FromPc(pc: u32) -> Option<Self> {
                match pc {
                    0 => Some(R::Start),
                    1 => Some(R::After),
                    _ => None,
                }
            }
        }
        #[derive(Clone, Copy)]
        enum C {
            Begin,
        }
        impl DwPhase for C {
            fn ToPc(self) -> u32 {
                0
            }
            fn FromPc(pc: u32) -> Option<Self> {
                if pc == 0 { Some(C::Begin) } else { None }
            }
        }
        let root_id = DwFrameId {
            Domain: 1,
            Local: 1,
        };
        let child_id = DwFrameId {
            Domain: 1,
            Local: 2,
        };
        fn RootF(ctx: &mut DwFrameCtx) -> DwControl {
            match ctx.Phase::<R>() {
                Some(R::Start) => Dw::Push(
                    DwFrameId {
                        Domain: 1,
                        Local: 2,
                    },
                    R::After,
                ),
                Some(R::After) => Dw::Complete(),
                None => Dw::Fail("root phase"),
            }
        }
        fn ChildF(_: &mut DwFrameCtx) -> DwControl {
            Dw::Complete()
        }
        let mut reg = DwFrameRegistry::New();
        reg.Register(DwFrameDef {
            Id: root_id,
            Step: RootF,
            DebugName: "Root",
        })
        .unwrap();
        reg.Register(DwFrameDef {
            Id: child_id,
            Step: ChildF,
            DebugName: "Child",
        })
        .unwrap();
        let mut s = DwSession::New(reg, root_id, 0).unwrap();
        let t0 = s.Tick().unwrap();
        assert_eq!(t0.Control, Some(DwControlSummary::Push));
        assert_eq!(t0.Frame, Some(child_id));
        assert_eq!(t0.Pc, Some(0));
        assert_eq!(t0.StackDepth, 2);
        let t1 = s.Tick().unwrap();
        assert_eq!(t1.Control, Some(DwControlSummary::Complete));
        assert_eq!(t1.Frame, Some(root_id));
        assert_eq!(t1.Pc, Some(1));
        assert_eq!(t1.StackDepth, 1);
        let t2 = s.Tick().unwrap();
        assert_eq!(t2.Status, DwRunStatus::Completed);
    }

    #[test]
    fn WaitInChildBlocksParentAndResumesChild() {
        #[derive(Clone, Copy)]
        enum R {
            Start,
            After,
        }
        impl DwPhase for R {
            fn ToPc(self) -> u32 {
                match self {
                    R::Start => 0,
                    R::After => 1,
                }
            }
            fn FromPc(pc: u32) -> Option<Self> {
                match pc {
                    0 => Some(R::Start),
                    1 => Some(R::After),
                    _ => None,
                }
            }
        }
        #[derive(Clone, Copy)]
        enum C {
            Start,
            Done,
        }
        impl DwPhase for C {
            fn ToPc(self) -> u32 {
                match self {
                    C::Start => 0,
                    C::Done => 1,
                }
            }
            fn FromPc(pc: u32) -> Option<Self> {
                match pc {
                    0 => Some(C::Start),
                    1 => Some(C::Done),
                    _ => None,
                }
            }
        }
        let root_id = DwFrameId {
            Domain: 2,
            Local: 1,
        };
        let child_id = DwFrameId {
            Domain: 2,
            Local: 2,
        };
        fn RootF(ctx: &mut DwFrameCtx) -> DwControl {
            match ctx.Phase::<R>() {
                Some(R::Start) => Dw::Push(
                    DwFrameId {
                        Domain: 2,
                        Local: 2,
                    },
                    R::After,
                ),
                Some(R::After) => Dw::Complete(),
                None => Dw::Fail("root"),
            }
        }
        fn ChildF(ctx: &mut DwFrameCtx) -> DwControl {
            match ctx.Phase::<C>() {
                Some(C::Start) => Dw::WaitTicks(2, C::Done),
                Some(C::Done) => Dw::Pop(),
                None => Dw::Fail("child"),
            }
        }
        let mut reg = DwFrameRegistry::New();
        reg.Register(DwFrameDef {
            Id: root_id,
            Step: RootF,
            DebugName: "Root",
        })
        .unwrap();
        reg.Register(DwFrameDef {
            Id: child_id,
            Step: ChildF,
            DebugName: "Child",
        })
        .unwrap();
        let mut s = DwSession::New(reg, root_id, 0).unwrap();
        s.Tick().unwrap();
        let t1 = s.Tick().unwrap();
        assert_eq!(t1.Status, DwRunStatus::Waiting);
        assert_eq!(t1.Frame, Some(child_id));
        let t2 = s.Tick().unwrap();
        assert_eq!(t2.Status, DwRunStatus::Waiting);
        assert_eq!(t2.Frame, Some(child_id));
        let t3 = s.Tick().unwrap();
        assert_eq!(t3.Status, DwRunStatus::Running);
        assert_eq!(t3.Frame, Some(child_id));
        assert_eq!(t3.Pc, Some(1));
    }

    #[test]
    fn ReplaceAndFailureCasesCovered() {
        let root = DwFrameId {
            Domain: 3,
            Local: 1,
        };
        let repl = DwFrameId {
            Domain: 3,
            Local: 2,
        };
        fn RootF(_: &mut DwFrameCtx) -> DwControl {
            Dw::Replace(DwFrameId {
                Domain: 3,
                Local: 2,
            })
        }
        fn ReplF(_: &mut DwFrameCtx) -> DwControl {
            Dw::Complete()
        }
        let mut reg = DwFrameRegistry::New();
        reg.Register(DwFrameDef {
            Id: root,
            Step: RootF,
            DebugName: "Root",
        })
        .unwrap();
        reg.Register(DwFrameDef {
            Id: repl,
            Step: ReplF,
            DebugName: "Repl",
        })
        .unwrap();
        let mut s = DwSession::New(reg, root, 0).unwrap();
        let t0 = s.Tick().unwrap();
        assert_eq!(t0.Control, Some(DwControlSummary::Replace));
        assert_eq!(t0.Frame, Some(repl));

        let mut reg2 = DwFrameRegistry::New();
        reg2.Register(DwFrameDef {
            Id: root,
            Step: |_| Dw::Pop(),
            DebugName: "RootPop",
        })
        .unwrap();
        let mut s2 = DwSession::New(reg2, root, 0).unwrap();
        let f = s2.Tick().unwrap();
        assert_eq!(f.Status, DwRunStatus::Failed);
    }
}
