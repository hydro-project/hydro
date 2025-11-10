

lattices/src/tombstone.rs
// For Vec<u8> items
impl Extend<Vec<u8>> for FstTombstoneSet<Vec<u8>> {
    fn extend<T: IntoIterator<Item = Vec<u8>>>(&mut self, iter: T) {
        let mut keys: Vec<_> = self.fst.stream().into_strs().unwrap();
Copilot AI
14 minutes ago
Converting Vec items to strings via into_strs().unwrap() will panic if the FST contains non-UTF8 byte sequences. Since this is a FstTombstoneSet<Vec<u8>>, arbitrary byte sequences are expected. Use the byte-oriented API instead, such as self.fst.stream() directly, or handle the conversion more safely.

Copilot uses AI. Check for mistakes.
@jhellerstein	Reply...
lattices/src/tombstone.rs
Comment on lines +190 to +194
        let mut keys: Vec<_> = self.fst.stream().into_strs().unwrap();
        keys.extend(
            iter.into_iter()
                .map(|v| String::from_utf8_lossy(&v).into_owned()),
        );
Copilot AI
14 minutes ago
The implementation converts Vec to String using from_utf8_lossy, which can lose data through replacement characters for invalid UTF-8. This creates inconsistency: existing FST data is extracted as strings (line 190), new data is lossily converted to strings (line 193), but the API accepts Vec<u8>. This means lookups may fail if the inserted bytes contained invalid UTF-8, as the lossy conversion would have changed them. Consider either: (1) using byte-oriented FST operations throughout, or (2) validating UTF-8 upfront and rejecting invalid sequences.

Copilot uses AI. Check for mistakes.
