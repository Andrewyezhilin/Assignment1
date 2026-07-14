# AGENTS.md

## Cursor Cloud specific instructions

### What this is
`ortalab` is a single-binary Rust CLI (a Balatro-style poker-hand scoring engine). It reads a
YAML `Round` (see `ortalib` crate types) from a file argument or from stdin when the argument is
`-`, and prints `floor(chips * mult)` to stdout. Optional `--explain` flag is accepted.

### Toolchain gotcha (important)
`Cargo.toml` uses `edition = "2024"`, which requires Rust >= 1.85. The VM base image's default
`rustup` toolchain is 1.83, which fails to even parse the manifest. The update script installs and
sets `stable` as the default toolchain (with `clippy` and `rustfmt`), so builds work out of the box.
If you ever see `feature edition2024 is required`, run `rustup default stable`.

### Common commands (standard, no wrappers)
- Build (dev): `cargo build`
- Run: `cargo run -- <round.yml>` or `cargo run -- -` (reads YAML round from stdin)
- Lint: `cargo clippy` (currently emits a few warnings; exits 0)
- Format check: `cargo fmt --check`
- Tests: `cargo test` — the `ortalab` crate itself has no unit tests (0 run). Assignment test
  cases are external/private; validate manually by feeding YAML rounds and checking the printed score.

### Input format
Cards are strings like `A♥`, `Q♣ Bonus`, `7♦ Glass Polychrome`; jokers like `Joker`,
`Blueprint Foil`, `Sock And Buskin Polychrome`. Top-level YAML keys: `cards_played` (required),
`cards_held_in_hand` (optional), `jokers` (optional). Suits use the unicode symbols ♠♥♣♦.
