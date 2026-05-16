#![allow(non_snake_case)]

use dunewyrm::{CompareTrace, DwActRequest};

#[path = "../samples/wyrmcoil.rs"]
mod wyrmcoil;

#[test]
fn DenseStoreMultiEntityVelocityAliveFilteringAndChunkRoundTrip() {
    let mut store = wyrmcoil::WcTransformStore::New();
    let player = store.Spawn(wyrmcoil::WcVec2 { X: 2.0, Y: 3.0 });
    let guard = store.Spawn(wyrmcoil::WcVec2 { X: 9.0, Y: 9.0 });

    store.SetVelocity(player, wyrmcoil::WcVec2 { X: 1.5, Y: -1.0 });
    store.SetVelocity(guard, wyrmcoil::WcVec2 { X: -2.0, Y: 0.5 });
    store.SetAlive(guard, false);
    store.Tick();

    assert_eq!(
        store.Position(player),
        Some(wyrmcoil::WcVec2 { X: 3.5, Y: 2.0 }),
        "alive player entity should integrate velocity into deterministic dense position lanes"
    );
    assert_eq!(
        store.Position(guard),
        Some(wyrmcoil::WcVec2 { X: 9.0, Y: 9.0 }),
        "non-alive guard entity should be filtered out of world integration loop"
    );

    let chunk = store.ExportChunk();
    let restored = wyrmcoil::WcTransformStore::FromChunk(chunk);
    assert_eq!(
        restored, store,
        "transform store chunk restore should preserve positions, velocities, and alive lanes exactly"
    );
}

#[test]
fn ActBridgeReadsBoardBackedCommandIntentAndTargetsOnlyRequestedEntity() {
    let mut world = wyrmcoil::WcWorld::New();
    let player = world.Transforms.Spawn(wyrmcoil::WcVec2::Zero());
    let guard = world.Transforms.Spawn(wyrmcoil::WcVec2::Zero());

    let mut board = dunewyrm::DwBoard::New();
    board
        .Set(wyrmcoil::WcKeys::CommandEntity, player.0 as i32)
        .expect("command entity write should succeed for player targeting");
    board
        .Set(wyrmcoil::WcKeys::CommandVelocityX, 1.0)
        .expect("command velocity x write should succeed for player targeting");
    board
        .Set(wyrmcoil::WcKeys::CommandVelocityY, 0.0)
        .expect("command velocity y write should succeed for player targeting");

    wyrmcoil::DispatchActs(
        &mut world,
        &board,
        &[DwActRequest {
            Id: wyrmcoil::WcActs::ApplyVelocityCommand,
        }],
    );

    assert_eq!(
        world.Transforms.Velocity(player),
        Some(wyrmcoil::WcVec2 { X: 1.0, Y: 0.0 }),
        "board-backed command intent should set velocity only for the addressed player entity"
    );
    assert_eq!(
        world.Transforms.Velocity(guard),
        Some(wyrmcoil::WcVec2::Zero()),
        "player-targeted command intent should not mutate guard velocity lanes"
    );

    board
        .Set(wyrmcoil::WcKeys::CommandEntity, 99)
        .expect("invalid command entity index write should still be representable on board");
    board
        .Set(wyrmcoil::WcKeys::CommandVelocityX, 5.0)
        .expect("invalid command velocity x write should succeed on board");
    board
        .Set(wyrmcoil::WcKeys::CommandVelocityY, 5.0)
        .expect("invalid command velocity y write should succeed on board");

    wyrmcoil::DispatchActs(
        &mut world,
        &board,
        &[DwActRequest {
            Id: wyrmcoil::WcActs::ApplyVelocityCommand,
        }],
    );

    assert_eq!(
        world.Transforms.Velocity(player),
        Some(wyrmcoil::WcVec2 { X: 1.0, Y: 0.0 }),
        "invalid target index should be ignored by the act bridge instead of mutating arbitrary entity lanes"
    );
}

#[test]
fn EngineTickMailboxCommandWritesBoardAndDispatchesCommandActs() {
    let mut engine = wyrmcoil::WcEngine::New();
    engine
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::MoveRightMessage());
    engine
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::NudgeGuardMessage());

    let _t0 = engine.Tick();
    let t1 = engine.Tick();

    assert!(
        t1.Runtime.ImmediateActs.contains(&DwActRequest {
            Id: wyrmcoil::WcActs::ApplyVelocityCommand,
        }),
        "player frame should emit ApplyVelocityCommand immediate act after consuming MoveRight mailbox message"
    );
    assert!(
        t1.Runtime.ImmediateActs.contains(&DwActRequest {
            Id: wyrmcoil::WcActs::NudgeEntityCommand,
        }),
        "player frame should emit NudgeEntityCommand immediate act after consuming nudge mailbox message"
    );
    assert!(
        t1.Runtime.DirtySlots.contains(&21)
            && t1.Runtime.DirtySlots.contains(&22)
            && t1.Runtime.DirtySlots.contains(&23),
        "typed command board keys should be marked dirty when command intent is written by frame logic"
    );

    let before_guard = engine
        .World
        .Transforms
        .Position(engine.Guard)
        .expect("guard entity should exist before deterministic loop steps");

    for _ in 0..4 {
        let _ = engine.Tick();
    }

    let after_guard = engine
        .World
        .Transforms
        .Position(engine.Guard)
        .expect("guard entity should exist after deterministic loop steps");
    assert!(
        after_guard.Y > before_guard.Y,
        "guard position should advance after guard command acts and world integration execute"
    );
}

#[test]
fn EngineChunkRestoreMatchesUninterruptedMultiEntityCommandExecution() {
    let mut uninterrupted = wyrmcoil::WcEngine::New();
    uninterrupted
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::MoveLeftMessage());
    uninterrupted
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::AlertGuardMessage());
    uninterrupted
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::NudgeGuardMessage());
    for _ in 0..10 {
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
    split
        .Session
        .MailboxMut()
        .Enqueue(wyrmcoil::NudgeGuardMessage());
    for _ in 0..5 {
        split.Tick();
    }
    let chunk = split.ExportChunk();
    let mut restored = wyrmcoil::WcEngine::FromChunk(chunk);
    for _ in 0..5 {
        restored.Tick();
    }

    assert_eq!(
        uninterrupted.World.Transforms.Positions, restored.World.Transforms.Positions,
        "restored WyrmCoil world positions should match uninterrupted multi-entity command execution"
    );
    assert_eq!(
        uninterrupted.World.Transforms.Velocities, restored.World.Transforms.Velocities,
        "restored WyrmCoil world velocities should match uninterrupted command bridge output"
    );

    let mut combined_trace = split.Session.Trace().to_vec();
    combined_trace.extend_from_slice(restored.Session.Trace());
    let comparison = CompareTrace(uninterrupted.Session.Trace(), &combined_trace);
    assert!(
        comparison.Matches,
        "restored runtime trace should match uninterrupted trace for multi-entity command-pressure continuation"
    );
}
