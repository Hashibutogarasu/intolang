use parser::LanguageLexeme;

use super::layout::*;
use super::model::Inex;

fn push_line(bytes: &mut Vec<u8>, text: &str) {
    bytes.extend_from_slice(text.as_bytes());
    bytes.push(b'\n');
}

impl From<&Inex> for Vec<u8> {
    fn from(inex: &Inex) -> Self {
        let mut bytes = Vec::new();
        push_line(&mut bytes, SHEBANG);
        push_line(&mut bytes, &format!("{MAGIC} {VERSION}"));
        push_line(&mut bytes, &format!("{NAME}:{FIELD}{}", inex.header.name));
        push_line(&mut bytes, &format!("{ALIAS}:{FIELD}{}", inex.header.alias));
        push_line(
            &mut bytes,
            &format!("{PACKAGE_VERSION}:{FIELD}{}", inex.header.version),
        );
        push_line(
            &mut bytes,
            &format!("{GENERATOR}:{FIELD}{}", inex.header.generator),
        );
        for source in &inex.sources {
            push_line(
                &mut bytes,
                &format!(
                    "{SOURCE}:{FIELD}{}{FIELD}{}{FIELD}{}{FIELD}{}",
                    source.alias,
                    LanguageLexeme::from(source.language),
                    source.path,
                    source.dependencies.join(",")
                ),
            );
        }
        push_line(&mut bytes, "");
        push_line(
            &mut bytes,
            &format!("{MESSAGE}{FIELD}{}", inex.message.len()),
        );
        bytes.extend_from_slice(inex.message.as_bytes());
        bytes.push(b'\n');
        for file in &inex.files {
            push_line(
                &mut bytes,
                &format!("{FILE}{FIELD}{}{FIELD}{}", file.contents.len(), file.path),
            );
            bytes.extend_from_slice(&file.contents);
            bytes.push(b'\n');
        }
        bytes
    }
}
