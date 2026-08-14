# Device synchronization protocol

This root owns the language-independent protocol registry and committed golden
fixtures. Rust and TypeScript implementations must encode the same logical
messages to the same canonical bytes.

- [`schema.md`](./schema.md) is the normative protocol 1.0 registry: GATT UUIDs,
  numeric identifiers, constrained-CBOR rules, compatibility policy, bounds,
  structured errors, and BLE framing.
- [`fixtures/logical-v1.txt`](./fixtures/logical-v1.txt) is the shared,
  append-only set of canonical protocol 1.x bytes. Rust and TypeScript tests
  both reproduce and decode every line.

The first command surface is read-oriented: hello, status, bounded session-page
reads, and a volatile clock anchor. It contains no remote timer controls.

## Compatibility rule

Committed numeric identifiers and fixture bytes are compatibility artifacts,
not snapshots that may be reformatted in place.

- A compatible 1.x addition assigns new identifiers, keeps every existing
  meaning and byte sequence, and appends fixtures without rewriting old lines.
- Reassigning an identifier, changing a required field or representation, or
  changing the interpretation of committed bytes requires an explicit new
  protocol major, a new registry version, and new fixture set. Historical files
  remain in the repository so supported decoders keep proving readability.
- A fixture correction that would change bytes is itself a protocol change; it
  must follow the same version decision rather than silently replacing history.

Run the compatibility gate from the repository root:

```sh
./scripts/check-protocol.sh
# or
pnpm check:protocol
```

The gate fails if either language implementation or the shared fixture registry
is absent, then runs Rust formatting, strict Clippy, Rust fixture/tests, and the
TypeScript format/lint/type/test suite.
