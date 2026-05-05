"""
ast_format.py — Layer 1: Format Java method bodies as structured Markdown.

Handles ALL tree-sitter-java statement types. No node types are silently dropped.
"""

from .java_parser import Node, node_text, find_child, find_children, find_descendants


def format_method_body(body_node: Node, source: str, indent: str = "") -> list[str]:
    """Format all statements in a method body block as structured markdown lines."""
    if body_node is None:
        return [f"{indent}- _(empty body)_"]
    return _format_statements(body_node, source, indent)


def _format_statements(block_node: Node, source: str, indent: str) -> list[str]:
    """Recursively format all child statements of a block/body node."""
    out = []
    for c in block_node.children:
        out.extend(_format_node(c, source, indent))
    return out


def _format_node(node: Node, source: str, indent: str) -> list[str]:
    """Format a single AST node. Returns list of markdown lines."""
    handler = _HANDLERS.get(node.type)
    if handler:
        return handler(node, source, indent)
    # Skip syntax tokens like { } ( ) ; etc.
    if node.type in ("{", "}", "(", ")", ";", ",", "//", "/*"):
        return []
    # Skip comment nodes
    if node.type in ("line_comment", "block_comment"):
        return []
    return []


# ── Statement handlers ─────────────────────────────────────────────────────

def _fmt_expression(node: Node, source: str, indent: str) -> list[str]:
    text = node_text(node, source).replace('\n', ' ').strip()
    return [f"{indent}- `{text}`"]


def _fmt_return(node: Node, source: str, indent: str) -> list[str]:
    text = node_text(node, source).replace('\n', ' ').strip()
    return [f"{indent}- **RETURN** `{text}`"]


def _fmt_break(node: Node, source: str, indent: str) -> list[str]:
    text = node_text(node, source).replace('\n', ' ').strip()
    return [f"{indent}- **BREAK** `{text}`"]


def _fmt_continue(node: Node, source: str, indent: str) -> list[str]:
    return [f"{indent}- **CONTINUE**"]


def _fmt_throw(node: Node, source: str, indent: str) -> list[str]:
    text = node_text(node, source).replace('\n', ' ').strip()
    return [f"{indent}- **THROW** `{text}`"]


def _fmt_var_decl(node: Node, source: str, indent: str) -> list[str]:
    text = node_text(node, source).replace('\n', ' ').strip()
    return [f"{indent}- **VAR** `{text}`"]


def _fmt_assert(node: Node, source: str, indent: str) -> list[str]:
    text = node_text(node, source).replace('\n', ' ').strip()
    return [f"{indent}- **ASSERT** `{text}`"]


# ── Control flow ───────────────────────────────────────────────────────────

def _fmt_if(node: Node, source: str, indent: str) -> list[str]:
    """Handle if / else-if / else chains, including braceless bodies."""
    out = []

    # Condition
    cond = find_child(node, "parenthesized_expression")
    cond_text = node_text(cond, source) if cond else "(?)"
    out.append(f"{indent}- **IF** `{cond_text}`:")

    # Consequence: find first block or non-keyword statement
    consequence = _find_body(node, source, skip_first_block=False)
    if consequence:
        if consequence.type == "block":
            out.extend(_format_statements(consequence, source, indent + "  "))
        else:
            # Single statement without braces
            out.extend(_format_node(consequence, source, indent + "  "))

    # Process else / else-if
    # In tree-sitter-java, else is: "else" keyword followed by block or if_statement
    children = list(node.children)
    i = 0
    while i < len(children):
        c = children[i]
        if node_text(c, source) == "else":
            # Next sibling is the else body
            if i + 1 < len(children):
                else_body = children[i + 1]
                if else_body.type == "if_statement":
                    # else-if chain
                    cond2 = find_child(else_body, "parenthesized_expression")
                    cond2_text = node_text(cond2, source) if cond2 else "(?)"
                    out.append(f"{indent}- **ELSE IF** `{cond2_text}`:")
                    body2 = _find_body(else_body, source, skip_first_block=False)
                    if body2:
                        if body2.type == "block":
                            out.extend(_format_statements(body2, source, indent + "  "))
                        else:
                            out.extend(_format_node(body2, source, indent + "  "))
                    # Continue processing the else-if's own else clauses
                    # by recursively exploring else_body's children
                    sub_else = _extract_else_chain(else_body, source, indent)
                    out.extend(sub_else)
                elif else_body.type == "block":
                    out.append(f"{indent}- **ELSE**:")
                    out.extend(_format_statements(else_body, source, indent + "  "))
                else:
                    # Single statement else
                    out.append(f"{indent}- **ELSE**:")
                    out.extend(_format_node(else_body, source, indent + "  "))
                i += 2
                continue
        i += 1

    return out


def _extract_else_chain(if_node: Node, source: str, indent: str) -> list[str]:
    """Extract else/else-if chain from an if_statement node (used for nested else-if)."""
    out = []
    children = list(if_node.children)
    i = 0
    while i < len(children):
        c = children[i]
        if node_text(c, source) == "else":
            if i + 1 < len(children):
                else_body = children[i + 1]
                if else_body.type == "if_statement":
                    cond = find_child(else_body, "parenthesized_expression")
                    cond_text = node_text(cond, source) if cond else "(?)"
                    out.append(f"{indent}- **ELSE IF** `{cond_text}`:")
                    body = _find_body(else_body, source, skip_first_block=False)
                    if body:
                        if body.type == "block":
                            out.extend(_format_statements(body, source, indent + "  "))
                        else:
                            out.extend(_format_node(body, source, indent + "  "))
                    out.extend(_extract_else_chain(else_body, source, indent))
                elif else_body.type == "block":
                    out.append(f"{indent}- **ELSE**:")
                    out.extend(_format_statements(else_body, source, indent + "  "))
                else:
                    out.append(f"{indent}- **ELSE**:")
                    out.extend(_format_node(else_body, source, indent + "  "))
                i += 2
                continue
        i += 1
    return out


def _find_body(node: Node, source: str, skip_first_block: bool = False) -> Node | None:
    """Find the body of a control flow statement (block or single statement)."""
    # Body types that could appear as consequence of if/for/while
    body_types = {
        "block", "expression_statement", "return_statement",
        "break_statement", "continue_statement", "throw_statement",
        "if_statement", "for_statement", "enhanced_for_statement",
        "while_statement", "do_statement", "switch_expression",
        "try_statement", "local_variable_declaration",
    }
    skipped = False
    for c in node.children:
        if c.type in body_types:
            if skip_first_block and c.type == "block" and not skipped:
                skipped = True
                continue
            return c
    return None


def _fmt_enhanced_for(node: Node, source: str, indent: str) -> list[str]:
    """for (Type var : iterable) { body }"""
    out = []
    # Extract the loop header
    header = node_text(node, source).split(')')[0] + ')'
    # Clean up: remove 'for (' prefix, show just the declaration
    header_clean = header.replace('for (', '').replace('for(', '').strip()
    out.append(f"{indent}- **FOR EACH** `{header_clean}`:")

    body = find_child(node, "block")
    if body:
        out.extend(_format_statements(body, source, indent + "  "))
    else:
        # Single statement body
        body = _find_body(node, source)
        if body:
            out.extend(_format_node(body, source, indent + "  "))
    return out


def _fmt_for(node: Node, source: str, indent: str) -> list[str]:
    """Traditional for (init; cond; update) { body }"""
    out = []
    header = node_text(node, source).split(')')[0] + ')'
    header_clean = header.replace('\n', ' ').strip()
    # Truncate if very long
    if len(header_clean) > 120:
        header_clean = header_clean[:117] + "..."
    out.append(f"{indent}- **FOR** `{header_clean}`:")

    body = find_child(node, "block")
    if body:
        out.extend(_format_statements(body, source, indent + "  "))
    else:
        body = _find_body(node, source)
        if body:
            out.extend(_format_node(body, source, indent + "  "))
    return out


def _fmt_while(node: Node, source: str, indent: str) -> list[str]:
    out = []
    cond = find_child(node, "parenthesized_expression")
    cond_text = node_text(cond, source) if cond else "(?)"
    out.append(f"{indent}- **WHILE** `{cond_text}`:")

    body = find_child(node, "block")
    if body:
        out.extend(_format_statements(body, source, indent + "  "))
    else:
        body = _find_body(node, source)
        if body:
            out.extend(_format_node(body, source, indent + "  "))
    return out


def _fmt_do_while(node: Node, source: str, indent: str) -> list[str]:
    out = []
    cond = find_child(node, "parenthesized_expression")
    cond_text = node_text(cond, source) if cond else "(?)"
    out.append(f"{indent}- **DO WHILE** `{cond_text}`:")

    body = find_child(node, "block")
    if body:
        out.extend(_format_statements(body, source, indent + "  "))
    return out


def _fmt_switch(node: Node, source: str, indent: str) -> list[str]:
    """switch (expr) { case X: ... default: ... }"""
    out = []
    cond = find_child(node, "parenthesized_expression")
    cond_text = node_text(cond, source) if cond else "(?)"
    out.append(f"{indent}- **SWITCH** `{cond_text}`:")

    # Find switch_block
    sw_block = find_child(node, "switch_block")
    if sw_block:
        for group in sw_block.children:
            if group.type == "switch_block_statement_group":
                _fmt_switch_group(group, source, indent + "  ", out)
            elif group.type == "switch_rule":
                _fmt_switch_rule(group, source, indent + "  ", out)
    return out


def _fmt_switch_group(group: Node, source: str, indent: str, out: list[str]):
    """Handle a switch case group (case X: statements...)"""
    for c in group.children:
        if c.type == "switch_label":
            label_text = node_text(c, source).strip().rstrip(':')
            out.append(f"{indent}- **{label_text.upper()}**:")
        else:
            out.extend(_format_node(c, source, indent + "  "))


def _fmt_switch_rule(rule: Node, source: str, indent: str, out: list[str]):
    """Handle switch rule (case X -> expr)"""
    text = node_text(rule, source).replace('\n', ' ').strip()
    out.append(f"{indent}- `{text}`")


def _fmt_try(node: Node, source: str, indent: str) -> list[str]:
    out = []
    out.append(f"{indent}- **TRY**:")

    # Try body
    body = find_child(node, "block")
    if body:
        out.extend(_format_statements(body, source, indent + "  "))

    # Catch clauses
    for catch in find_children(node, "catch_clause"):
        param = find_child(catch, "catch_formal_parameter")
        param_text = node_text(param, source) if param else "?"
        out.append(f"{indent}- **CATCH** `({param_text})`:")
        catch_body = find_child(catch, "block")
        if catch_body:
            out.extend(_format_statements(catch_body, source, indent + "  "))

    # Finally clause
    finally_clause = find_child(node, "finally_clause")
    if finally_clause:
        out.append(f"{indent}- **FINALLY**:")
        finally_body = find_child(finally_clause, "block")
        if finally_body:
            out.extend(_format_statements(finally_body, source, indent + "  "))

    return out


def _fmt_try_with_resources(node: Node, source: str, indent: str) -> list[str]:
    out = []
    # Resource spec
    res = find_child(node, "resource_specification")
    res_text = node_text(res, source).replace('\n', ' ').strip() if res else ""
    out.append(f"{indent}- **TRY** `{res_text}`:")

    body = find_child(node, "block")
    if body:
        out.extend(_format_statements(body, source, indent + "  "))

    for catch in find_children(node, "catch_clause"):
        param = find_child(catch, "catch_formal_parameter")
        param_text = node_text(param, source) if param else "?"
        out.append(f"{indent}- **CATCH** `({param_text})`:")
        catch_body = find_child(catch, "block")
        if catch_body:
            out.extend(_format_statements(catch_body, source, indent + "  "))

    return out


def _fmt_synchronized(node: Node, source: str, indent: str) -> list[str]:
    out = []
    cond = find_child(node, "parenthesized_expression")
    cond_text = node_text(cond, source) if cond else "(?)"
    out.append(f"{indent}- **SYNCHRONIZED** `{cond_text}`:")

    body = find_child(node, "block")
    if body:
        out.extend(_format_statements(body, source, indent + "  "))
    return out


def _fmt_labeled(node: Node, source: str, indent: str) -> list[str]:
    out = []
    label = find_child(node, "identifier")
    label_text = node_text(label, source) if label else "?"
    out.append(f"{indent}- **LABEL** `{label_text}`:")
    # The labeled statement's body
    for c in node.children:
        if c.type not in ("identifier", ":"):
            out.extend(_format_node(c, source, indent + "  "))
    return out


def _fmt_block(node: Node, source: str, indent: str) -> list[str]:
    """Nested block (anonymous scope)."""
    return _format_statements(node, source, indent)


# ── Handler registry ───────────────────────────────────────────────────────

_HANDLERS = {
    "expression_statement": _fmt_expression,
    "return_statement": _fmt_return,
    "break_statement": _fmt_break,
    "continue_statement": _fmt_continue,
    "throw_statement": _fmt_throw,
    "local_variable_declaration": _fmt_var_decl,
    "assert_statement": _fmt_assert,
    "if_statement": _fmt_if,
    "enhanced_for_statement": _fmt_enhanced_for,
    "for_statement": _fmt_for,
    "while_statement": _fmt_while,
    "do_statement": _fmt_do_while,
    "switch_expression": _fmt_switch,
    "switch_statement": _fmt_switch,  # Some tree-sitter versions use this
    "try_statement": _fmt_try,
    "try_with_resources_statement": _fmt_try_with_resources,
    "synchronized_statement": _fmt_synchronized,
    "labeled_statement": _fmt_labeled,
    "block": _fmt_block,
}
