//! Tests of the partial-DDS codec.

use super::*;

/// A 16×8 DXT5 texture with 3 mips.
fn dxt5() -> Vec<u8> {
    let mut header = vec![0u8; HEADER];
    header[0..4].copy_from_slice(b"DDS ");
    header[4..8].copy_from_slice(&124u32.to_le_bytes());
    header[12..16].copy_from_slice(&8u32.to_le_bytes());
    header[16..20].copy_from_slice(&16u32.to_le_bytes());
    header[28..32].copy_from_slice(&3u32.to_le_bytes());
    header[84..88].copy_from_slice(b"DXT5");
    // Mips: 4×2, 2×1 and 1×1 blocks of 16 bytes; repetitive so LZ4 can shrink it.
    let body: Vec<u8> = (0..(128 + 32 + 16) as u32)
        .map(|i| (i / 32) as u8)
        .collect();
    [header, body].concat()
}

#[test]
fn mip_sizes_and_raw_round_trip() {
    let file = dxt5();
    let dds = info(&file).unwrap();
    assert_eq!(
        (0..3).map(|i| mip_size(&dds, i)).collect::<Vec<_>>(),
        [128, 32, 16]
    );
    let (stored, m) = encode_raw(&file, 15).unwrap();
    assert_eq!(m, [128, 32, 16, 0]);
    assert_eq!(stored.len(), file.len(), "raw chunks keep the size");
    assert_eq!(
        decode(&stored, file.len()).unwrap()[HEADER..],
        file[HEADER..]
    );
    let record = template(&stored[..HEADER]);
    assert_eq!((&record[0..4], record[124]), (&b"DDS "[..], 15));
    assert!(record[32..76].iter().all(|b| *b == 0));
}

#[test]
fn decodes_lz4_chunks() {
    let file = dxt5();
    let dds = info(&file).unwrap();
    let mip0 = &file[HEADER..HEADER + 128];
    let packed = lz4_flex::block::compress(mip0);
    assert!(packed.len() < 128);
    let mut stored = file[..HEADER].to_vec();
    [packed.len() as u32, 32, 16, 0]
        .iter()
        .enumerate()
        .for_each(|(i, v)| {
            stored[32 + i * 4..36 + i * 4].copy_from_slice(&v.to_le_bytes());
        });
    stored.extend(&packed);
    stored.extend(&file[HEADER + 128..]);
    assert_eq!(
        decode(&stored, file.len()).unwrap()[HEADER..],
        file[HEADER..]
    );
    assert_eq!(chunk_count(&dds), 3);
}
