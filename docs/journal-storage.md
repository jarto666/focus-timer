# Device journal storage budget

This note records the evidence behind the first fixed journal capacity. It is
specific to the current ESP32-C3 firmware build and must be revisited if the
partition table or record schema changes.

## Measured partition

The ESP-IDF 5.5.3 build emits
`device/target/riscv32imc-esp-espidf/debug/partition-table.bin`. Decoding that
artifact with ESP-IDF's `gen_esp32part.py` reports:

```text
nvs,data,nvs,0x9000,24K
phy_init,data,phy,0xf000,4K
factory,app,factory,0x10000,1M
```

The checked ESP-IDF NVS constants use 4 KiB pages, 32-byte entries, and 126
entries per page. ESP-IDF's page manager excludes one complete page from
`available_entries` for garbage collection. The 24 KiB partition therefore has
six physical pages and at most `5 * 126 = 630` live available entries.

## Record and entry budget

The host-tested version-1 codecs measure these maxima:

| Value | Encoded bytes | Worst-case NVS entries |
| --- | ---: | ---: |
| Stable device identity | 23 | 3 |
| Metadata copy A or B | 27 | 3 each |
| Journal slot | 122 | 6 each |

An ESP-IDF v2 blob uses one `BLOB_DATA` metadata entry, one 32-byte entry per
rounded data block, and one `BLOB_IDX` entry. A maximum 122-byte slot therefore
uses `1 + ceil(122 / 32) + 1 = 6` entries.

The selected capacity is **64 records**. Its worst-case live sync footprint is:

```text
focus_sync namespace       1
identity                   3
metadata A + B             6
64 maximum slots         384
-----------------------------
total                    394 entries
```

The existing `focus_timer` namespace and maximum 35-byte settings blob consume
about five more live entries. The combined worst-case live set is therefore
about 399 of 630 available entries, leaving roughly 231 entries (37%) for NVS
rewrite/GC headroom and stack-owned keys. This supports the initial capacity of
64 without changing or erasing the existing settings namespace.

Capacity remains a compile-time bound (`focus_sync::JOURNAL_CAPACITY`). The
warning-free `journal-fill-diagnostic` build appends 65 records and checks that
a cursor at zero reports a gap after the 64-slot eviction. Physical execution
of that diagnostic and live NVS statistics still gate final acceptance.

## Reproduce

From the repository root after a default firmware build:

```sh
python3 "$IDF_PATH/components/partition_table/gen_esp32part.py" \
  device/target/riscv32imc-esp-espidf/debug/partition-table.bin
source "$HOME/.cargo/env"
cargo test --manifest-path device/Cargo.toml -p focus-sync maximum_record_size
```
