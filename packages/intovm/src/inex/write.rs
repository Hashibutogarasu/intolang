use bundle::Entry;
use std::collections::HashMap;

use super::layout::*;
use super::model::Inex;

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put_text(bytes: &mut Vec<u8>, text: &str) {
    put_u32(bytes, text.len() as u32);
    bytes.extend_from_slice(text.as_bytes());
}

fn digest_bytes(hash: &str) -> [u8; DIGEST_LENGTH] {
    let mut digest = [0u8; DIGEST_LENGTH];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hash[index * 2..index * 2 + 2], 16).unwrap_or_default();
    }
    digest
}

impl From<&Inex> for Vec<u8> {
    fn from(inex: &Inex) -> Self {
        let mut table = Vec::new();
        let mut data = Vec::new();
        let mut offsets: HashMap<&str, u64> = HashMap::new();
        let mut count = 0u32;
        for (path, entry) in inex.bundle.entries() {
            count += 1;
            put_text(&mut table, path.as_str());
            match entry {
                Entry::Directory => {
                    table.push(DIRECTORY);
                    put_u32(&mut table, 0);
                    put_u64(&mut table, 0);
                    put_u64(&mut table, 0);
                    table.extend_from_slice(&[0u8; DIGEST_LENGTH]);
                }
                Entry::File { size, hash, mode } => {
                    let offset = *offsets.entry(hash.as_str()).or_insert_with(|| {
                        let offset = data.len() as u64;
                        data.extend_from_slice(inex.bundle.read(path.as_str()).unwrap_or_default());
                        offset
                    });
                    table.push(FILE);
                    put_u32(&mut table, *mode);
                    put_u64(&mut table, *size);
                    put_u64(&mut table, offset);
                    table.extend_from_slice(&digest_bytes(hash));
                }
            }
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(SHEBANG);
        bytes.extend_from_slice(MAGIC);
        put_u32(&mut bytes, VERSION);
        put_text(&mut bytes, &inex.generator);
        put_u32(&mut bytes, count);
        bytes.extend_from_slice(&table);
        bytes.extend_from_slice(&data);
        bytes
    }
}
