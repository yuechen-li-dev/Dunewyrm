# Dunewyrm

Dunewyrm is the Rust control-kernel sibling of the Dominatus / DragonGod family.

## What Dunewyrm is

- A Rust-native expression of the same execution-model truths used in the family runtimes.
- A project that prioritizes explicit state, explicit progression, deterministic behavior, and testability.
- Currently in **M8 actuation intent runtime (immediate + deferred recorded acts)**.

## What Dunewyrm is not

- Not a line-by-line DragonGod C++ copy.
- Not a macro-heavy or trait-heavy Rust showcase.
- Not a broad full-runtime implementation yet (currently M1 single-frame kernel only).

## Status

This repository currently contains:

- Primer rules in `primer/`.
- Project and contributor rules in `AGENTS.md`.
- Stack runtime kernel implementation for M2 plus M3 typed board memory (`bool`, `i32`, `f32`) with dirty tracking and slot collision diagnostics, and M4 deterministic mailbox visible/staged semantics.
- A compact architecture/milestone contract in `docs/architecture.md`.

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
- **M9 — First sample**: tiny pressure test for Rust authoring ergonomics.

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
