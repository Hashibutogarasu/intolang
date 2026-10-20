(identifier) @variable
(property_identifier) @property
(primitive_type) @type
(type_identifier) @type
(this) @variable.special

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))

(member_expression
  object: (identifier) @type
  (#match? @type "^[A-Z]"))

(call_expression
  function: (identifier) @function)
(call_expression
  function: (member_expression
    property: (property_identifier) @function.method))
(call_expression
  function: (identifier) @constructor
  (#match? @constructor "^[A-Z]"))

(enum_variant
  name: (identifier) @constant)
(field_declaration
  name: (identifier) @property)
(parameter
  name: (identifier) @variable.parameter)
(lambda_parameter
  name: (identifier) @variable.parameter)
(variable_declaration
  name: (identifier) @variable)

(function_declaration
  name: (identifier) @function)
(event_declaration
  name: (identifier) @function)
(macro_declaration
  name: (identifier) @function)

(attribute_item
  name: (identifier) @attribute)
(attribute_item
  ["#" "[" "]"] @attribute)

(number) @number
(boolean) @boolean
(string) @string
(char) @string
(escape_sequence) @string.escape

(foreign) @keyword
(unary_expression
  operator: _ @operator)
(binary_expression
  operator: _ @operator)
["=" "=>" "..." "?" "*" "&"] @operator

[
  "import"
  "export"
  "event"
  "enum"
  "macro"
  "init"
  "fun"
  "into"
  "now"
  "late"
  "const"
  "if"
  "else"
  "while"
  "break"
  "return"
  "throw"
  "unload"
  "thread"
  "await"
  "as"
  "pub"
  "static"
  "abstract"
  "default"
  "override"
] @keyword

["(" ")" "[" "]" "{" "}"] @punctuation.bracket
(type_parameters ["<" ">"] @punctuation.bracket)
(event_type_arguments ["<" ">"] @punctuation.bracket)
(generic_type ["<" ">"] @punctuation.bracket)
["," ";" ":" "."] @punctuation.delimiter

(line_comment) @comment
(doc_comment) @comment.doc
