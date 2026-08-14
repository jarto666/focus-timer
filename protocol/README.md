# Device synchronization protocol

This root owns the language-independent protocol registry and committed golden
fixtures. Rust and TypeScript implementations must encode the same logical
messages to the same canonical bytes.

- [`schema.md`](./schema.md) is the normative protocol 1.0 registry: GATT UUIDs,
  numeric identifiers, constrained-CBOR rules, compatibility policy, bounds,
  structured errors, and BLE framing.
- `fixtures/` will contain the committed canonical byte fixtures introduced by
  OpenSpec task 3.5.

The first command surface is read-oriented: hello, status, bounded session-page
reads, and a volatile clock anchor. It contains no remote timer controls.
