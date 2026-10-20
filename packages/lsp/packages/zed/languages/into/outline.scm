(class_declaration
  (modifiers)? @context
  name: (type_identifier) @name) @item

(enum_declaration
  "enum" @context
  name: (type_identifier) @name) @item

(enum_variant
  name: (identifier) @name) @item

(function_declaration
  "fun" @context
  name: (identifier) @name) @item

(event_declaration
  "event" @context
  name: (identifier) @name) @item

(macro_declaration
  "macro" @context
  name: (identifier) @name) @item

(field_declaration
  "into" @context
  name: (identifier) @name) @item

(init_block
  "init" @name) @item

(variable_declaration
  "into" @context
  name: (identifier) @name) @item
