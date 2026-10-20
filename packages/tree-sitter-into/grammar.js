/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

const PRIMITIVE_TYPES = [
  'Boolean',
  'String',
  'Path',
  'Char',
  'Int8',
  'Int16',
  'Int32',
  'Int64',
  'Float',
  'Float32',
  'Float64',
  'Void',
  'Pointer',
];

const MODIFIERS = ['pub', 'static', 'abstract', 'default', 'override'];

const PREC = {
  assign: 1,
  or: 2,
  and: 3,
  compare: 4,
  bit_or: 5,
  bit_xor: 6,
  bit_and: 7,
  shift: 8,
  add: 9,
  multiply: 10,
  cast: 11,
  unary: 12,
  call: 13,
  member: 14,
};

/**
 * @param {RuleOrLiteral} rule
 * @returns {ChoiceRule}
 */
function commaSep(rule) {
  return optional(commaSep1(rule));
}

/**
 * @param {RuleOrLiteral} rule
 * @returns {SeqRule}
 */
function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}

module.exports = grammar({
  name: 'into',

  extras: $ => [/\s/, $.doc_comment, $.line_comment],

  word: $ => $.identifier,

  supertypes: $ => [$._expression, $._type],

  conflicts: $ => [
    [$._expression, $.lambda_parameter],
  ],

  rules: {
    source_file: $ => repeat($._item),

    _item: $ =>
      choice(
        $.import_statement,
        $.export_statement,
        $.class_declaration,
        $.function_declaration,
        $.event_declaration,
        $.enum_declaration,
        $.macro_declaration,
        $.init_block,
        $._statement,
      ),

    import_statement: $ => seq('import', $.string, optional(';')),
    export_statement: $ => seq('export', $.string, optional(';')),

    attribute_item: $ =>
      seq(
        '#',
        '[',
        field('name', $.identifier),
        optional(seq('(', commaSep($._expression), ')')),
        ']',
      ),

    modifiers: _ => repeat1(choice(...MODIFIERS)),

    event_type_arguments: $ => seq('<', commaSep1($._type), '>'),

    type_parameters: $ => seq('<', commaSep1(alias($.identifier, $.type_identifier)), '>'),

    class_declaration: $ =>
      seq(
        $.modifiers,
        field('name', alias($.identifier, $.type_identifier)),
        optional($.type_parameters),
        '(',
        ')',
        optional(seq(':', field('parent', $._type))),
        $.class_body,
      ),

    class_body: $ =>
      seq(
        '{',
        repeat(
          choice(
            $.attribute_item,
            $.field_declaration,
            $.function_declaration,
            $.event_declaration,
          ),
        ),
        '}',
      ),

    field_declaration: $ =>
      seq(
        $.modifiers,
        'into',
        choice(
          seq(':', field('type', $._type), field('name', $.identifier)),
          field('name', $.identifier),
        ),
        optional(seq('=', field('value', $._expression))),
        optional(';'),
      ),

    function_declaration: $ =>
      seq(
        optional($.modifiers),
        optional(alias(choice('c', 'cpp'), $.foreign)),
        'fun',
        field('name', $.identifier),
        optional($.type_parameters),
        $.parameters,
        optional(seq(':', field('return_type', $._type))),
        choice($.block, ';'),
      ),

    parameters: $ => seq('(', commaSep($.parameter), ')'),

    parameter: $ =>
      seq(
        field('name', $.identifier),
        ':',
        field('type', $._type),
        optional(seq('=', field('default', $._expression))),
      ),

    event_declaration: $ =>
      seq(
        optional($.modifiers),
        'event',
        field('name', $.identifier),
        optional($.event_type_arguments),
        $.parameters,
        $.block,
      ),

    enum_declaration: $ =>
      seq(
        optional($.modifiers),
        'enum',
        field('name', alias($.identifier, $.type_identifier)),
        '{',
        commaSep($.enum_variant),
        optional(','),
        '}',
      ),

    enum_variant: $ =>
      seq(
        field('name', $.identifier),
        optional(seq('=', field('value', $._expression))),
      ),

    macro_declaration: $ =>
      seq(
        optional($.modifiers),
        'macro',
        field('name', $.identifier),
        '{',
        repeat($.attribute_item),
        '}',
      ),

    init_block: $ => seq('init', $.block),

    block: $ => seq('{', repeat($._statement), '}'),

    _statement: $ =>
      choice(
        $.variable_declaration,
        $.if_statement,
        $.while_statement,
        $.break_statement,
        $.return_statement,
        $.throw_statement,
        $.unload_statement,
        $.attribute_item,
        $.block,
        $.expression_statement,
      ),

    variable_declaration: $ =>
      seq(
        optional('const'),
        optional(choice('now', 'late')),
        'into',
        optional('*'),
        field('name', $.identifier),
        optional(seq(':', field('type', $._type))),
        optional(seq('=', field('value', $._expression))),
        optional(';'),
      ),

    if_statement: $ =>
      prec.right(
        seq(
          'if',
          field('condition', $._expression),
          field('consequence', $.block),
          optional(
            seq('else', field('alternative', choice($.if_statement, $.block))),
          ),
        ),
      ),

    while_statement: $ =>
      seq('while', field('condition', $._expression), $.block),

    break_statement: $ => seq('break', optional(';')),
    return_statement: $ =>
      prec.right(seq('return', optional($._expression), optional(';'))),
    throw_statement: $ => seq('throw', $._expression, optional(';')),
    unload_statement: $ => seq('unload', $.identifier, optional(';')),

    expression_statement: $ => seq($._expression, optional(';')),

    _expression: $ =>
      choice(
        $.identifier,
        $.number,
        $.string,
        $.char,
        $.boolean,
        $.this,
        $.parenthesized_expression,
        $.call_expression,
        $.member_expression,
        $.binary_expression,
        $.unary_expression,
        $.cast_expression,
        $.assignment_expression,
        $.lambda_expression,
        $.object_expression,
        $.array_expression,
        $.thread_expression,
        $.await_expression,
      ),

    object_expression: $ =>
      seq('{', commaSep1($.object_pair), optional(','), '}'),

    object_pair: $ => seq(field('key', $.string), ':', field('value', $._expression)),

    array_expression: $ => seq('[', commaSep($._expression), ']'),

    parenthesized_expression: $ => seq('(', $._expression, ')'),

    call_expression: $ =>
      prec(
        PREC.call,
        seq(
          field('function', $._expression),
          field('arguments', $.arguments),
        ),
      ),

    arguments: $ => seq('(', commaSep($._expression), ')'),

    member_expression: $ =>
      prec(
        PREC.member,
        seq(
          field('object', $._expression),
          '.',
          field('property', alias($.identifier, $.property_identifier)),
        ),
      ),

    assignment_expression: $ =>
      prec.right(
        PREC.assign,
        seq(field('left', $._expression), '=', field('right', $._expression)),
      ),

    binary_expression: $ => {
      /** @type {Array<[string, number]>} */
      const table = [
        ['||', PREC.or],
        ['&&', PREC.and],
        ['==', PREC.compare],
        ['!=', PREC.compare],
        ['<', PREC.compare],
        ['<=', PREC.compare],
        ['>', PREC.compare],
        ['>=', PREC.compare],
        ['|', PREC.bit_or],
        ['^', PREC.bit_xor],
        ['&', PREC.bit_and],
        ['<<', PREC.shift],
        ['>>', PREC.shift],
        ['+', PREC.add],
        ['-', PREC.add],
        ['*', PREC.multiply],
        ['/', PREC.multiply],
        ['%', PREC.multiply],
      ];
      return choice(
        ...table.map(([operator, precedence]) =>
          prec.left(
            precedence,
            seq(
              field('left', $._expression),
              field('operator', operator),
              field('right', $._expression),
            ),
          ),
        ),
      );
    },

    unary_expression: $ =>
      prec(
        PREC.unary,
        seq(field('operator', choice('!', '-')), field('argument', $._expression)),
      ),

    cast_expression: $ =>
      prec.left(
        PREC.cast,
        seq(field('value', $._expression), 'as', field('type', $._type)),
      ),

    lambda_expression: $ =>
      prec.right(
        seq(
          $.lambda_parameters,
          '=>',
          field('body', choice($.block, $._expression)),
        ),
      ),

    lambda_parameters: $ => seq('(', commaSep($.lambda_parameter), ')'),

    lambda_parameter: $ =>
      seq(field('name', $.identifier), optional(seq(':', field('type', $._type)))),

    thread_expression: $ => seq('thread', $.block),

    await_expression: $ => prec(PREC.unary, seq('await', $._expression)),

    _type: $ =>
      choice(
        $.primitive_type,
        $.generic_type,
        $.pointer_type,
        $.reference_type,
        $.array_type,
        $.optional_type,
        $.variadic_type,
        alias($.identifier, $.type_identifier),
      ),

    primitive_type: _ => choice(...PRIMITIVE_TYPES),

    generic_type: $ =>
      prec(
        1,
        seq(
          field('name', alias($.identifier, $.type_identifier)),
          '<',
          commaSep1($._type),
          '>',
        ),
      ),

    pointer_type: $ => prec(2, seq($._type, '*')),
    reference_type: $ => prec(2, seq($._type, '&')),
    optional_type: $ => prec(2, seq($._type, '?')),
    variadic_type: $ => prec(2, seq($._type, '...')),
    array_type: $ => seq('[', $._type, ']'),

    identifier: _ => /[A-Za-z_][A-Za-z0-9_]*/,

    number: _ => /\d+(\.\d+)?/,

    boolean: _ => choice('true', 'false'),

    this: _ => 'this',

    string: $ =>
      seq(
        '"',
        repeat(choice(alias(/[^"\\]+/, $.string_content), $.escape_sequence)),
        '"',
      ),

    char: $ =>
      seq(
        "'",
        choice(alias(/[^'\\\n]/, $.string_content), $.escape_sequence),
        "'",
      ),

    escape_sequence: _ => token.immediate(/\\./),

    line_comment: _ => token(prec(-1, /\/\/[^\n]*/)),
    doc_comment: _ => token(prec(1, /\/\/\/[^\n]*/)),
  },
});
