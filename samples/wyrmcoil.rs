#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use dunewyrm::{
    Dw, DwActRequest, DwControl, DwFrameCtx, DwFrameDef, DwFrameRegistry, DwMessage, DwPhase,
    DwRuntimeChunk, DwSession, DwTickResult,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WcVec2 {
    pub X: f32,
    pub Y: f32,
}

impl WcVec2 {
    pub fn Zero() -> Self {
        Self { X: 0.0, Y: 0.0 }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WcEntityId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct WcTransformStore {
    pub Positions: Vec<WcVec2>,
    pub Velocities: Vec<WcVec2>,
    pub Alive: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WcTransformStoreChunk {
    pub Positions: Vec<WcVec2>,
    pub Velocities: Vec<WcVec2>,
    pub Alive: Vec<bool>,
}

impl WcTransformStore {
    pub fn New() -> Self {
        Self {
            Positions: Vec::new(),
            Velocities: Vec::new(),
            Alive: Vec::new(),
        }
    }
    pub fn Spawn(&mut self, position: WcVec2) -> WcEntityId {
        let id = WcEntityId(self.Positions.len());
        self.Positions.push(position);
        self.Velocities.push(WcVec2::Zero());
        self.Alive.push(true);
        id
    }
    pub fn SetVelocity(&mut self, id: WcEntityId, velocity: WcVec2) {
        if id.0 < self.Velocities.len() && self.Alive[id.0] {
            self.Velocities[id.0] = velocity;
        }
    }
    pub fn Position(&self, id: WcEntityId) -> Option<WcVec2> {
        self.Positions.get(id.0).copied()
    }
    pub fn Velocity(&self, id: WcEntityId) -> Option<WcVec2> {
        self.Velocities.get(id.0).copied()
    }
    pub fn Tick(&mut self) {
        for index in 0..self.Positions.len() {
            if self.Alive[index] {
                self.Positions[index].X += self.Velocities[index].X;
                self.Positions[index].Y += self.Velocities[index].Y;
            }
        }
    }
    pub fn ExportChunk(&self) -> WcTransformStoreChunk {
        WcTransformStoreChunk {
            Positions: self.Positions.clone(),
            Velocities: self.Velocities.clone(),
            Alive: self.Alive.clone(),
        }
    }
    pub fn FromChunk(chunk: WcTransformStoreChunk) -> Self {
        Self {
            Positions: chunk.Positions,
            Velocities: chunk.Velocities,
            Alive: chunk.Alive,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WcWorld {
    pub Transforms: WcTransformStore,
}
#[derive(Clone, Debug, PartialEq)]
pub struct WcWorldChunk {
    pub Transforms: WcTransformStoreChunk,
}
impl WcWorld {
    pub fn New() -> Self {
        Self {
            Transforms: WcTransformStore::New(),
        }
    }
    pub fn Tick(&mut self) {
        self.Transforms.Tick();
    }
    pub fn ExportChunk(&self) -> WcWorldChunk {
        WcWorldChunk {
            Transforms: self.Transforms.ExportChunk(),
        }
    }
    pub fn FromChunk(chunk: WcWorldChunk) -> Self {
        Self {
            Transforms: WcTransformStore::FromChunk(chunk.Transforms),
        }
    }
}

pub mod WcFrames {
    use dunewyrm::DwFrameId;
    pub const Domain: u64 = 310;
    pub const Root: DwFrameId = DwFrameId { Domain, Local: 1 };
    pub const Player: DwFrameId = DwFrameId { Domain, Local: 2 };
    pub const Guard: DwFrameId = DwFrameId { Domain, Local: 3 };
}
pub mod WcActs {
    use dunewyrm::DwActId;
    pub const Domain: u64 = 311;
    pub const MovePlayerRight: DwActId = DwActId { Domain, Local: 1 };
    pub const MovePlayerLeft: DwActId = DwActId { Domain, Local: 2 };
    pub const StopPlayer: DwActId = DwActId { Domain, Local: 3 };
    pub const EnemyStep: DwActId = DwActId { Domain, Local: 4 };
    pub const CallBackup: DwActId = DwActId { Domain, Local: 5 };
}
pub mod WcKeys {
    use dunewyrm::DwKey;
    pub const GuardAlert: DwKey<bool> = DwKey::New("GuardAlert", 20);
}
pub mod WcMailKinds {
    pub const MoveRight: u32 = 1;
    pub const MoveLeft: u32 = 2;
    pub const Stop: u32 = 3;
    pub const AlertGuard: u32 = 4;
}

#[derive(Clone, Copy)]
enum RootPhase {
    Player,
    Guard,
    Loop,
}
impl DwPhase for RootPhase {
    fn ToPc(self) -> u32 {
        match self {
            RootPhase::Player => 0,
            RootPhase::Guard => 1,
            RootPhase::Loop => 2,
        }
    }
    fn FromPc(pc: u32) -> Option<Self> {
        match pc {
            0 => Some(RootPhase::Player),
            1 => Some(RootPhase::Guard),
            2 => Some(RootPhase::Loop),
            _ => None,
        }
    }
}
#[derive(Clone, Copy)]
enum UnitPhase {
    Enter,
    Finish,
}
impl DwPhase for UnitPhase {
    fn ToPc(self) -> u32 {
        match self {
            UnitPhase::Enter => 0,
            UnitPhase::Finish => 1,
        }
    }
    fn FromPc(pc: u32) -> Option<Self> {
        match pc {
            0 => Some(UnitPhase::Enter),
            1 => Some(UnitPhase::Finish),
            _ => None,
        }
    }
}

fn Root(ctx: &mut DwFrameCtx) -> DwControl {
    match ctx.Phase::<RootPhase>() {
        Some(RootPhase::Player) => Dw::Push(WcFrames::Player, RootPhase::Guard),
        Some(RootPhase::Guard) => Dw::Push(WcFrames::Guard, RootPhase::Loop),
        Some(RootPhase::Loop) => Dw::Continue(RootPhase::Player),
        None => Dw::Fail("wyrmcoil root phase invalid"),
    }
}
fn Player(ctx: &mut DwFrameCtx) -> DwControl {
    match ctx.Phase::<UnitPhase>() {
        Some(UnitPhase::Enter) => {
            while let Some(message) = ctx.MailboxMut().ConsumeFront() {
                if message.Kind == WcMailKinds::MoveRight {
                    ctx.Immediate(WcActs::MovePlayerRight);
                } else if message.Kind == WcMailKinds::MoveLeft {
                    ctx.Immediate(WcActs::MovePlayerLeft);
                } else if message.Kind == WcMailKinds::Stop {
                    ctx.Immediate(WcActs::StopPlayer);
                } else if message.Kind == WcMailKinds::AlertGuard {
                    ctx.BoardMut()
                        .Set(WcKeys::GuardAlert, true)
                        .expect("guard alert key write should succeed");
                }
            }
            Dw::Continue(UnitPhase::Finish)
        }
        Some(UnitPhase::Finish) => Dw::Pop(),
        None => Dw::Fail("wyrmcoil player phase invalid"),
    }
}
fn Guard(ctx: &mut DwFrameCtx) -> DwControl {
    match ctx.Phase::<UnitPhase>() {
        Some(UnitPhase::Enter) => {
            ctx.Immediate(WcActs::EnemyStep);
            if ctx.Board().GetOr(WcKeys::GuardAlert, false) {
                ctx.Deferred(WcActs::CallBackup, 1);
            }
            Dw::Continue(UnitPhase::Finish)
        }
        Some(UnitPhase::Finish) => Dw::Pop(),
        None => Dw::Fail("wyrmcoil guard phase invalid"),
    }
}

pub fn BuildRegistry() -> DwFrameRegistry {
    let mut registry = DwFrameRegistry::New();
    registry
        .Register(DwFrameDef {
            Id: WcFrames::Root,
            Step: Root,
            DebugName: "WcRoot",
        })
        .expect("WcRoot should register exactly once");
    registry
        .Register(DwFrameDef {
            Id: WcFrames::Player,
            Step: Player,
            DebugName: "WcPlayer",
        })
        .expect("WcPlayer should register exactly once");
    registry
        .Register(DwFrameDef {
            Id: WcFrames::Guard,
            Step: Guard,
            DebugName: "WcGuard",
        })
        .expect("WcGuard should register exactly once");
    registry
}

pub fn DispatchActs(
    world: &mut WcWorld,
    acts: &[DwActRequest],
    player: WcEntityId,
    guard: WcEntityId,
) {
    for act in acts {
        if act.Id == WcActs::MovePlayerRight {
            world
                .Transforms
                .SetVelocity(player, WcVec2 { X: 1.0, Y: 0.0 });
        } else if act.Id == WcActs::MovePlayerLeft {
            world
                .Transforms
                .SetVelocity(player, WcVec2 { X: -1.0, Y: 0.0 });
        } else if act.Id == WcActs::StopPlayer {
            world.Transforms.SetVelocity(player, WcVec2::Zero());
        } else if act.Id == WcActs::EnemyStep {
            world
                .Transforms
                .SetVelocity(guard, WcVec2 { X: 0.0, Y: 1.0 });
        } else if act.Id == WcActs::CallBackup {
            world
                .Transforms
                .SetVelocity(guard, WcVec2 { X: 1.0, Y: 1.0 });
        }
    }
}

pub struct WcEngine {
    pub Session: DwSession,
    pub World: WcWorld,
    pub Player: WcEntityId,
    pub Guard: WcEntityId,
}
#[derive(Clone, Debug, PartialEq)]
pub struct WcEngineChunk {
    pub Runtime: DwRuntimeChunk,
    pub World: WcWorldChunk,
    pub Player: WcEntityId,
    pub Guard: WcEntityId,
}
pub struct WcTickResult {
    pub Runtime: DwTickResult,
    pub World: WcWorld,
}

impl WcEngine {
    pub fn New() -> Self {
        let mut world = WcWorld::New();
        let player = world.Transforms.Spawn(WcVec2::Zero());
        let guard = world.Transforms.Spawn(WcVec2 { X: 5.0, Y: 5.0 });
        let session = DwSession::New(BuildRegistry(), WcFrames::Root, 0)
            .expect("WyrmCoil session should construct");
        Self {
            Session: session,
            World: world,
            Player: player,
            Guard: guard,
        }
    }
    pub fn Tick(&mut self) -> WcTickResult {
        let runtime = self
            .Session
            .Tick()
            .expect("WyrmCoil engine tick should succeed");
        DispatchActs(
            &mut self.World,
            &runtime.ImmediateActs,
            self.Player,
            self.Guard,
        );
        DispatchActs(
            &mut self.World,
            &runtime.MaturedDeferredActs,
            self.Player,
            self.Guard,
        );
        self.World.Tick();
        WcTickResult {
            Runtime: runtime,
            World: self.World.clone(),
        }
    }
    pub fn ExportChunk(&self) -> WcEngineChunk {
        WcEngineChunk {
            Runtime: self.Session.ExportChunk(),
            World: self.World.ExportChunk(),
            Player: self.Player,
            Guard: self.Guard,
        }
    }
    pub fn FromChunk(chunk: WcEngineChunk) -> Self {
        let session = DwSession::FromChunk(BuildRegistry(), chunk.Runtime)
            .expect("WyrmCoil session restore should succeed");
        Self {
            Session: session,
            World: WcWorld::FromChunk(chunk.World),
            Player: chunk.Player,
            Guard: chunk.Guard,
        }
    }
}

pub fn MoveRightMessage() -> DwMessage {
    DwMessage {
        Kind: WcMailKinds::MoveRight,
        Value: 1,
    }
}
pub fn MoveLeftMessage() -> DwMessage {
    DwMessage {
        Kind: WcMailKinds::MoveLeft,
        Value: 1,
    }
}
pub fn StopMessage() -> DwMessage {
    DwMessage {
        Kind: WcMailKinds::Stop,
        Value: 1,
    }
}
pub fn AlertGuardMessage() -> DwMessage {
    DwMessage {
        Kind: WcMailKinds::AlertGuard,
        Value: 1,
    }
}
