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
pub use session::{DwFrameCtx, DwRunStatus, DwSession, DwTickResult};

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
    fn FrameIdSupportsEqualityAndDomainSeparation() {
        let a = DwFrameId {
            Domain: 1,
            Local: 7,
        };
        let b = DwFrameId {
            Domain: 1,
            Local: 7,
        };
        let c = DwFrameId {
            Domain: 2,
            Local: 7,
        };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn RegistrySupportsRegisterAndLookupAndRejectsDuplicates() {
        let frame_id = DwFrameId {
            Domain: 3,
            Local: 1,
        };
        let mut registry = DwFrameRegistry::New();

        assert!(
            registry
                .Register(DwFrameDef {
                    Id: frame_id,
                    Step: Root,
                    DebugName: "Root",
                })
                .is_ok()
        );

        let found = registry.Find(frame_id).expect("frame should exist");
        assert_eq!(found.DebugName, "Root");

        let duplicate = registry.Register(DwFrameDef {
            Id: frame_id,
            Step: Root,
            DebugName: "RootDuplicate",
        });
        assert!(duplicate.is_err());
    }

    #[test]
    fn PhaseMappingAndContextPhaseWork() {
        assert_eq!(RootPhase::FromPc(1), Some(RootPhase::Wait));
        assert_eq!(RootPhase::Wait.ToPc(), 1);

        let ctx = DwFrameCtx::New(
            DwFrameId {
                Domain: 1,
                Local: 1,
            },
            1,
            5,
        );
        assert_eq!(ctx.Pc(), 1);
        assert_eq!(ctx.Tick(), 5);
        assert_eq!(ctx.Phase::<RootPhase>(), Some(RootPhase::Wait));
    }

    #[test]
    fn SessionTickDeterminismCoversContinueWaitStayCompleteAndFail() {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        enum DemoPhase {
            Start,
            Hold,
            Finish,
            Fail,
        }

        impl DwPhase for DemoPhase {
            fn ToPc(self) -> u32 {
                match self {
                    DemoPhase::Start => 0,
                    DemoPhase::Hold => 1,
                    DemoPhase::Finish => 2,
                    DemoPhase::Fail => 3,
                }
            }

            fn FromPc(pc: u32) -> Option<Self> {
                match pc {
                    0 => Some(DemoPhase::Start),
                    1 => Some(DemoPhase::Hold),
                    2 => Some(DemoPhase::Finish),
                    3 => Some(DemoPhase::Fail),
                    _ => None,
                }
            }
        }

        fn DemoFrame(ctx: &mut DwFrameCtx) -> DwControl {
            match ctx.Phase::<DemoPhase>() {
                Some(DemoPhase::Start) => Dw::Continue(DemoPhase::Hold),
                Some(DemoPhase::Hold) => {
                    if ctx.Tick() == 1 {
                        Dw::WaitTicks(2, DemoPhase::Finish)
                    } else {
                        Dw::Stay()
                    }
                }
                Some(DemoPhase::Finish) => Dw::Continue(DemoPhase::Fail),
                Some(DemoPhase::Fail) => Dw::Fail("terminal fail"),
                None => Dw::Fail("unknown phase"),
            }
        }

        let frame_id = DwFrameId {
            Domain: 9,
            Local: 9,
        };
        let mut registry = DwFrameRegistry::New();
        registry
            .Register(DwFrameDef {
                Id: frame_id,
                Step: DemoFrame,
                DebugName: "Demo",
            })
            .unwrap();

        let mut session = DwSession::New(registry, frame_id, DemoPhase::Start.ToPc()).unwrap();

        let t0 = session.Tick().unwrap();
        assert_eq!(t0.Status, DwRunStatus::Running);
        assert_eq!(t0.Pc, DemoPhase::Hold.ToPc());

        let t1 = session.Tick().unwrap();
        assert_eq!(t1.Status, DwRunStatus::Waiting);
        assert_eq!(t1.Pc, DemoPhase::Hold.ToPc());

        let t2 = session.Tick().unwrap();
        assert_eq!(t2.Status, DwRunStatus::Waiting);
        assert_eq!(t2.Pc, DemoPhase::Hold.ToPc());

        let t3 = session.Tick().unwrap();
        assert_eq!(t3.Status, DwRunStatus::Running);
        assert_eq!(t3.Pc, DemoPhase::Finish.ToPc());

        let t4 = session.Tick().unwrap();
        assert_eq!(t4.Status, DwRunStatus::Running);
        assert_eq!(t4.Pc, DemoPhase::Fail.ToPc());

        let t5 = session.Tick().unwrap();
        assert_eq!(t5.Status, DwRunStatus::Failed);
        assert_eq!(t5.FailureReason, Some("terminal fail"));

        let t6 = session.Tick().unwrap();
        assert_eq!(t6.Status, DwRunStatus::Failed);
    }

    #[test]
    fn AuthoringShapeProofCompletes() {
        let frame_id = DwFrameId {
            Domain: 4,
            Local: 2,
        };
        let mut registry = DwFrameRegistry::New();
        registry
            .Register(DwFrameDef {
                Id: frame_id,
                Step: Root,
                DebugName: "Root",
            })
            .unwrap();

        let mut session = DwSession::New(registry, frame_id, RootPhase::Start.ToPc()).unwrap();
        assert_eq!(session.Tick().unwrap().Status, DwRunStatus::Running);
        assert_eq!(session.Tick().unwrap().Status, DwRunStatus::Waiting);
        assert_eq!(session.Tick().unwrap().Status, DwRunStatus::Running);
        assert_eq!(session.Tick().unwrap().Status, DwRunStatus::Completed);
    }
}
