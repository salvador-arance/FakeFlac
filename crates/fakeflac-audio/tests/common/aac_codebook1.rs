// Symphonia
// Copyright (c) 2019-2026 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// The two tables below were taken from symphonia-codec-aac 0.6.1, `src/aac/codebooks.rs`
// (https://github.com/pdeljanov/Symphonia), licensed under MPL-2.0. They are standard data of
// ISO/IEC 14496-3 (spectral Huffman codebook 1). This file is the only MPL-2.0 file of the
// test support; the rest of the repository is MIT.

/// AAC spectral codebook 1 (ISO/IEC 14496-3, spectral Huffman codebook 1): length in bits and code of the quadruple
/// `(w, x, y, z)`, each value in {-1, 0, 1}, at index `27 (w + 1) + 9 (x + 1) + 3 (y + 1) + (z + 1)`.
pub(super) const SPECTRUM_CODEBOOK1_LENS: [u8; 81] = [
    11, 9, 11, 10, 7, 10, 11, 9, 11, 10, 7, 10, 7, 5, 7, 9, 7, 10, 11, 9, 11, 9, 7, 9, 11, 9, 11,
    9, 7, 9, 7, 5, 7, 9, 7, 9, 7, 5, 7, 5, 1, 5, 7, 5, 7, 9, 7, 9, 7, 5, 7, 9, 7, 9, 11, 9, 11, 9,
    7, 9, 11, 9, 11, 10, 7, 9, 7, 5, 7, 9, 7, 10, 11, 9, 11, 10, 7, 9, 11, 9, 11,
];

pub(super) const SPECTRUM_CODEBOOK1_CODES: [u32; 81] = [
    0x7f8, 0x1f1, 0x7fd, 0x3f5, 0x068, 0x3f0, 0x7f7, 0x1ec, 0x7f5, 0x3f1, 0x072, 0x3f4, 0x074,
    0x011, 0x076, 0x1eb, 0x06c, 0x3f6, 0x7fc, 0x1e1, 0x7f1, 0x1f0, 0x061, 0x1f6, 0x7f2, 0x1ea,
    0x7fb, 0x1f2, 0x069, 0x1ed, 0x077, 0x017, 0x06f, 0x1e6, 0x064, 0x1e5, 0x067, 0x015, 0x062,
    0x012, 0x000, 0x014, 0x065, 0x016, 0x06d, 0x1e9, 0x063, 0x1e4, 0x06b, 0x013, 0x071, 0x1e3,
    0x070, 0x1f3, 0x7fe, 0x1e7, 0x7f3, 0x1ef, 0x060, 0x1ee, 0x7f0, 0x1e2, 0x7fa, 0x3f3, 0x06a,
    0x1e8, 0x075, 0x010, 0x073, 0x1f4, 0x06e, 0x3f7, 0x7f6, 0x1e0, 0x7f9, 0x3f2, 0x066, 0x1f5,
    0x7ff, 0x1f7, 0x7f4,
];
