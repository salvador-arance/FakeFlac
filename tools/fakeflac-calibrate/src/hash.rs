//! Stable hashes (inputs.md section 4, generation.md section 3). Not std's `DefaultHasher`.

/// FNV-1a, 64 bits: offset basis `0xcbf29ce484222325`, prime `0x100000001b3`, wrapping.
pub fn fnv1a64(_bytes: &[u8]) -> u64 {
    todo!("RP-11: inputs.md section 4")
}

/// MurmurHash3 64-bit finaliser.
pub fn fmix64(_k: u64) -> u64 {
    todo!("RP-11: inputs.md section 4")
}

/// 16 lower-case hex digits of `fmix64(fnv1a64(bytes))` (recipe ids, fingerprint ids, params hashes).
pub fn id16(_bytes: &[u8]) -> String {
    todo!("RP-11: generation.md sections 3 and 4.1, evaluation.md section 9")
}
