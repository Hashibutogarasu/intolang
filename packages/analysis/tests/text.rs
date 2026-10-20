use std::path::Path;

use analysis::{LineIndex, path_to_uri, uri_to_path};
use ipc::Position;
use types::Span;

fn span(start: usize, end: usize) -> Span {
    Span {
        line: 1,
        end_line: 1,
        start,
        end,
    }
}

#[test]
fn offsets_and_positions_convert_both_ways() {
    let index = LineIndex::new("ab\n日本\n\nend");
    assert_eq!(index.position(0), Position::new(0, 0));
    assert_eq!(index.position(3), Position::new(1, 0));
    assert_eq!(index.position(5), Position::new(1, 2));
    assert_eq!(index.position(6), Position::new(2, 0));
    assert_eq!(index.position(100), Position::new(3, 3));
    assert_eq!(index.offset(Position::new(1, 1)), 4);
    assert_eq!(index.offset(Position::new(1, 99)), 5);
    assert_eq!(index.offset(Position::new(99, 0)), 7);
    assert_eq!(index.range(span(3, 5)).end, Position::new(1, 2));
}

#[test]
fn a_line_range_skips_the_blanks_and_the_line_break() {
    let index = LineIndex::new("first\r\n    second  \r\n");
    let range = index.line_range(2);
    assert_eq!(range.start, Position::new(1, 4));
    assert_eq!(range.end, Position::new(1, 12));
    assert_eq!(index.end(), Position::new(2, 0));
}

#[test]
fn uris_round_trip_through_percent_encoding() {
    let path = Path::new("/home/user/My Project/日本語.into");
    let uri = path_to_uri(path);
    assert_eq!(
        uri,
        "file:///home/user/My%20Project/%E6%97%A5%E6%9C%AC%E8%AA%9E.into"
    );
    assert_eq!(uri_to_path(&uri).as_deref(), Some(path));
    assert_eq!(uri_to_path("https://example.com"), None);
}
