# Super Mario Bros. — Rust Recreation Plan

## Goal

Build a playable, native Rust 2D platform game in this repository, using the original NES title as a behavioral reference. This is a clean-room recreation: study observable game rules, but do not copy Nintendo source code, extracted graphics, music, sound effects, or encoded level data into the new game. Use original placeholder/replacement assets until separately licensed assets are supplied.

This is distinct from running the ROM in an emulator. The recreation must run as a native Rust application. The existing NRS emulator remains a separate workstream.

## Repository integration

- Keep the existing nrs-core, nrs-cpu, and nrs-rom crates intact.
- Add the recreation as a separate workspace application/crate.
- Do not change the existing CPU implementation to implement game mechanics.
- Preserve a boundary between ROM-analysis tooling and the native game.

## Game decomposition

### 1. Runtime and rendering
- Window, fixed logical resolution, integer scaling and frame pacing.
- Camera following the player with horizontal level scrolling.
- Tilemap renderer, sprite animation and layered backgrounds.
- HUD: score, coins, world/stage, timer and lives.
- Pause, restart, game-over and stage-complete states.

### 2. World and level model
- Tile definitions: empty, solid, one-way/platform, breakable, question block, pipe, hazard and goal.
- Data-driven level format with dimensions, tile grid, entities and checkpoints.
- Collision queries against tiles and moving entities.
- Camera bounds and level transition boundaries.
- Original test stages; do not copy the ROM's encoded level maps.

### 3. Player controller
- Horizontal acceleration, deceleration, maximum run speed and friction.
- Grounded/airborne states, jump initiation, variable jump height, gravity and fall limits.
- Run button behavior, facing direction and animation state.
- Head collision, landing, edge behavior, enemy stomp and damage response.
- Fixed-step simulation for deterministic movement.

### 4. Interactables and power-ups
- Question blocks and hidden/revealed contents.
- Breakable blocks and coin collection.
- Mushroom/fire-flower-like power-up behavior with original replacement art.
- Pipes and stage exits.
- Score, coin and extra-life rules.

### 5. Enemies and hazards
- Walking enemy state machine: patrol, turn at obstacles/edges, stomped and defeated.
- Shell-like moving hazard state machine and interactions.
- Flying/jumping enemy behavior.
- Projectile behavior for the powered-up player.
- Damage, invulnerability window, knockback and death sequence.
- Explicit state logic independent of render frame rate.

### 6. Game flow and progression
- Title/menu, stage start, active play, death, stage clear and game over.
- Countdown timer and timeout.
- World/stage progression and checkpoints.
- Score/lives persistence across stages.
- Deterministic reset behavior.

### 7. Audio
- Audio event interface separated from game logic.
- Placeholder tones/effects initially; do not copy original recordings.
- Music state changes represented with original compositions.

## Proposed architecture

Initial crate: apps/smb-recreation

Suggested modules:
- app: startup, window, input and main loop.
- game: game state, transitions, score and timer.
- world: tile definitions, level data, collision and camera.
- player: movement, physics, states and animation selection.
- entities: enemies, power-ups, projectiles and interactive blocks.
- render: tilemap, sprites, HUD and scaling.
- audio: event interface and audio backend.
- assets: asset loading and validation.

Prefer a small 2D framework for the first vertical slice. Keep simulation independent of rendering and input so it can be unit-tested without opening a window.

## Milestones

### M0 — Repository and ROM inventory
- Confirm the exact ROM path, branch and file identity.
- Record iNES header metadata and checksum for local analysis only.
- Inventory existing workspace crates and CI constraints.
- Do not commit extracted ROM graphics, audio, code or level data.

### M1 — Playable movement prototype
- Create the standalone Rust application.
- Render a test tilemap and placeholder player.
- Implement fixed-step movement, jump, gravity and solid-tile collision.
- Add camera tracking and keyboard/controller input.
- Add unit tests for movement and collision.

### M2 — Core platform interactions
- Add breakable/question blocks, coins, pipes and goal trigger.
- Add one patrol enemy and stomp interaction.
- Add damage/death/respawn and score/lives.
- Test head bumps, landing, side collision, enemy contact and falling.

### M3 — Full gameplay loop
- Add additional enemy and power-up state machines.
- Add HUD, timer, title, stage clear and game-over screens.
- Add level loading from a documented data format.
- Add several original test stages.

### M4 — Fidelity and polish
- Compare observable movement and interaction behavior against the reference game.
- Tune physics through recorded test scenarios, not guessed frame values.
- Add replacement art, animation and original audio.
- Profile frame time and remove avoidable allocations from update/render.

## Engineering requirements

- Rust edition 2024, workspace formatting and lint conventions.
- No unsafe code in game logic.
- Fixed simulation timestep; rendering may interpolate but must not alter physics.
- Unit tests for physics, collision, entity state transitions, scoring and stage flow.
- Deterministic test seeds and reproducible level fixtures.
- No breaking changes to existing NRS emulator crates.

## First acceptance target

A native window opens and displays a scrolling test level. The player can run, jump, land on platforms, hit the underside of a block, collect a coin, stomp a simple enemy and reach a goal. The simulation has automated tests and builds with the repository workspace commands.