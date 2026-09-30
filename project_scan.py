#!/usr/bin/env python3
"""
generate_structure.py

Walks a project directory and writes two Markdown files:
  1. A project structure doc — folder tree + per-file descriptions + full
     function signatures (Rust: visibility, params, return type, enclosing
     impl type).
  2. A data types doc — every Rust struct/enum with its fields or variants,
     skipping any file/folder that has none, so it stays focused on data
     shape rather than mirroring the whole tree.

No AI involved — pure text/brace-balance parsing.

Usage:
    python3 generate_structure.py [root_dir] [structure_output] [types_output]

Defaults:
    root_dir        = current directory
    structure_output = PROJECT_STRUCTURE.md
    types_output     = PROJECT_STRUCTS.md
"""

import os
import re
import sys

# Directories we never want to descend into
IGNORE_DIRS = {
    ".git",
    "target",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".idea",
    ".vscode",
    ".cargo",
    ".pytest_cache",
}

# Which extensions we bother trying to describe
CODE_EXTENSIONS = {
    ".rs",
    ".py",
    ".js",
    ".ts",
    ".jsx",
    ".tsx",
    ".c",
    ".cpp",
    ".h",
    ".hpp",
    ".go",
    ".java",
    ".sh",
    ".toml",
    ".rb",
}

MAX_DESC_LEN = 160
MAX_SIG_LEN = 220
MAX_LINES_SCANNED = 40

# Matches the start of a Rust fn item, up to and including its opening '('.
# Captures visibility/async/unsafe/extern modifiers, the name, and any generics.
RUST_FN_START_RE = re.compile(
    r'(?P<mods>(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?)'
    r"fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<generics><[^>]*>)?\s*\("
)

# Matches "impl SomeType {" or "impl<T> Trait for SomeType {" — group(2) is the
# target type when it's a trait impl ("for X"), otherwise group(1) is used.
RUST_IMPL_RE = re.compile(
    r"^\s*impl(?:\s*<[^>]*>)?\s+([\w:]+)(?:<[^>]*>)?(?:\s+for\s+([\w:]+))?"
)

# Matches the start of a struct or enum declaration (up through the name/generics,
# not including the body — the body is located separately since it may be a
# brace block, a tuple, or just a trailing semicolon for a unit struct).
RUST_TYPE_START_RE = re.compile(
    r"^\s*(?P<mods>(?:pub(?:\([^)]*\))?\s+)?)"
    r"(?P<kind>struct|enum)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<generics><[^>]*>)?"
)


def _scan_balanced_braces(text: str, open_idx: int) -> int:
    """Same as _scan_balanced but for {} instead of ()."""
    depth = 0
    i = open_idx
    n = len(text)
    while i < n:
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return n - 1


def _scan_type_body(text: str, start_idx: int):
    """From just after a struct/enum's name+generics, find its body.
    Returns (kind, inner_text) where kind is 'brace' (struct-with-fields or
    enum), 'tuple' (tuple struct), or 'unit' (no body, just `;`)."""
    depth = 0
    i = start_idx
    n = len(text)
    while i < n:
        c = text[i]
        if c in "<[":
            depth += 1
        elif c in ">]":
            if depth > 0:
                depth -= 1
        elif c == "{" and depth == 0:
            close = _scan_balanced_braces(text, i)
            return "brace", text[i + 1 : close]
        elif c == "(" and depth == 0:
            close = _scan_balanced(text, i)
            return "tuple", text[i + 1 : close]
        elif c == ";" and depth == 0:
            return "unit", ""
        i += 1
    return "unit", ""


def _split_top_level(s: str, sep: str = ",") -> list:
    """Split on `sep` but only at bracket depth 0, so commas inside nested
    <>, (), [] or {} (e.g. a HashMap<K, V> field type) don't split early."""
    parts = []
    depth = 0
    current = ""
    for ch in s:
        if ch in "<([{":
            depth += 1
        elif ch in ">)]}":
            if depth > 0:
                depth -= 1
        if ch == sep and depth == 0:
            parts.append(current)
            current = ""
        else:
            current += ch
    if current.strip():
        parts.append(current)
    return [" ".join(p.split()) for p in parts if p.strip()]


def extract_rust_types(filepath: str):
    """Return a list of (kind, name, generics, mods, fields, doc) for every
    struct/enum in a Rust file. `fields` is a list of display strings — field
    declarations for a struct, positional types for a tuple struct (index-
    prefixed), or variant names for an enum. Doc comments work the same way
    as for functions: an unbroken run of `///` lines directly above."""
    try:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            text = f.read()
    except OSError:
        return []

    lines = text.split("\n")
    line_offsets = []
    cursor = 0
    for line in lines:
        line_offsets.append(cursor)
        cursor += len(line) + 1

    types = []
    doc_buffer = []

    for i, line in enumerate(lines):
        stripped = line.strip()

        if stripped.startswith("///"):
            doc_buffer.append(stripped.lstrip("/").strip())
            continue
        if stripped.startswith("#["):
            continue

        match = RUST_TYPE_START_RE.search(line)
        if match:
            header_end = line_offsets[i] + match.end()
            body_kind, body_text = _scan_type_body(text, header_end)
            body_text = _strip_code_comments(body_text)

            kind = match.group("kind")
            raw_fields = _split_top_level(body_text) if body_kind != "unit" else []
            if kind == "struct" and body_kind == "tuple":
                fields = [f"{idx}: {f}" for idx, f in enumerate(raw_fields)]
            else:
                fields = raw_fields

            types.append(
                (
                    kind,
                    match.group("name"),
                    match.group("generics") or "",
                    match.group("mods").strip(),
                    fields,
                    " ".join(doc_buffer).strip(),
                )
            )
            doc_buffer = []
        elif stripped == "":
            doc_buffer = []
        else:
            doc_buffer = []

    return types


def _strip_code_comments(s: str) -> str:
    """Remove // line comments and /* */ block comments from a snippet of
    Rust code. Only /// doc comments (handled separately, outside this
    function) should ever surface as descriptive text — plain comments
    inside a signature (e.g. an inline note in a tuple return type) aren't
    real syntax and shouldn't leak into the displayed signature."""
    s = re.sub(r"/\*.*?\*/", " ", s, flags=re.S)
    s = re.sub(r"//[^\n]*", " ", s)
    return s


def _scan_balanced(text: str, open_idx: int) -> int:
    """Given the index of an opening '(' in text, return the index of its
    matching closing ')'. Assumes text[open_idx] == '('."""
    depth = 0
    i = open_idx
    n = len(text)
    while i < n:
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return n - 1  # unbalanced (shouldn't happen in valid Rust) — bail at EOF


def _scan_signature_tail(text: str, start_idx: int):
    """Scan forward from just after the closing ')' of a param list to find
    the top-level '{' or ';' that ends the signature (e.g. after a return
    type and/or a where-clause). Tracks <, (, [ depth so generics/arrays in
    the return type don't get mistaken for the function body opener."""
    depth = 0
    i = start_idx
    n = len(text)
    while i < n:
        c = text[i]
        if c in "<([":
            depth += 1
        elif c in ">)]":
            if depth > 0:
                depth -= 1
        elif c == "{" and depth == 0:
            return text[start_idx:i]
        elif c == ";" and depth == 0:
            return text[start_idx:i]
        i += 1
    return text[start_idx:]


def extract_rust_functions(filepath: str):
    """Return a list of (mods, name, generics, impl_type, params, return_part, doc)
    for every fn in a Rust file.

    impl_type is the enclosing "impl Type { ... }" or "impl Trait for Type { ... }"
    block's type name (tracked via brace-depth), or "" for free functions.
    Parameters and return type are reconstructed by balancing parens/brackets
    across the file's raw text, so multi-line signatures work too.

    A function gets a doc string if it has an unbroken run of `///` lines
    directly above it (attribute lines like #[test] are skipped over,
    everything else breaks the chain, matching how rustdoc treats them).
    """
    try:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            text = f.read()
    except OSError:
        return []

    lines = text.split("\n")
    line_offsets = []
    cursor = 0
    for line in lines:
        line_offsets.append(cursor)
        cursor += len(line) + 1  # +1 accounts for the '\n' split() removed

    functions = []
    doc_buffer = []
    depth = 0
    impl_stack = []  # list of (depth_of_body, type_name)

    for i, line in enumerate(lines):
        # Pop any impl blocks we've exited before looking at this line
        while impl_stack and depth < impl_stack[-1][0]:
            impl_stack.pop()
        current_impl = impl_stack[-1][1] if impl_stack else ""

        stripped = line.strip()

        if stripped.startswith("///"):
            doc_buffer.append(stripped.lstrip("/").strip())
        elif stripped.startswith("#["):
            pass  # attribute — doesn't break the doc chain
        else:
            match = RUST_FN_START_RE.search(line)
            if match:
                open_paren_idx = line_offsets[i] + match.end() - 1
                close_paren_idx = _scan_balanced(text, open_paren_idx)
                raw_params = _strip_code_comments(
                    text[open_paren_idx + 1 : close_paren_idx]
                )
                params = " ".join(raw_params.split())

                tail = _strip_code_comments(
                    _scan_signature_tail(text, close_paren_idx + 1)
                ).strip()
                return_part = (
                    f" {' '.join(tail.split())}" if tail.startswith("->") else ""
                )

                functions.append(
                    (
                        match.group("mods").strip(),
                        match.group("name"),
                        match.group("generics") or "",
                        current_impl,
                        params,
                        return_part,
                        " ".join(doc_buffer).strip(),
                    )
                )
                doc_buffer = []
            elif stripped == "":
                doc_buffer = []
            else:
                doc_buffer = []  # any other code line breaks the doc-comment chain

        # Track impl blocks via naive brace counting (ignores strings/comments,
        # which is a fine tradeoff for rustfmt-formatted code)
        impl_match = RUST_IMPL_RE.match(line)
        opens = line.count("{")
        closes = line.count("}")
        if impl_match and opens > 0:
            target_type = impl_match.group(2) or impl_match.group(1)
            impl_stack.append((depth + 1, target_type))
        depth += opens - closes

    return functions


def extract_description(filepath: str) -> str:
    """Pull a short description from the top-of-file comment/docstring."""
    try:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            lines = [f.readline() for _ in range(MAX_LINES_SCANNED)]
    except OSError:
        return ""

    lines = [l for l in lines if l is not None]
    ext = os.path.splitext(filepath)[1]

    # Rust: consecutive //! or /// lines at the top (module/item doc comments)
    if ext == ".rs":
        collected = []
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("//!") or stripped.startswith("///"):
                text = stripped.lstrip("/!").strip()
                if text:
                    collected.append(text)
            elif collected:
                break
        if collected:
            return " ".join(collected)

    # Python: module-level docstring
    if ext == ".py":
        text = "".join(lines)
        stripped = text.lstrip()
        for quote in ('"""', "'''"):
            if stripped.startswith(quote):
                end = stripped.find(quote, 3)
                if end != -1:
                    doc = stripped[3:end].strip()
                    return " ".join(doc.split())
        # fall back to leading # comments
        collected = []
        for line in lines:
            s = line.strip()
            if s.startswith("#!"):
                continue
            if s.startswith("#"):
                collected.append(s.lstrip("#").strip())
            elif collected:
                break
            elif s:
                break
        if collected:
            return " ".join(collected)

    # C-style block comment /* ... */ or leading // lines
    if ext in (
        ".js",
        ".ts",
        ".jsx",
        ".tsx",
        ".c",
        ".cpp",
        ".h",
        ".hpp",
        ".go",
        ".java",
    ):
        text = "".join(lines)
        stripped = text.lstrip()
        if stripped.startswith("/*"):
            end = stripped.find("*/")
            if end != -1:
                doc = stripped[2:end]
                doc = " ".join(l.strip().lstrip("*").strip() for l in doc.splitlines())
                return " ".join(doc.split())
        collected = []
        for line in lines:
            s = line.strip()
            if s.startswith("//"):
                collected.append(s.lstrip("/").strip())
            elif collected:
                break
            elif s:
                break
        if collected:
            return " ".join(collected)

    # Shell: leading # comments (skip shebang)
    if ext == ".sh":
        collected = []
        for line in lines:
            s = line.strip()
            if s.startswith("#!"):
                continue
            if s.startswith("#"):
                collected.append(s.lstrip("#").strip())
            elif collected:
                break
            elif s:
                break
        if collected:
            return " ".join(collected)

    return ""


def truncate(text: str, max_len: int = MAX_DESC_LEN) -> str:
    text = text.strip()
    if len(text) <= max_len:
        return text
    return text[: max_len - 1].rstrip() + "…"


def build_signature(
    mods: str,
    name: str,
    generics: str,
    impl_type: str,
    params: str,
    return_part: str,
    max_len: int = MAX_SIG_LEN,
) -> str:
    """Assemble a display signature, truncating only the param list (never
    the return type) if the whole thing would exceed max_len."""
    qualified_name = f"{impl_type}::{name}" if impl_type else name
    head = f"{mods + ' ' if mods else ''}fn {qualified_name}{generics}("
    tail = f"){return_part}"
    budget = max(max_len - len(head) - len(tail), 15)
    if len(params) > budget:
        params = params[: budget - 1].rstrip() + "…"
    return head + params + tail


def _insert_type_path(tree: dict, parts: list, value: list):
    node = tree
    for p in parts[:-1]:
        node = node.setdefault(p, {})
    node[parts[-1]] = value  # leaf: a list of type tuples


def _render_types_tree(node: dict, indent: str, out_lines: list):
    for key in sorted(node.keys()):
        val = node[key]
        if isinstance(val, dict):
            out_lines.append(f"{indent}- **{key}/**")
            _render_types_tree(val, indent + "  ", out_lines)
        else:
            out_lines.append(f"{indent}- `{key}`")
            for kind, name, generics, mods, fields, doc in val:
                header = f"{mods + ' ' if mods else ''}{kind} {name}{generics}"
                if doc:
                    out_lines.append(f"{indent}  - **{header}** — {truncate(doc)}")
                else:
                    out_lines.append(f"{indent}  - **{header}**")
                for field in fields:
                    out_lines.append(f"{indent}    - `{truncate(field, 150)}`")


def build_types_doc(root: str, out_lines: list):
    """Like build_tree, but only for Rust structs/enums — skips any
    directory or file that doesn't contain at least one, so this stays
    focused on data shape rather than mirroring the whole project tree."""
    root = os.path.abspath(root)
    tree = {}

    for current_dir, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(
            d for d in dirnames if d not in IGNORE_DIRS and not d.startswith(".")
        )
        for fname in sorted(filenames):
            if not fname.endswith(".rs") or fname.startswith("."):
                continue
            fpath = os.path.join(current_dir, fname)
            found_types = extract_rust_types(fpath)
            if not found_types:
                continue
            rel_parts = os.path.relpath(fpath, root).split(os.sep)
            _insert_type_path(tree, rel_parts, found_types)

    _render_types_tree(tree, "  ", out_lines)


def build_tree(root: str, out_lines: list):
    root = os.path.abspath(root)
    for current_dir, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(
            d for d in dirnames if d not in IGNORE_DIRS and not d.startswith(".")
        )
        filenames = sorted(f for f in filenames if not f.startswith("."))

        rel = os.path.relpath(current_dir, root)
        depth = 0 if rel == "." else rel.count(os.sep) + 1
        indent = "  " * depth

        if rel != ".":
            out_lines.append(f"{indent}- **{os.path.basename(current_dir)}/**")

        file_indent = "  " * (depth + 1) if rel != "." else "  "
        for fname in filenames:
            fpath = os.path.join(current_dir, fname)
            ext = os.path.splitext(fname)[1]
            desc = ""
            if ext in CODE_EXTENSIONS:
                desc = truncate(extract_description(fpath))
            if desc:
                out_lines.append(f"{file_indent}- `{fname}` — {desc}")
            else:
                out_lines.append(f"{file_indent}- `{fname}`")

            if ext == ".rs":
                fn_indent = file_indent + "  "
                for (
                    mods,
                    name,
                    generics,
                    impl_type,
                    params,
                    return_part,
                    fn_doc,
                ) in extract_rust_functions(fpath):
                    sig_text = build_signature(
                        mods, name, generics, impl_type, params, return_part
                    )
                    if fn_doc:
                        out_lines.append(
                            f"{fn_indent}- `{sig_text}` — {truncate(fn_doc)}"
                        )
                    else:
                        out_lines.append(f"{fn_indent}- `{sig_text}`")


def main():
    root_dir = sys.argv[1] if len(sys.argv) > 1 else "."
    output_file = sys.argv[2] if len(sys.argv) > 2 else "PROJECT_STRUCTURE.md"
    types_output_file = sys.argv[3] if len(sys.argv) > 3 else "PROJECT_STRUCTS.md"

    project_name = os.path.basename(os.path.abspath(root_dir))

    lines = [f"# Project Structure: {project_name}", ""]
    build_tree(root_dir, lines)
    with open(output_file, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    print(f"Wrote {output_file} ({len(lines)} lines)")

    type_lines = [f"# Data Types: {project_name}", ""]
    build_types_doc(root_dir, type_lines)
    with open(types_output_file, "w", encoding="utf-8") as f:
        f.write("\n".join(type_lines) + "\n")
    print(f"Wrote {types_output_file} ({len(type_lines)} lines)")


if __name__ == "__main__":
    main()
