# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0]

### Breaking Changes

- `decode_buffer` signature changed: now takes a caller-supplied `&mut Vec<u8>` output
  buffer and returns `Result<(), DecodeError>` instead of allocating and returning a
  `Result<Vec<u8>, DecodeError>`. Callers must pass a `Vec` and can reuse it across calls.
- Minimum Rust version raised to **1.85.1** (Rust edition 2024).

### Fixed

- `encode_stream` now validates options before writing any output, so callers receive an
  error immediately rather than after a partial write.
- `encode_stream` single-part encoding correctly seeks to position 0 and encodes the full
  `length` bytes; previously `begin = 0` caused a subtraction overflow in debug builds.

### Performance

- `encode_buffer`: replaced intermediate heap-allocated staging `Vec` with a 512-byte
  stack line buffer; full lines are written directly to the writer. ~25% faster.
- `encode_byte`: replaced branchy `match` + `overflowing_add` with a 256-entry `const`
  lookup table. Combined with the above: **+53% throughput** on `encode_buffer`.
- `decode_buffer`: replaced `enumerate` + manual `iter.next()` lookahead with an index
  loop, enabling better compiler codegen on the common fast path. ~25% faster.
- `decode_stream`: hoisted the `line_buf` and `decoded_buf` `Vec` allocations outside the
  per-line loop; buffers are now cleared and reused. ~20% faster end-to-end.

### Changed

- Updated dependencies: `crc32fast` 1.3.2 → 1.5.0, `criterion` 0.4 → 0.5,
  `rand` 0.8 → 0.9.
- Removed `lazy_static` dev-dependency; replaced with `std::sync::LazyLock` (stable
  since Rust 1.80).
- Benchmarks now use `iter_batched` / `iter_with_large_drop` to exclude setup allocations
  from measured time, giving more stable and representative numbers.

---

## [0.2.2] - 2023

### Fixed

- Decoding no longer fails when the `=yend` line is missing `pcrc32`/`crc32` fields;
  absent checksums are now tolerated.

### Performance

- Improved CRC32 calculation speed.
- Improved decode I/O throughput.

### Changed

- Migrated CI from Travis CI and AppVeyor to GitHub Actions.
- Minimum Rust version raised to **1.70.0**.
- Updated dev-dependencies.

---

## [0.2.1] - 2021

### Breaking Changes

- `decode_file` and `decode_stream` now return `Result<Box<Path>, DecodeError>` instead
  of `Result<String, DecodeError>`.
- Internal `unwrap()` calls removed; all errors are now properly propagated via
  `DecodeError` / `EncodeError`.

### Performance

- Improved decode performance.

### Changed

- Migrated to Rust edition 2018 with stricter lint compliance.
- Minimum Rust version raised to **1.42.0**.
- Updated dependencies.

---

## [0.1.1] - 2019

### Fixed

- Fixed absolute paths in `use` statements.

---

## [0.1.0] - 2019

### Breaking Changes

- Public API redesigned around `EncodeOptions` and `DecodeOptions` builder structs,
  replacing the previous free-function API.

### Fixed

- Fixed header parsing for `=ybegin` / `=ypart` / `=yend` lines.
- Fixed asymmetric encode/decode for lines starting with a dot (NNTP dot-stuffing).
- Fixed line length handling.
- Fixed headers in encoded output.

### Added

- Multi-part encoding and decoding support.
- Benchmarks.
- Integration (roundtrip) tests.

### Performance

- Improved encoding performance.
- Improved CRC32 performance.

---

## [0.0.4] - 2018

### Fixed

- Fixed decoding of two consecutive dots at the start of a line.

### Added

- Decode from stream (`decode_stream`).
- CRC32 verification.

### Performance

- Improved encoding performance.

---

## [0.0.3] - 2017

### Added

- Multi-line block support.
- Minimal documentation.
