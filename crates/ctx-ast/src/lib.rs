use regex::Regex;
use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser, Tree};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Module,
    Class,
    Function,
    Test,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub file_path: String,
    pub name: String,
    pub kind: SymbolKind,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolSlice {
    pub file_path: String,
    pub symbol_name: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
struct RawSymbol {
    symbol: Symbol,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
}

pub fn extract_symbols(code: &str, file_path: &str) -> Vec<Symbol> {
    if let Some(raw) = extract_symbols_tree_sitter(code, file_path) {
        return raw.into_iter().map(|entry| entry.symbol).collect();
    }

    extract_symbols_regex_fallback(code, file_path)
}

pub fn slice_symbols(code: &str, file_path: &str, symbol_names: &[&str]) -> Vec<SymbolSlice> {
    let names = symbol_names
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();

    if names.is_empty() {
        return Vec::new();
    }

    let raws = extract_symbols_tree_sitter(code, file_path)
        .unwrap_or_else(|| fallback_raw_symbols(code, file_path));

    raws.into_iter()
        .filter(|entry| names.iter().any(|name| *name == entry.symbol.name))
        .map(|entry| {
            let slice = code
                .get(entry.start_byte..entry.end_byte)
                .unwrap_or_default();
            SymbolSlice {
                file_path: entry.symbol.file_path,
                symbol_name: entry.symbol.name,
                content: slice.to_string(),
                start_line: entry.start_line,
                end_line: entry.end_line,
            }
        })
        .collect()
}

fn extract_symbols_tree_sitter(code: &str, file_path: &str) -> Option<Vec<RawSymbol>> {
    let mut parser = Parser::new();
    let language_set = if file_path.ends_with(".rs") {
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).ok()
    } else if file_path.ends_with(".py") {
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .ok()
    } else {
        return None;
    };

    language_set?;
    let tree = parser.parse(code, None)?;

    if file_path.ends_with(".rs") {
        Some(extract_rust_symbols(code, &tree, file_path))
    } else {
        Some(extract_python_symbols(code, &tree, file_path))
    }
}

fn extract_rust_symbols(code: &str, tree: &Tree, file_path: &str) -> Vec<RawSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        match node.kind() {
            "function_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    let kind = if name.starts_with("test_") || has_test_attribute(code, node) {
                        SymbolKind::Test
                    } else {
                        SymbolKind::Function
                    };
                    symbols.push(raw_symbol(file_path, &name, kind, &signature, node));
                }
            }
            "struct_item" | "enum_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(
                        file_path,
                        &name,
                        SymbolKind::Class,
                        &signature,
                        node,
                    ));
                }
            }
            "use_declaration" => {
                let import = first_line(node_text(code, node));
                symbols.push(raw_symbol(
                    file_path,
                    &import,
                    SymbolKind::Import,
                    &import,
                    node,
                ));
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    symbols
}

fn extract_python_symbols(code: &str, tree: &Tree, file_path: &str) -> Vec<RawSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        match node.kind() {
            "function_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    let kind = if name.starts_with("test_") {
                        SymbolKind::Test
                    } else {
                        SymbolKind::Function
                    };
                    symbols.push(raw_symbol(file_path, &name, kind, &signature, node));
                }
            }
            "class_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(
                        file_path,
                        &name,
                        SymbolKind::Class,
                        &signature,
                        node,
                    ));
                }
            }
            "import_statement" | "import_from_statement" => {
                let import = first_line(node_text(code, node));
                symbols.push(raw_symbol(
                    file_path,
                    &import,
                    SymbolKind::Import,
                    &import,
                    node,
                ));
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    symbols
}

fn raw_symbol(
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    signature: &str,
    node: Node<'_>,
) -> RawSymbol {
    RawSymbol {
        symbol: Symbol {
            file_path: file_path.to_string(),
            name: name.to_string(),
            kind,
            signature: signature.to_string(),
        },
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
    }
}

fn has_test_attribute(code: &str, node: Node<'_>) -> bool {
    let start = node.start_byte();
    if start == 0 {
        return false;
    }

    let prefix = &code[..start];
    prefix
        .lines()
        .rev()
        .take(3)
        .any(|line| line.trim().starts_with("#[test]"))
}

fn node_text(code: &str, node: Node<'_>) -> String {
    code.get(node.byte_range()).unwrap_or_default().to_string()
}

fn first_line(text: String) -> String {
    text.lines().next().unwrap_or_default().trim().to_string()
}

fn extract_symbols_regex_fallback(code: &str, file_path: &str) -> Vec<Symbol> {
    fallback_raw_symbols(code, file_path)
        .into_iter()
        .map(|entry| entry.symbol)
        .collect()
}

fn fallback_raw_symbols(code: &str, file_path: &str) -> Vec<RawSymbol> {
    let rust_fn =
        Regex::new(r"(?m)^\s*(?:pub\s+)?fn\s+([a-zA-Z0-9_]+)\s*\(([^)]*)\)").expect("regex");
    let py_fn = Regex::new(r"(?m)^\s*def\s+([a-zA-Z0-9_]+)\s*\(([^)]*)\)").expect("regex");
    let py_class = Regex::new(r"(?m)^\s*class\s+([a-zA-Z0-9_]+)").expect("regex");
    let mut out = Vec::new();

    for captures in rust_fn.captures_iter(code) {
        let Some(m) = captures.get(0) else {
            continue;
        };
        let name = captures.get(1).map(|v| v.as_str()).unwrap_or_default();
        let args = captures.get(2).map(|v| v.as_str()).unwrap_or_default();

        out.push(RawSymbol {
            symbol: Symbol {
                file_path: file_path.to_string(),
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature: format!("fn {name}({args})"),
            },
            start_byte: m.start(),
            end_byte: m.end(),
            start_line: line_of_byte(code, m.start()),
            end_line: line_of_byte(code, m.end()),
        });
    }

    for captures in py_fn.captures_iter(code) {
        let Some(m) = captures.get(0) else {
            continue;
        };
        let name = captures.get(1).map(|v| v.as_str()).unwrap_or_default();
        let args = captures.get(2).map(|v| v.as_str()).unwrap_or_default();

        out.push(RawSymbol {
            symbol: Symbol {
                file_path: file_path.to_string(),
                name: name.to_string(),
                kind: if name.starts_with("test_") {
                    SymbolKind::Test
                } else {
                    SymbolKind::Function
                },
                signature: format!("def {name}({args})"),
            },
            start_byte: m.start(),
            end_byte: m.end(),
            start_line: line_of_byte(code, m.start()),
            end_line: line_of_byte(code, m.end()),
        });
    }

    for captures in py_class.captures_iter(code) {
        let Some(m) = captures.get(0) else {
            continue;
        };
        let name = captures.get(1).map(|v| v.as_str()).unwrap_or_default();
        out.push(RawSymbol {
            symbol: Symbol {
                file_path: file_path.to_string(),
                name: name.to_string(),
                kind: SymbolKind::Class,
                signature: format!("class {name}"),
            },
            start_byte: m.start(),
            end_byte: m.end(),
            start_line: line_of_byte(code, m.start()),
            end_line: line_of_byte(code, m.end()),
        });
    }

    out
}

fn line_of_byte(code: &str, byte_idx: usize) -> usize {
    code[..byte_idx.min(code.len())]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1
}
