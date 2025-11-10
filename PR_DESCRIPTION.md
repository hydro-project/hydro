## Add RoaringBitmap and FST Tombstone Storage for Lattices

This PR adds efficient, compressed tombstone storage implementations for `SetUnionWithTombstones` and `MapUnionWithTombstones` lattices.

### Motivation

The existing TODO comment in `set_union_with_tombstones.rs` suggested opportunities for "cool storage tricks" with different tombstone implementations. This PR implements two highly efficient storage strategies:

1. **RoaringBitmap** - For integer keys (u64)
2. **FST (Finite State Transducer)** - For string and byte array keys

Both provide significant space savings over `HashSet` while maintaining or improving performance.

### Changes

#### New Module: `lattices/src/tombstone.rs`
- `RoaringTombstoneSet` - Bitmap compression for u64 keys
- `FstTombstoneSet<T>` - FST compression for String and Vec<u8> keys
- Shared implementations used by both set and map lattices

#### Enhanced: `set_union_with_tombstones.rs`
- Specialized merge implementations using bitmap OR and FST union
- Type aliases: `SetUnionWithTombstonesRoaring`, `SetUnionWithTombstonesFstString`, `SetUnionWithTombstonesFstBytes`
- Comprehensive documentation with performance tables and examples
- 11 tests covering basic ops, efficiency, and lattice properties (idempotency, commutativity)

#### Enhanced: `map_union_with_tombstones.rs`
- Specialized merge implementations for all three tombstone types
- Type aliases: `MapUnionWithTombstonesRoaring`, `MapUnionWithTombstonesFstString`, `MapUnionWithTombstonesFstBytes`
- Same documentation and testing approach as sets
- 8 tests covering all implementations

### Performance Characteristics

| Implementation | Space Efficiency | Merge Speed | Lookup Speed | False Positives |
|----------------|------------------|-------------|--------------|-----------------|
| RoaringBitmap  | Excellent        | Excellent   | Excellent    | None            |
| FST            | Very Good        | Good        | Very Good    | None            |
| HashSet        | Poor             | Good        | Excellent    | None            |

### Usage Examples

```rust
// For u64 integer keys
let mut set = SetUnionWithTombstonesRoaring::new_from(
    HashSet::from([1u64, 2, 3]), 
    RoaringTombstoneSet::new()
);

// For String keys
let mut map = MapUnionWithTombstonesFstString::new_from(
    HashMap::from([("key".to_string(), value)]), 
    FstTombstoneSet::new()
);
```

### Testing

- ✅ All 121 existing tests pass
- ✅ 11 new tests for set tombstones
- ✅ 8 new tests for map tombstones
- ✅ Tests cover basic operations, merge efficiency, and lattice properties
- ✅ Clippy clean with `-D warnings`

### Dependencies Added

- `roaring = "0.10"` - RoaringBitmap implementation
- `fst = "0.4"` - FST implementation

### Breaking Changes

None. This is a purely additive change. All existing code continues to work unchanged.

### Documentation

- Comprehensive module-level docs explaining when to use each implementation
- Performance characteristics tables
- Usage examples for each type alias
- Performance notes (e.g., FST `extend()` rebuilds the structure)
