# Dunewyrm Architecture Contract (Pre-M0)

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
