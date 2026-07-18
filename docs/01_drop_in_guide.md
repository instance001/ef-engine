# Drop-In Guide

This repo is meant to be easy to steal from.

## Smallest Useful Use

You only need a few concepts:

- `EngineState`
- `AttemptInput`
- `apply_attempt(...)`
- `metabolize_constraint(...)`

Everything else is support around those moves.

If you are in a hurry, copy `engine.rs`, make one tiny host, and ignore the
rest until you need save/load.

## Typical Integration

1. Copy the crate or the core files into your project.
2. Rename the nouns if they do not fit your host.
3. Record each meaningful attempt through `apply_attempt(...)`.
4. Vault honest failures instead of hiding them.
5. Metabolize repeated or triangulated failures into constraints.
6. Consult those constraints before future attempts.

## Rename Early

If these words feel wrong, rename them on day one:

- `vault`
- `constraint`
- `triangulation`
- `routing`
- `stuck`

The mechanism survives renaming.
Clarity inside your host matters more than fidelity to this repo's vocabulary.

## Good Adaptation Signs

You are using the pattern well if:

- each attempt is inspectable
- failures are preserved as first-class records
- retries show what changed
- constraints affect future routing
- the host gets less confused over time
