# Prototypes

Single-file experiments for design questions. They are not part of the
workspace and nothing depends on them.

Run one with:

```sh
rustc --edition 2024 --test prototypes/slot_proto.rs -o target/slot_proto && target/slot_proto
```

- `slot_proto.rs`: companion slots chosen by a field's type (filters) and
  overridden per field (a counter in a patch).
- `views_proto.rs`: a value view and a slot view over every representation
  (struct, partial, patch, staged value), and one `Debug` hook on top of the
  slot view that hides a field. Shows why the slot view can't be a lifetime
  GAT.
