# Dunewyrm Architecture Contract (M3)

Dunewyrm is a Rust-native sibling of DragonGod, not a line-by-line port.

## Runtime truth preserved

- Explicit state and explicit PC/phase progression.
- Deterministic ticks and stack-based control.
- Typed board memory, dirty tracking, and deterministic mailbox boundaries.
- Persistence chunks and trace comparison.

## Rust posture

- Use Rust features where they clarify design (enums, matching, typed phases, helper functions).
- Prefer owned runtime state over lifetime-heavy borrowed architectures.
- Keep authoring friendly and explicit; avoid trait/object/generic theater.

## Early constraints

- No async/generators/nightly/proc-macro dependency in early runtime stages.
- `FrameId` and `ActId` should evolve as domain-scoped numeric identities from the start.
- Board memory should be typed and bounded, not a generic object store.


## M2 stack semantics

- Runtime uses an explicit frame stack with one frame-step call per tick.
- `Push` stores parent resume PC and schedules child start at PC 0 on a later tick.
- `Pop` removes top frame and resumes parent on a later tick.
- `Replace` removes current top frame and installs target frame at PC 0 on a later tick.
- Child `Complete` is treated as a successful pop; root `Complete` ends the session.
- `WaitTicks` applies only to the current top frame and blocks parent execution while waiting.
- Stack runtime is still pre-board, pre-mailbox, pre-utility, and pre-actuation.


## M3 typed board memory

- Session-owned board shared by all frames in the active stack.
- Typed keys (`DwKey<T>`) are currently closed to `bool`, `i32`, and `f32`.
- Board supports `Set`, `TryGet`, `GetOr`, `IsDirty`, `DirtySlots`, and `ClearDirty`.
- Dirty tracking is automatic for successful writes and reset at tick start.
- Slot collisions are diagnosed when a slot is reused with different name or type.
- Board is control working memory only, not a generic object store and not persistence.
