#![allow(non_snake_case)]

use dunewyrm::{CompareTrace, DwActRequest};

#[path = "../samples/wyrmcoil.rs"]
mod wyrmcoil;

#[test]
fn DenseStoreSpawnVelocityTickAndChunkRoundTrip() {
    let mut store = wyrmcoil::WcTransformStore::New();
    let e0 = store.Spawn(wyrmcoil::WcVec2 { X: 2.0, Y: 3.0 });
    store.SetVelocity(e0, wyrmcoil::WcVec2 { X: 1.5, Y: -1.0 });
    store.Tick();
    assert_eq!(
        store.Position(e0),
        Some(wyrmcoil::WcVec2 { X: 3.5, Y: 2.0 }),
        "position should advance deterministically by velocity each store tick"
    );
    let chunk = store.ExportChunk();
    let restored = wyrmcoil::WcTransformStore::FromChunk(chunk);
    assert_eq!(
        restored, store,
        "transform store chunk restore should preserve dense arrays exactly"
    );
}

#[test]
fn ActBridgeDispatchesImmediateAndDeferredAndIgnoresUnknown() {
    let mut world = wyrmcoil::WcWorld::New();
    let player = world.Transforms.Spawn(wyrmcoil::WcVec2::Zero());
    let guard = world.Transforms.Spawn(wyrmcoil::WcVec2::Zero());
    let acts = vec![
        DwActRequest {
            Id: wyrmcoil::WcActs::MovePlayerRight,
        },
        DwActRequest {
            Id: wyrmcoil::WcActs::EnemyStep,
        },
        DwActRequest {
            Id: dunewyrm::DwActId {
                Domain: 999,
                Local: 999,
            },
        },
    ];
    wyrmcoil::DispatchActs(&mut world, &acts, player, guard);
    assert_eq!(
        world.Transforms.Velocity(player),
        Some(wyrmcoil::WcVec2 { X: 1.0, Y: 0.0 }),
        "player velocity should be set by MovePlayerRight act"
    );
    assert_eq!(
        world.Transforms.Velocity(guard),
        Some(wyrmcoil::WcVec2 { X: 0.0, Y: 1.0 }),
        "guard velocity should be set by EnemyStep act"
    );
}

#[test]
fn EngineTickDispatchesActsMailboxAndWorldUpdate() {
    let mut engine = wyrmcoil::WcEngine::New();
    engine
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::MoveRightMessage());
    engine
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::AlertGuardMessage());
    engine.Session.MailboxMut().Enqueue(wyrmcoil::StopMessage());

    let _t0 = engine.Tick();
    let t1 = engine.Tick();
    assert!(
        t1.Runtime.ImmediateActs.contains(&DwActRequest {
            Id: wyrmcoil::WcActs::MovePlayerRight
        }),
        "player frame should convert MoveRight mailbox message into immediate act"
    );

    let mut saw_enemy_step = false;
    let mut runtime_probe = engine.Tick();
    for _ in 0..4 {
        if runtime_probe.Runtime.ImmediateActs.contains(&DwActRequest {
            Id: wyrmcoil::WcActs::EnemyStep,
        }) {
            saw_enemy_step = true;
            break;
        }
        runtime_probe = engine.Tick();
    }
    assert!(
        saw_enemy_step,
        "guard frame should emit EnemyStep immediate act within a short deterministic loop window"
    );

    let mut matured_call_backup = false;
    let mut final_tick = engine.Tick();
    for _ in 0..6 {
        if final_tick
            .Runtime
            .MaturedDeferredActs
            .contains(&DwActRequest {
                Id: wyrmcoil::WcActs::CallBackup,
            })
        {
            matured_call_backup = true;
            break;
        }
        final_tick = engine.Tick();
    }
    assert!(
        matured_call_backup,
        "deferred CallBackup act should mature on a later deterministic tick after guard alert"
    );
    assert_eq!(
        engine.World.Transforms.Position(engine.Player),
        final_tick.World.Transforms.Position(engine.Player),
        "tick result world snapshot should match engine world after update"
    );
}

#[test]
fn EngineChunkRestoreMatchesUninterruptedWorldAndTrace() {
    let mut uninterrupted = wyrmcoil::WcEngine::New();
    uninterrupted
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::MoveLeftMessage());
    uninterrupted
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::AlertGuardMessage());
    for _ in 0..8 {
        uninterrupted.Tick();
    }

    let mut split = wyrmcoil::WcEngine::New();
    split
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::MoveLeftMessage());
    split
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::AlertGuardMessage());
    for _ in 0..4 {
        split.Tick();
    }
    let chunk = split.ExportChunk();
    let mut restored = wyrmcoil::WcEngine::FromChunk(chunk);
    for _ in 0..4 {
        restored.Tick();
    }

    assert_eq!(
        uninterrupted.World, restored.World,
        "restored engine world should match uninterrupted execution across dense store updates"
    );
    let mut combined_trace = split.Session.Trace().to_vec();
    combined_trace.extend_from_slice(restored.Session.Trace());
    let comparison = CompareTrace(uninterrupted.Session.Trace(), &combined_trace);
    assert!(
        comparison.Matches,
        "restored runtime trace should match uninterrupted trace for engine-level chunk resume continuity"
    );
}
