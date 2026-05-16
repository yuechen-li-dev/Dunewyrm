# Dunewyrm

Dunewyrm is the Rust control-kernel sibling of the Dominatus / DragonGod family.


## Read this first

- Architecture contract: `docs/architecture.md`
- Practical frame-authoring guide: `docs/authoring.md`
- Canonical external sample: `samples/guard_patrol.rs`
- Sample integration coverage: `tests/m9_guard_patrol_sample.rs`

## What Dunewyrm is

- A Rust-native expression of the same execution-model truths used in the family runtimes.
- A project that prioritizes explicit state, explicit progression, deterministic behavior, and testability.
- Currently in **M10 docs/API polish/warning cleanup on top of the M1–M9 runtime arc**.

## What Dunewyrm is not

- Not a line-by-line DragonGod C++ copy.
- Not a macro-heavy or trait-heavy Rust showcase.
- Not a broad full-runtime implementation yet (currently M10 docs/API polish on top of M1–M9 runtime arc).

## Status

This repository currently contains:

- Primer rules in `primer/`.
- Project and contributor rules in `AGENTS.md`.
- Stack runtime kernel implementation for M2 plus M3 typed board memory (`bool`, `i32`, `f32`) with dirty tracking and slot collision diagnostics, and M4 deterministic mailbox visible/staged semantics.
- A compact architecture/milestone contract in `docs/architecture.md`.
- A practical authoring guide in `docs/authoring.md` for frame/runtime usage.

## Milestone sketch

- **Pre-M0 — Scaffold / contract**: repo setup, docs, Cargo smoke test.
- **M1 — Single-frame explicit PC runtime**: Frame IDs, phase trait, control enum, context, registry, continue/wait/complete/fail/stay, tests.
- **M2 — Stack semantics**: Push, Pop, Replace, parent resume, failure behavior.
- **M3 — Typed board memory**: `bool`, `i32`, `f32`, dirty tracking, collision diagnostics.
- **M4 — Mailbox**: visible/staged message queues, deterministic FIFO visibility.
- **M5 — Persistence chunks**: plain data chunks, export/import, save/restore equivalence.
- **M6 — Tick trace and comparison**: trace entries, first mismatch diagnostics, readable formatting.
- **M7 — Utility decisions**: scorers, Decide, hysteresis, min-commit, tie-break, decision traces.
- **M8 — Actuation**: domain-scoped act IDs, immediate/deferred acts, deferred persistence.
- **M9 — First external sample**: tiny Guard Patrol / Recover library-usage proof across stack/board/mailbox/utility/actuation/trace/persistence.

## Test

```bash
cargo test
```


## M7 utility decisions

- Authoring uses plain scorer functions (`DwScoreFn`) with explicit candidates via `Dw::When`.
- `Dw::Decide` clamps scores into `[0,1]`, applies tie-break, min-commit, and hysteresis deterministically.
- The decision tick is real: selected child is pushed and starts on a later tick, and parent resumes the same PC after child return.
- Utility commitment memory is runtime-owned, keyed by frame+PC, and persisted in runtime chunks.
- Decision records are included in tick results and tick trace entries for inspection.
- M8 adds recorded actuation intent only: `DwActId`, immediate acts, deferred acts, and deferred persistence.
- No side-effect handlers or act payload schemas are included yet.


## M9 first external sample

- `samples/guard_patrol.rs` is an intentionally tiny external-style author sample.
- It defines its own frame IDs, act IDs, and board keys, then builds a registry through public APIs only.
- Integration tests in `tests/m9_guard_patrol_sample.rs` exercise stack, board, mailbox, utility decisions, immediate/deferred actuation, trace comparison, and save/restore equivalence.
- The sample is deliberately small; larger domain samples are deferred to later milestones.

## M11 WyrmCoil prototype sample

- `samples/wyrmcoil.rs` adds a tiny engine-core prototype scaffold authored as external-style library usage.
- Thesis: **frames decide, stores iterate, acts connect, mailbox reports back, chunks persist both**.
- WyrmCoil intentionally uses dense typed stores (`Vec<WcVec2>`, `Vec<bool>`) and fixed act IDs; it does **not** implement ECS, renderer, physics, or payload redesign.
- `tests/m11_wyrmcoil_sample.rs` validates deterministic store updates, act bridging, engine tick behavior, mailbox-to-act flow, and runtime+world chunk restore equivalence.
