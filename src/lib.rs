#![allow(non_snake_case)]

mod board;
mod control;
mod ids;
mod mailbox;
mod phase;
mod registry;
mod session;

pub use board::{DwBoard, DwBoardKind, DwBoardValue, DwKey, DwSlotCollision};
pub use control::{Dw, DwControl, DwControlSummary};
pub use ids::DwFrameId;
pub use mailbox::{DwMailbox, DwMessage};
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

    mod Keys {
        use crate::DwKey;
        pub const Alerted: DwKey<bool> = DwKey::New("Alerted", 1);
        pub const Count: DwKey<i32> = DwKey::New("Count", 2);
        pub const Pressure: DwKey<f32> = DwKey::New("Pressure", 3);
    }

    #[test]
    fn BoardSetGetDirtyAndCollisionBehavior() {
        let mut board = DwBoard::New();
        assert_eq!(board.GetOr(Keys::Alerted, false), false);
        assert_eq!(board.GetOr(Keys::Count, -1), -1);
        assert!((board.GetOr(Keys::Pressure, 1.5) - 1.5).abs() < f32::EPSILON);

        board.Set(Keys::Alerted, true).unwrap();
        board.Set(Keys::Count, 7).unwrap();
        board.Set(Keys::Pressure, 2.0).unwrap();
        board.Set(Keys::Alerted, false).unwrap();

        assert_eq!(board.TryGet(Keys::Alerted), Some(false));
        assert_eq!(board.TryGet(Keys::Count), Some(7));
        assert_eq!(board.TryGet(Keys::Pressure), Some(2.0));
        assert!(board.IsDirty(Keys::Alerted));
        assert_eq!(board.DirtySlots(), vec![1, 2, 3]);

        board.ClearDirty();
        assert_eq!(board.DirtySlots(), Vec::<u32>::new());

        let alias_ok = DwKey::<bool>::New("Alerted", 1);
        board.Set(alias_ok, true).unwrap();

        let bad_name = DwKey::<bool>::New("Other", 1);
        assert!(board.Set(bad_name, true).is_err());
        assert!(board.LastSlotCollision().is_some());

        let bad_type = DwKey::<i32>::New("Alerted", 1);
        assert!(board.Set(bad_type, 1).is_err());
        assert_eq!(board.DirtySlots(), vec![1]);
    }

    #[test]
    fn BoardFlowsAcrossParentAndChildFramesAndDirtyResetsPerTick() {
        #[derive(Clone, Copy)]
        enum R {
            Start,
            Verify,
            Done,
        }
        impl DwPhase for R {
            fn ToPc(self) -> u32 {
                match self {
                    R::Start => 0,
                    R::Verify => 1,
                    R::Done => 2,
                }
            }
            fn FromPc(pc: u32) -> Option<Self> {
                match pc {
                    0 => Some(R::Start),
                    1 => Some(R::Verify),
                    2 => Some(R::Done),
                    _ => None,
                }
            }
        }
        #[derive(Clone, Copy)]
        enum C {
            Start,
        }
        impl DwPhase for C {
            fn ToPc(self) -> u32 {
                0
            }
            fn FromPc(pc: u32) -> Option<Self> {
                if pc == 0 { Some(C::Start) } else { None }
            }
        }

        fn RootF(ctx: &mut DwFrameCtx) -> DwControl {
            match ctx.Phase::<R>() {
                Some(R::Start) => {
                    ctx.BoardMut().Set(Keys::Count, 10).unwrap();
                    Dw::Push(
                        DwFrameId {
                            Domain: 4,
                            Local: 2,
                        },
                        R::Verify,
                    )
                }
                Some(R::Verify) => {
                    let count = ctx.Board().GetOr(Keys::Count, -1);
                    if count == 11 {
                        Dw::Continue(R::Done)
                    } else {
                        Dw::Fail("count")
                    }
                }
                Some(R::Done) => Dw::Complete(),
                None => Dw::Fail("root phase"),
            }
        }

        fn ChildF(ctx: &mut DwFrameCtx) -> DwControl {
            let count = ctx.Board().GetOr(Keys::Count, -1);
            if count != 10 {
                return Dw::Fail("missing parent value");
            }
            ctx.BoardMut().Set(Keys::Count, 11).unwrap();
            Dw::Pop()
        }

        let root = DwFrameId {
            Domain: 4,
            Local: 1,
        };
        let child = DwFrameId {
            Domain: 4,
            Local: 2,
        };
        let mut reg = DwFrameRegistry::New();
        reg.Register(DwFrameDef {
            Id: root,
            Step: RootF,
            DebugName: "Root",
        })
        .unwrap();
        reg.Register(DwFrameDef {
            Id: child,
            Step: ChildF,
            DebugName: "Child",
        })
        .unwrap();

        let mut s = DwSession::New(reg, root, 0).unwrap();
        let t0 = s.Tick().unwrap();
        assert_eq!(t0.DirtySlots, vec![2]);
        let t1 = s.Tick().unwrap();
        assert_eq!(t1.DirtySlots, vec![2]);
        let t2 = s.Tick().unwrap();
        assert_eq!(t2.DirtySlots, Vec::<u32>::new());
        assert_eq!(s.Board().TryGet(Keys::Count), Some(11));
    }

    #[test]
    fn MailboxEmptyPeekConsumeAndFifoBehavior() {
        let mut mailbox = DwMailbox::New();
        assert_eq!(
            mailbox.PeekFront(),
            None,
            "expected empty mailbox peek to return None before any seeding"
        );
        assert_eq!(
            mailbox.ConsumeFront(),
            None,
            "expected empty mailbox consume to return None before any seeding"
        );

        mailbox.EnqueueVisibleForTest(DwMessage { Kind: 1, Value: 11 });
        mailbox.EnqueueVisibleForTest(DwMessage { Kind: 2, Value: 22 });
        assert_eq!(
            mailbox.PeekFront(),
            Some(DwMessage { Kind: 1, Value: 11 }),
            "expected peek to show the front visible message without consuming it"
        );
        assert_eq!(
            mailbox.ConsumeFront(),
            Some(DwMessage { Kind: 1, Value: 11 }),
            "expected consume to remove the earliest visible message first (FIFO)"
        );
        assert_eq!(
            mailbox.ConsumeFront(),
            Some(DwMessage { Kind: 2, Value: 22 }),
            "expected consume to continue preserving FIFO order"
        );
    }

    #[test]
    fn MailboxStagingBoundaryAndWaitPromotionAreDeterministic() {
        #[derive(Clone, Copy)]
        enum P {
            Start,
            Check,
            Done,
        }
        impl DwPhase for P {
            fn ToPc(self) -> u32 {
                match self {
                    P::Start => 0,
                    P::Check => 1,
                    P::Done => 2,
                }
            }
            fn FromPc(pc: u32) -> Option<Self> {
                match pc {
                    0 => Some(P::Start),
                    1 => Some(P::Check),
                    2 => Some(P::Done),
                    _ => None,
                }
            }
        }

        fn RootF(ctx: &mut DwFrameCtx) -> DwControl {
            match ctx.Phase::<P>() {
                Some(P::Start) => {
                    let before = ctx.Mailbox().PeekFront();
                    assert_eq!(
                        before,
                        Some(DwMessage { Kind: 7, Value: 70 }),
                        "expected seeded visible message to be readable at start phase"
                    );
                    ctx.MailboxMut().Enqueue(DwMessage { Kind: 8, Value: 80 });
                    let same_tick = ctx.Mailbox().PeekFront();
                    assert_eq!(
                        same_tick,
                        Some(DwMessage { Kind: 7, Value: 70 }),
                        "expected staged message to remain invisible during same tick"
                    );
                    Dw::WaitTicks(1, P::Check)
                }
                Some(P::Check) => {
                    let consumed = ctx.MailboxMut().ConsumeFront();
                    assert_eq!(
                        consumed,
                        Some(DwMessage { Kind: 7, Value: 70 }),
                        "expected old visible message to stay available until explicitly consumed"
                    );
                    let promoted = ctx.Mailbox().PeekFront();
                    assert_eq!(
                        promoted,
                        Some(DwMessage { Kind: 8, Value: 80 }),
                        "expected staged message to promote at tick boundary while wait elapsed"
                    );
                    Dw::Continue(P::Done)
                }
                Some(P::Done) => Dw::Complete(),
                None => Dw::Fail("bad phase"),
            }
        }

        let root = DwFrameId {
            Domain: 5,
            Local: 1,
        };
        let mut reg = DwFrameRegistry::New();
        reg.Register(DwFrameDef {
            Id: root,
            Step: RootF,
            DebugName: "Root",
        })
        .unwrap();
        let mut s = DwSession::New(reg, root, 0).unwrap();
        s.MailboxMut()
            .EnqueueVisibleForTest(DwMessage { Kind: 7, Value: 70 });
        let t0 = s.Tick().unwrap();
        assert_eq!(
            t0.VisibleMailbox,
            vec![DwMessage { Kind: 7, Value: 70 }],
            "expected visible snapshot to keep unconsumed visible message after start tick"
        );
        assert_eq!(
            t0.StagedMailbox,
            vec![DwMessage { Kind: 8, Value: 80 }],
            "expected staged snapshot to include message enqueued during tick"
        );
        let t1 = s.Tick().unwrap();
        assert_eq!(
            t1.Status,
            DwRunStatus::Running,
            "expected wait countdown tick to return running after wait reaches zero"
        );
        assert_eq!(
            t1.VisibleMailbox,
            vec![
                DwMessage { Kind: 7, Value: 70 },
                DwMessage { Kind: 8, Value: 80 }
            ],
            "expected staged message promotion at tick boundary to preserve FIFO behind existing visible messages"
        );
        assert_eq!(
            t1.StagedMailbox,
            Vec::<DwMessage>::new(),
            "expected staged queue to be empty immediately after promotion"
        );
    }
}
