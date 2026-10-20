import { Language, Parser, Query } from "web-tree-sitter";

// Served from public/tree-sitter, which `pnpm grammar` fills from the Zed
// extension: the grammar built to wasm, and its highlight queries.
const BASE = "/tree-sitter";

/** The semantic token type a highlight capture is shown as; captures not listed are left plain. */
const TOKEN_OF: Record<string, string> = {
  comment: "comment",
  "comment.doc": "comment",
  string: "string",
  "string.escape": "string",
  number: "number",
  boolean: "keyword",
  keyword: "keyword",
  operator: "operator",
  function: "function",
  "function.method": "method",
  type: "type",
  constructor: "class",
  constant: "enumMember",
  property: "property",
  variable: "variable",
  "variable.parameter": "parameter",
  "variable.special": "keyword",
  attribute: "decorator",
};

export const LEGEND = Array.from(new Set(Object.values(TOKEN_OF)));

export type Highlighter = {
  legend: string[];
  /** The semantic tokens of `text` in the encoding Monaco reads. */
  tokens(text: string): Uint32Array;
};

export async function loadHighlighter(): Promise<Highlighter> {
  await Parser.init({ locateFile: (file: string) => `${BASE}/${file}` });
  const language = await Language.load(`${BASE}/into.wasm`);
  const parser = new Parser();
  parser.setLanguage(language);
  const query = new Query(language, await (await fetch(`${BASE}/highlights.scm`)).text());

  return {
    legend: LEGEND,
    tokens(text) {
      const tree = parser.parse(text);
      if (!tree) {
        return new Uint32Array();
      }
      // Later captures win, as in tree-sitter's own highlighting.
      const painted = new Int16Array(text.length).fill(-1);
      for (const capture of query.captures(tree.rootNode)) {
        const token = TOKEN_OF[capture.name];
        if (token !== undefined) {
          painted.fill(LEGEND.indexOf(token), capture.node.startIndex, capture.node.endIndex);
        }
      }
      tree.delete();
      return encode(text, painted);
    },
  };
}

/** Runs of one token type within a line, as Monaco's delta-encoded numbers. */
function encode(text: string, painted: Int16Array): Uint32Array {
  const data: number[] = [];
  let line = 0;
  let lineStart = 0;
  let previousLine = 0;
  let previousStart = 0;
  let index = 0;
  while (index < text.length) {
    if (text[index] === "\n") {
      line += 1;
      lineStart = index + 1;
      index += 1;
      continue;
    }
    const type = painted[index];
    if (type < 0) {
      index += 1;
      continue;
    }
    let end = index;
    while (end < text.length && text[end] !== "\n" && painted[end] === type) {
      end += 1;
    }
    const start = index - lineStart;
    const deltaLine = line - previousLine;
    data.push(deltaLine, deltaLine === 0 ? start - previousStart : start, end - index, type, 0);
    previousLine = line;
    previousStart = start;
    index = end;
  }
  return new Uint32Array(data);
}
