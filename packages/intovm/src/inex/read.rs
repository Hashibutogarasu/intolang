use anyhow::{Error, Result, anyhow, bail};
use parser::LanguageLexeme;

use super::layout::*;
use super::model::{Inex, InexFile, InexHeader, InexSource};

/// A cursor over the bytes of an `.inex` file.
struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn line(&mut self) -> Result<&'a str> {
        let rest = &self.bytes[self.position..];
        let end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| anyhow!("The inex file is truncated"))?;
        self.position += end + 1;
        std::str::from_utf8(&rest[..end]).map_err(|_| anyhow!("The inex file has a broken line"))
    }

    fn block(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self.position + length;
        if end + 1 > self.bytes.len() || self.bytes[end] != b'\n' {
            bail!("The inex file is truncated");
        }
        let block = &self.bytes[self.position..end];
        self.position = end + 1;
        Ok(block)
    }

    fn at_end(&self) -> bool {
        self.position >= self.bytes.len()
    }
}

fn field<'a>(line: &'a str, key: &str) -> Result<&'a str> {
    line.strip_prefix(key)
        .and_then(|rest| rest.strip_prefix(':'))
        .and_then(|rest| rest.strip_prefix(FIELD))
        .ok_or_else(|| anyhow!("The inex header lacks `{key}`"))
}

fn source(line: &str) -> Result<InexSource> {
    let mut parts = field(line, SOURCE)?.splitn(4, FIELD);
    let (Some(alias), Some(language), Some(path), Some(dependencies)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        bail!("The inex source line is malformed");
    };
    Ok(InexSource {
        alias: alias.to_owned(),
        language: language.parse::<LanguageLexeme>()?.into(),
        path: path.to_owned(),
        dependencies: dependencies
            .split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect(),
    })
}

impl TryFrom<&[u8]> for Inex {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let mut reader = Reader { bytes, position: 0 };
        let mut first = reader.line().map_err(|_| anyhow!("Not an inex file"))?;
        if first.starts_with("#!") {
            first = reader.line().map_err(|_| anyhow!("Not an inex file"))?;
        }
        let Some(version) = first
            .strip_prefix(MAGIC)
            .and_then(|rest| rest.strip_prefix(' '))
        else {
            bail!("Not an inex file");
        };
        let version: u32 = version.parse().map_err(|_| anyhow!("Not an inex file"))?;
        if version != VERSION {
            bail!("inex version {version} is not supported");
        }
        let header = InexHeader {
            name: field(reader.line()?, NAME)?.to_owned(),
            alias: field(reader.line()?, ALIAS)?.to_owned(),
            version: field(reader.line()?, PACKAGE_VERSION)?.to_owned(),
            generator: field(reader.line()?, GENERATOR)?.to_owned(),
        };
        let mut sources = Vec::new();
        loop {
            let line = reader.line()?;
            if line.is_empty() {
                break;
            }
            sources.push(source(line)?);
        }
        let line = reader.line()?;
        let length = line
            .strip_prefix(MESSAGE)
            .and_then(|rest| rest.strip_prefix(FIELD))
            .ok_or_else(|| anyhow!("The inex file has no message"))?;
        let message = String::from_utf8(reader.block(length.parse()?)?.to_vec())
            .map_err(|_| anyhow!("The inex message is not text"))?;
        let mut files = Vec::new();
        while !reader.at_end() {
            let line = reader.line()?;
            let mut parts = line
                .strip_prefix(FILE)
                .and_then(|rest| rest.strip_prefix(FIELD))
                .ok_or_else(|| anyhow!("The inex file has an unknown section"))?
                .splitn(2, FIELD);
            let (Some(length), Some(path)) = (parts.next(), parts.next()) else {
                bail!("The inex file section is malformed");
            };
            files.push(InexFile {
                path: path.to_owned(),
                contents: reader.block(length.parse()?)?.to_vec(),
            });
        }
        Ok(Inex {
            header,
            message,
            sources,
            files,
        })
    }
}
