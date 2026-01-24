module.exports = grammar({
  name: "wui",

  extras: $ => [/\s/],

  rules: {
    source_file: $ => repeat($._node),

    _node: $ => choice($.element, $.text),

    element: $ => choice($.element_with_children, $.self_closing_element),

    element_with_children: $ => seq($.start_tag, repeat($._node), $.end_tag),

    start_tag: $ => seq("<", field("name", $.tag_name), repeat($.attribute), ">"),

    end_tag: $ => seq("</", field("name", $.tag_name), ">"),

    self_closing_element: $ => seq(
      "<",
      field("name", $.tag_name),
      repeat($.attribute),
      "/>"
    ),

    attribute: $ => seq(
      field("name", $.attribute_name),
      optional(seq("=", field("value", $.attribute_value)))
    ),

    attribute_value: $ => choice($.string, $.expr, $.bare_literal),

    string: $ => /"([^"\\]|\\.)*"/,

    expr: $ => seq("{", /[^}]*/, "}"),

    bare_literal: $ => /[A-Za-z0-9_.:-]+/,

    tag_name: $ => /[A-Za-z_][A-Za-z0-9_:\-]*/,

    attribute_name: $ => /[A-Za-z_][A-Za-z0-9_:\-]*/,

    text: $ => /[^<>{}]+/,
  },
});
