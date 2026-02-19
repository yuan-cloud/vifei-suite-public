# A3 C2 Proof · Streaming Fixture Parse Equivalence (2026-02-17)

## Bead

- `bd-6wkf` (A3-1)

## Change

- Tour parse stage now opens fixture via `File` and parses through `BufReader<File>`.
- Removed full-file `read_to_string` + `Cursor` buffering in runtime path.

File changed:

- `crates/vifei-tour/src/lib.rs`

## Equivalence oracle

For the same fixture bytes:

1. Parsed `ImportEvent` sequence from streamed reader equals buffered-reader sequence.
2. Tour artifacts and determinism checks remain unchanged.

## Proof test

Added test:

- `stream_fixture_parse_matches_buffered_parse`

This compares:

- `parse_cassette(BufReader<Cursor<String>>)`
- `parse_cassette(BufReader<File>)`

against the same fixture content and asserts exact event equality.

## Safety notes

- No reducer/projection/eventlog ordering logic changed.
- No hash composition changes.
- This is input transport optimization only.

## Verification

- `cargo test -p vifei-tour stream_fixture_parse_matches_buffered_parse`
- `cargo test -p vifei-tour`
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

All passed.
