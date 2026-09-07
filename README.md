# Emberflow - fire + water terminal screensavers

Built with [iocraft](https://github.com/ccbrown/iocraft) - the React moment for terminal.
Declarative `element!` + `#[component]` + hooks (`use_state`, `use_future`, `use_terminal_events`).

## Run

```bash
cargo run
```

Opens fullscreen animated TUI.

## Controls

- `1` fire
- `2` water
- `3` split (fire + water side by side)
- `space` pause
- `+` / `-` speed (0.25x to 4x)
- `q` / `esc` quit

## Smoke test (no fullscreen)

```bash
cargo run -- smoke
```

Prints one static frame of each - good for CI / screenshots.

## What is inside

- `src/main.rs` - everything
- Fire: procedural heat field with flicker fuel, turbulence sway, rising sparks, truecolor ramp black -> red -> orange -> yellow -> white, log pile at bottom
- Water: 3 layered sine waves, foam crest, spray drops, shimmer, rising bubbles, swimming fish
- App shell: fullscreen `View`, header/footer chrome, live tick, speed state
