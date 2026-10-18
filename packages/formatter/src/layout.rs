use types::{Keyword, Punct};

use crate::options::FormatOptions;
use crate::piece::{Kind, Piece};
use crate::spacing::needs_space;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Brace {
    Block,
    Enum,
}

struct Writer {
    output: String,
    unit: String,
    options: FormatOptions,
    depth: usize,
    parens: usize,
    saved_parens: Vec<usize>,
    braces: Vec<Brace>,
    in_attribute: bool,
    previous: Option<Piece>,
}

impl Writer {
    fn at_line_start(&self) -> bool {
        self.output.is_empty() || self.output.ends_with('\n')
    }

    fn end_line(&mut self) {
        if !self.at_line_start() {
            self.output.push('\n');
        }
    }

    fn blank_lines(&mut self, wanted: usize) {
        if self.output.is_empty() {
            return;
        }
        let present = self
            .output
            .bytes()
            .rev()
            .take_while(|byte| *byte == b'\n')
            .count()
            .saturating_sub(1);
        for _ in present..wanted.min(self.options.max_blank_lines) {
            self.output.push('\n');
        }
    }

    fn indent(&mut self, levels: usize) {
        for _ in 0..levels {
            self.output.push_str(&self.unit);
        }
    }

    fn continuation(&self) -> usize {
        self.depth + self.parens.min(1) * self.options.continuation_levels
    }

    fn gap_before(&mut self, prev: &Piece, piece: &Piece) {
        if piece.line > prev.end_line + 1 {
            self.blank_lines(piece.line - prev.end_line - 1);
        }
    }
}

fn opens_enum(pieces: &[Piece], index: usize) -> bool {
    pieces[..index]
        .iter()
        .rev()
        .take_while(|piece| {
            !matches!(
                piece.kind,
                Kind::Punct(Punct::Semicolon | Punct::LeftBrace | Punct::RightBrace)
            )
        })
        .any(|piece| piece.is_keyword(Keyword::Enum))
}

pub(crate) fn layout(pieces: &[Piece], options: FormatOptions) -> String {
    let mut writer = Writer {
        output: String::new(),
        unit: options.indent.unit(),
        options,
        depth: 0,
        parens: 0,
        saved_parens: Vec::new(),
        braces: Vec::new(),
        in_attribute: false,
        previous: None,
    };

    for (index, piece) in pieces.iter().enumerate() {
        let closing = piece.is(Punct::RightBrace);
        if closing {
            writer.end_line();
            writer.depth = writer.depth.saturating_sub(1);
            writer.braces.pop();
        }

        let previous = writer.previous.take();
        if piece.kind == Kind::Comment {
            match &previous {
                Some(prev) if !writer.at_line_start() && prev.end_line == piece.line => {
                    writer.output.push(' ');
                }
                _ => {
                    writer.end_line();
                    if let Some(prev) = &previous {
                        writer.gap_before(prev, piece);
                    }
                    let levels = writer.continuation();
                    writer.indent(levels);
                }
            }
        } else if writer.at_line_start() {
            if let Some(prev) = &previous {
                writer.gap_before(prev, piece);
            }
            let levels = writer.continuation();
            writer.indent(levels);
        } else if let Some(prev) = &previous {
            if writer.parens > 0 && piece.line > prev.end_line {
                let nesting = if piece.is(Punct::RightParen) {
                    writer.parens - 1
                } else {
                    writer.parens
                };
                writer.output.push('\n');
                let levels = writer.depth + nesting * writer.options.continuation_levels;
                writer.indent(levels);
            } else if needs_space(prev, piece) {
                writer.output.push(' ');
            }
        }

        writer.output.push_str(&piece.text);

        let mut break_after = false;
        match piece.kind {
            Kind::Comment => break_after = true,
            Kind::Punct(Punct::LeftParen) => writer.parens += 1,
            Kind::Punct(Punct::RightParen) => writer.parens = writer.parens.saturating_sub(1),
            Kind::Punct(Punct::LeftBracket) => {
                if previous.as_ref().is_some_and(|prev| prev.is(Punct::Hash)) {
                    writer.in_attribute = true;
                }
            }
            Kind::Punct(Punct::RightBracket) => {
                if writer.in_attribute {
                    writer.in_attribute = false;
                    break_after = true;
                }
            }
            Kind::Punct(Punct::LeftBrace) => {
                let brace = if opens_enum(pieces, index) {
                    Brace::Enum
                } else {
                    Brace::Block
                };
                writer.braces.push(brace);
                writer.saved_parens.push(writer.parens);
                writer.parens = 0;
                writer.depth += 1;
                break_after = true;
            }
            Kind::Punct(Punct::RightBrace) => {
                writer.parens = writer.saved_parens.pop().unwrap_or(0);
                let joins_else = pieces
                    .get(index + 1)
                    .is_some_and(|next| next.is_keyword(Keyword::Else));
                break_after = !joins_else && writer.parens == 0;
            }
            Kind::Punct(Punct::Semicolon) => break_after = writer.parens == 0,
            Kind::Punct(Punct::Comma) => {
                break_after = writer.parens == 0
                    && !writer.in_attribute
                    && writer.braces.last() == Some(&Brace::Enum);
            }
            _ => {}
        }
        let trailing_comment = pieces
            .get(index + 1)
            .is_some_and(|next| next.kind == Kind::Comment && next.line == piece.end_line);
        if break_after && !trailing_comment {
            writer.output.push('\n');
        }
        writer.previous = Some(piece.clone());
    }

    writer.end_line();
    writer.output
}
