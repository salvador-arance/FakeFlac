//! Stable hashes (inputs.md section 4, generation.md section 3). Not std's `DefaultHasher`.

/// FNV-1a, 64 bits: offset basis `0xcbf29ce484222325`, prime `0x100000001b3`, wrapping.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// MurmurHash3 64-bit finaliser.
pub fn fmix64(k: u64) -> u64 {
    let mut k = k;
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51_afd7_ed55_8ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    k ^= k >> 33;
    k
}

/// 16 lower-case hex digits of `fmix64(fnv1a64(bytes))` (recipe ids, fingerprint ids, params hashes).
pub fn id16(bytes: &[u8]) -> String {
    format!("{:016x}", fmix64(fnv1a64(bytes)))
}
