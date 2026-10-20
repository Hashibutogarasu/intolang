use bundle::{Bundle, Entry};
use errors::{Context, Error, Errors, Result};

use super::layout::*;
use super::model::Inex;

/// A cursor over the bytes of an `.inex` file.
struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| errors::Error::from(Errors::InexTruncated))?;
        let taken = &self.bytes[self.position..end];
        self.position = end;
        Ok(taken)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32> {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64> {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(bytes))
    }

    fn text(&mut self) -> Result<String> {
        let length = self.u32()? as usize;
        String::from_utf8(self.take(length)?.to_vec())
            .map_err(|_| errors::Error::from(Errors::InexNameNotText))
    }

    fn rest(&self) -> &'a [u8] {
        &self.bytes[self.position..]
    }
}

struct Raw {
    path: String,
    kind: u8,
    mode: u32,
    size: u64,
    offset: u64,
    digest: [u8; DIGEST_LENGTH],
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl TryFrom<&[u8]> for Inex {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let mut bytes = bytes;
        if bytes.starts_with(b"#!") {
            let end = bytes
                .iter()
                .position(|byte| *byte == b'\n')
                .ok_or_else(|| errors::Error::from(Errors::NotInex))?;
            bytes = &bytes[end + 1..];
        }
        let Some(rest) = bytes.strip_prefix(MAGIC) else {
            return Err(errors::Error::from(Errors::NotInex));
        };
        if let Some(text) = rest.strip_prefix(b" ") {
            let digits: String = text
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .map(|byte| char::from(*byte))
                .collect();
            let version: u32 = digits
                .parse()
                .map_err(|_| errors::Error::from(Errors::NotInex))?;
            return Err(errors::Error::from(Errors::InexVersionUnsupported {
                version: version.to_string(),
            }));
        }
        let mut reader = Reader {
            bytes: rest,
            position: 0,
        };
        let version = reader
            .u32()
            .map_err(|_| errors::Error::from(Errors::NotInex))?;
        if version != VERSION {
            return Err(errors::Error::from(Errors::InexVersionUnsupported {
                version: version.to_string(),
            }));
        }
        let generator = reader.text()?;
        let count = reader.u32()?;
        let mut table = Vec::new();
        for _ in 0..count {
            let path = reader.text()?;
            let kind = reader.u8()?;
            let mode = reader.u32()?;
            let size = reader.u64()?;
            let offset = reader.u64()?;
            let mut digest = [0u8; DIGEST_LENGTH];
            digest.copy_from_slice(reader.take(DIGEST_LENGTH)?);
            table.push(Raw {
                path,
                kind,
                mode,
                size,
                offset,
                digest,
            });
        }
        let data = reader.rest();
        let mut bundle = Bundle::new();
        for raw in table {
            match raw.kind {
                DIRECTORY => bundle
                    .add_directory(&raw.path)
                    .with_context(|| format!("The inex file has a bad entry `{}`", raw.path))?,
                FILE => {
                    let range = usize::try_from(raw.offset)
                        .ok()
                        .zip(usize::try_from(raw.size).ok())
                        .and_then(|(offset, size)| Some(offset..offset.checked_add(size)?))
                        .filter(|range| range.end <= data.len())
                        .ok_or_else(|| {
                            errors::Error::from(Errors::InexEntryOutside {
                                path: raw.path.to_string(),
                            })
                        })?;
                    bundle
                        .add_file(&raw.path, &data[range], raw.mode)
                        .with_context(|| format!("The inex file has a bad entry `{}`", raw.path))?;
                    let matches = matches!(
                        bundle.get(&raw.path),
                        Some(Entry::File { hash, .. }) if *hash == hex(&raw.digest)
                    );
                    if !matches {
                        return Err(errors::Error::from(Errors::InexHashMismatch {
                            path: raw.path.to_string(),
                        }));
                    }
                }
                other => {
                    return Err(errors::Error::from(Errors::InexUnknownKind {
                        kind: other.to_string(),
                    }));
                }
            }
        }
        Ok(Inex { generator, bundle })
    }
}
