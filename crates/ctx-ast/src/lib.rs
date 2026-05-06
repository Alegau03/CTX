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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    C,
    CHeader,
    Cpp,
    CppHeader,
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
    let language = source_language_for_file(file_path)?;
    let language_set = match language {
        SourceLanguage::Rust => parser.set_language(&tree_sitter_rust::LANGUAGE.into()).ok(),
        SourceLanguage::Python => parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .ok(),
        SourceLanguage::JavaScript => parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .ok(),
        SourceLanguage::TypeScript => parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .ok(),
        SourceLanguage::Tsx => parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
            .ok(),
        SourceLanguage::C | SourceLanguage::CHeader => parser
            .set_language(&tree_sitter_c::LANGUAGE.into())
            .ok(),
        SourceLanguage::Cpp | SourceLanguage::CppHeader => parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .ok(),
    };

    language_set?;
    let tree = parser.parse(code, None)?;

    Some(match language {
        SourceLanguage::Rust => extract_rust_symbols(code, &tree, file_path),
        SourceLanguage::Python => extract_python_symbols(code, &tree, file_path),
        SourceLanguage::JavaScript | SourceLanguage::TypeScript | SourceLanguage::Tsx => {
            extract_javascript_symbols(code, &tree, file_path)
        }
        SourceLanguage::C | SourceLanguage::CHeader => {
            extract_c_symbols(code, &tree, file_path)
        }
        SourceLanguage::Cpp | SourceLanguage::CppHeader => {
            extract_cpp_symbols(code, &tree, file_path)
        }
    })
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

fn extract_javascript_symbols(code: &str, tree: &Tree, file_path: &str) -> Vec<RawSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        match node.kind() {
            "function_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(
                        file_path,
                        &name,
                        SymbolKind::Function,
                        &signature,
                        node,
                    ));
                }
            }
            "class_declaration" => {
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
            "method_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    let kind = if is_js_test_name(&name) {
                        SymbolKind::Test
                    } else {
                        SymbolKind::Function
                    };
                    symbols.push(raw_symbol(file_path, &name, kind, &signature, node));
                }
            }
            "import_statement" => {
                let import = first_line(node_text(code, node));
                symbols.push(raw_symbol(
                    file_path,
                    &import,
                    SymbolKind::Import,
                    &import,
                    node,
                ));
            }
            "lexical_declaration" | "variable_declaration" => {
                extract_js_variable_symbols(code, file_path, node, &mut symbols);
            }
            "call_expression" => {
                if let Some(test_symbol) = extract_js_test_call(code, file_path, node) {
                    symbols.push(test_symbol);
                }
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

fn raw_symbol_with_span(
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    signature: &str,
    span_node: Node<'_>,
) -> RawSymbol {
    RawSymbol {
        symbol: Symbol {
            file_path: file_path.to_string(),
            name: name.to_string(),
            kind,
            signature: signature.to_string(),
        },
        start_byte: span_node.start_byte(),
        end_byte: span_node.end_byte(),
        start_line: span_node.start_position().row + 1,
        end_line: span_node.end_position().row + 1,
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
    let js_import = Regex::new(r"(?m)^\s*import\s+.+$").expect("regex");
    let js_fn =
        Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+([a-zA-Z0-9_]+)\s*\(([^)]*)\)")
            .expect("regex");
    let js_class = Regex::new(r"(?m)^\s*(?:export\s+)?class\s+([a-zA-Z0-9_]+)").expect("regex");
    let js_arrow = Regex::new(
        r"(?m)^\s*(?:export\s+)?(?:const|let|var)\s+([a-zA-Z0-9_]+)\s*=\s*(?:async\s*)?\(([^)]*)\)\s*=>",
    )
    .expect("regex");
    let js_test =
        Regex::new(r#"(?m)^\s*(?:test|it|describe)\(\s*["']([^"']+)["']\s*,"#).expect("regex");
    let md_heading = Regex::new(r"(?m)^(#{1,3})\s+(.+?)\s*$").expect("regex");
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

    for captures in js_import.captures_iter(code) {
        let Some(m) = captures.get(0) else {
            continue;
        };
        let import = m.as_str().trim();
        out.push(RawSymbol {
            symbol: Symbol {
                file_path: file_path.to_string(),
                name: import.to_string(),
                kind: SymbolKind::Import,
                signature: import.to_string(),
            },
            start_byte: m.start(),
            end_byte: m.end(),
            start_line: line_of_byte(code, m.start()),
            end_line: line_of_byte(code, m.end()),
        });
    }

    for captures in js_fn.captures_iter(code) {
        let Some(m) = captures.get(0) else {
            continue;
        };
        let name = captures.get(1).map(|v| v.as_str()).unwrap_or_default();
        let args = captures.get(2).map(|v| v.as_str()).unwrap_or_default();
        out.push(RawSymbol {
            symbol: Symbol {
                file_path: file_path.to_string(),
                name: name.to_string(),
                kind: if is_js_test_name(name) {
                    SymbolKind::Test
                } else {
                    SymbolKind::Function
                },
                signature: format!("function {name}({args})"),
            },
            start_byte: m.start(),
            end_byte: m.end(),
            start_line: line_of_byte(code, m.start()),
            end_line: line_of_byte(code, m.end()),
        });
    }

    for captures in js_class.captures_iter(code) {
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

    for captures in js_arrow.captures_iter(code) {
        let Some(m) = captures.get(0) else {
            continue;
        };
        let name = captures.get(1).map(|v| v.as_str()).unwrap_or_default();
        let args = captures.get(2).map(|v| v.as_str()).unwrap_or_default();
        out.push(RawSymbol {
            symbol: Symbol {
                file_path: file_path.to_string(),
                name: name.to_string(),
                kind: if is_js_test_name(name) {
                    SymbolKind::Test
                } else {
                    SymbolKind::Function
                },
                signature: format!("const {name} = ({args}) =>"),
            },
            start_byte: m.start(),
            end_byte: m.end(),
            start_line: line_of_byte(code, m.start()),
            end_line: line_of_byte(code, m.end()),
        });
    }

    for captures in js_test.captures_iter(code) {
        let Some(m) = captures.get(0) else {
            continue;
        };
        let name = captures.get(1).map(|v| v.as_str()).unwrap_or_default();
        out.push(RawSymbol {
            symbol: Symbol {
                file_path: file_path.to_string(),
                name: name.to_string(),
                kind: SymbolKind::Test,
                signature: m.as_str().trim().to_string(),
            },
            start_byte: m.start(),
            end_byte: m.end(),
            start_line: line_of_byte(code, m.start()),
            end_line: line_of_byte(code, m.end()),
        });
    }

    if file_path.ends_with(".md") {
        for captures in md_heading.captures_iter(code) {
            let Some(m) = captures.get(0) else {
                continue;
            };
            let name = captures
                .get(2)
                .map(|v| v.as_str())
                .unwrap_or_default()
                .trim();
            if name.is_empty() {
                continue;
            }

            out.push(RawSymbol {
                symbol: Symbol {
                    file_path: file_path.to_string(),
                    name: name.to_string(),
                    kind: SymbolKind::Module,
                    signature: m.as_str().trim().to_string(),
                },
                start_byte: m.start(),
                end_byte: m.end(),
                start_line: line_of_byte(code, m.start()),
                end_line: line_of_byte(code, m.end()),
            });
        }
    }

    out
}

fn source_language_for_file(file_path: &str) -> Option<SourceLanguage> {
    if file_path.ends_with(".rs") {
        Some(SourceLanguage::Rust)
    } else if file_path.ends_with(".py") {
        Some(SourceLanguage::Python)
    } else if file_path.ends_with(".tsx") {
        Some(SourceLanguage::Tsx)
    } else if file_path.ends_with(".ts") {
        Some(SourceLanguage::TypeScript)
    } else if file_path.ends_with(".js")
        || file_path.ends_with(".jsx")
        || file_path.ends_with(".mjs")
        || file_path.ends_with(".cjs")
    {
        Some(SourceLanguage::JavaScript)
    } else if file_path.ends_with(".c") {
        Some(SourceLanguage::C)
    } else if file_path.ends_with(".h") {
        Some(SourceLanguage::CHeader)
    } else if file_path.ends_with(".cpp")
        || file_path.ends_with(".cc")
        || file_path.ends_with(".cxx") {
        Some(SourceLanguage::Cpp)
    } else if file_path.ends_with(".hpp")
        || file_path.ends_with(".hxx")
        || file_path.ends_with(".hh") {
        Some(SourceLanguage::CppHeader)
    } else {
        None
    }
}

fn extract_js_variable_symbols(
    code: &str,
    file_path: &str,
    declaration_node: Node<'_>,
    out: &mut Vec<RawSymbol>,
) {
    let mut cursor = declaration_node.walk();
    for child in declaration_node.children(&mut cursor) {
        if child.kind() != "variable_declarator" {
            continue;
        }
        let Some(name_node) = child.child_by_field_name("name") else {
            continue;
        };
        let Some(value_node) = child.child_by_field_name("value") else {
            continue;
        };
        if value_node.kind() != "arrow_function" && value_node.kind() != "function" {
            continue;
        }

        let name = node_text(code, name_node);
        let signature = first_line(node_text(code, declaration_node));
        let kind = if is_js_test_name(&name) {
            SymbolKind::Test
        } else {
            SymbolKind::Function
        };
        out.push(raw_symbol_with_span(
            file_path,
            &name,
            kind,
            &signature,
            declaration_node,
        ));
    }
}

fn extract_js_test_call(code: &str, file_path: &str, node: Node<'_>) -> Option<RawSymbol> {
    let function_node = node.child_by_field_name("function")?;
    let callee = node_text(code, function_node);
    if callee != "test" && callee != "it" && callee != "describe" {
        return None;
    }

    let arguments_node = node.child_by_field_name("arguments")?;
    let mut cursor = arguments_node.walk();
    let first_argument = arguments_node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "string")?;
    let raw_name = node_text(code, first_argument);
    let name = raw_name
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();
    let signature = first_line(node_text(code, node));
    Some(raw_symbol(
        file_path,
        &name,
        SymbolKind::Test,
        &signature,
        node,
    ))
}

fn is_js_test_name(name: &str) -> bool {
    name.starts_with("test") || name.ends_with("Test")
}

fn line_of_byte(code: &str, byte_idx: usize) -> usize {
    code[..byte_idx.min(code.len())]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1
}

// ─── C / Zephyr symbol extraction ────────────────────────────────────────────

fn extract_c_symbols(code: &str, tree: &Tree, file_path: &str) -> Vec<RawSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        match node.kind() {
            // Regular C functions
            "function_definition" => {
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    if let Some(name) = find_function_name(code, declarator) {
                        let signature = first_line(node_text(code, node));
                        let kind = if name.starts_with("test_") || name.starts_with("Test") {
                            SymbolKind::Test
                        } else {
                            SymbolKind::Function
                        };
                        symbols.push(raw_symbol(file_path, &name, kind, &signature, node));
                    }
                }
                // Don't recurse into function bodies
                continue;
            }

            // Structs
            "struct_specifier" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Class, &signature, node));
                }
            }

            // Enums
            "enum_specifier" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Class, &signature, node));
                }
            }

            // typedef — alias name is the last type_identifier
            "type_definition" => {
                let mut cursor = node.walk();
                let type_ids: Vec<_> = node
                    .children(&mut cursor)
                    .filter(|c| c.kind() == "type_identifier")
                    .collect();
                if let Some(name_node) = type_ids.last() {
                    let name = node_text(code, *name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Class, &signature, node));
                }
            }

            // #include
            "preproc_include" => {
                let import = first_line(node_text(code, node));
                symbols.push(raw_symbol(file_path, &import, SymbolKind::Import, &import, node));
            }

            // #define (simple constant macros)
            "preproc_def" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Function, &signature, node));
                }
            }

            // #define with parameters (function-like macros)
            "preproc_function_def" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    let kind = if is_zephyr_kernel_macro(&name) {
                        SymbolKind::Class
                    } else {
                        SymbolKind::Function
                    };
                    symbols.push(raw_symbol(file_path, &name, kind, &signature, node));
                }
            }

            // RTOS macro calls — at file scope and inside function bodies
            "expression_statement" | "declaration" | "call_expression" => {
                if let Some(rtos_sym) = extract_rtos_macro_call(code, file_path, node) {
                    symbols.push(rtos_sym);
                }
            }

            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    // Extract CONFIG_* Kconfig references and DT_* devicetree lookups
    // These appear anywhere in the file, not just at file scope
    extract_kconfig_refs(code, file_path, &mut symbols);
    extract_dt_refs(code, file_path, &mut symbols);
    // Full-tree scan for RTOS calls inside function bodies
    extract_rtos_calls_deep(code, tree, file_path, &mut symbols);

    symbols
}

/// Recursively find the innermost identifier from a declarator node.
/// Handles: foo, *foo, foo(...), (*foo)(...)
fn find_function_name(code: &str, node: Node<'_>) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(code, node)),
        "qualified_identifier" => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter(|c| c.kind() == "identifier")
                .last()
                .map(|n| node_text(code, n))
        }
        "function_declarator" | "pointer_declarator" | "parenthesized_declarator" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(name) = find_function_name(code, child) {
                    return Some(name);
                }
            }
            None
        }
        _ => None,
    }
}

/// RTOS kernel object definition macros — treated as type definitions.
/// Covers Zephyr static kernel objects and FreeRTOS static allocation structs.
fn is_zephyr_kernel_macro(name: &str) -> bool {
    matches!(
        name,
        // Zephyr kernel objects
        "K_THREAD_DEFINE"
            | "K_SEM_DEFINE"
            | "K_MUTEX_DEFINE"
            | "K_FIFO_DEFINE"
            | "K_LIFO_DEFINE"
            | "K_QUEUE_DEFINE"
            | "K_PIPE_DEFINE"
            | "K_MSGQ_DEFINE"
            | "K_MBOX_DEFINE"
            | "K_TIMER_DEFINE"
            | "K_MEM_SLAB_DEFINE"
            | "K_HEAP_DEFINE"
            | "K_STACK_DEFINE"
            | "K_POLL_EVENT_DEFINE"
            // FreeRTOS static allocation types used as global objects
            | "StaticTask_t"
            | "StaticQueue_t"
            | "StaticSemaphore_t"
            | "StaticTimer_t"
            | "StaticEventGroup_t"
            | "StaticStreamBuffer_t"
    )
}

/// Extract RTOS macro calls (Zephyr devicetree/driver registration + FreeRTOS task/queue/semaphore creation).
/// These appear as expression_statement or declaration at file scope.
fn extract_rtos_macro_call(code: &str, file_path: &str, node: Node<'_>) -> Option<RawSymbol> {
    let call = if node.kind() == "call_expression" {
        node
    } else {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .find(|c| c.kind() == "call_expression")?
    };

    let fn_node = call.child_by_field_name("function")?;
    let macro_name = node_text(code, fn_node);

    // Zephyr devicetree and driver registration macros
    let zephyr_macros = [
        "DEVICE_DT_DEFINE",
        "DEVICE_DT_INST_DEFINE",
        "DEVICE_DEFINE",
        "SYS_DEVICE_DEFINE",
        "SYS_INIT",
        "SHELL_CMD_REGISTER",
        "SHELL_SUBCMD_SET_CREATE",
        "SENSOR_DEVICE_DT_INST_DEFINE",
        "UART_DEVICE_CONFIG_DEFINE",
        "GPIO_DEVICE_INIT",
        "PINCTRL_DT_INST_DEFINE",
        "IRQ_CONNECT",
        "DEVICE_DT_DEFINE_WITH_PM",
    ];

    // Zephyr kernel object instantiation calls (K_*_DEFINE)
    let zephyr_kernel_macros = [
        "K_THREAD_DEFINE",
        "K_SEM_DEFINE",
        "K_MUTEX_DEFINE",
        "K_FIFO_DEFINE",
        "K_LIFO_DEFINE",
        "K_QUEUE_DEFINE",
        "K_PIPE_DEFINE",
        "K_MSGQ_DEFINE",
        "K_MBOX_DEFINE",
        "K_TIMER_DEFINE",
        "K_MEM_SLAB_DEFINE",
        "K_HEAP_DEFINE",
        "K_STACK_DEFINE",
        "K_POLL_EVENT_DEFINE",
    ];

    // FreeRTOS task, queue, semaphore, timer creation calls
    let freertos_macros = [
        "xTaskCreate",
        "xTaskCreateStatic",
        "xTaskCreatePinnedToCore",
        "xTaskCreateStaticPinnedToCore",
        "xQueueCreate",
        "xQueueCreateStatic",
        "xSemaphoreCreateBinary",
        "xSemaphoreCreateBinaryStatic",
        "xSemaphoreCreateMutex",
        "xSemaphoreCreateMutexStatic",
        "xSemaphoreCreateRecursiveMutex",
        "xSemaphoreCreateRecursiveMutexStatic",
        "xSemaphoreCreateCounting",
        "xSemaphoreCreateCountingStatic",
        "xTimerCreate",
        "xTimerCreateStatic",
        "xEventGroupCreate",
        "xEventGroupCreateStatic",
        "xStreamBufferCreate",
        "xStreamBufferCreateStatic",
        "xMessageBufferCreate",
        "xMessageBufferCreateStatic",
    ];

    let is_rtos_macro = zephyr_macros.contains(&macro_name.as_str())
        || zephyr_kernel_macros.contains(&macro_name.as_str())
        || freertos_macros.contains(&macro_name.as_str());

    if !is_rtos_macro {
        return None;
    }

    let args = call.child_by_field_name("arguments")?;
    let mut arg_cursor = args.walk();
    let device_name = args
        .named_children(&mut arg_cursor)
        .find(|c| c.kind() != ",")
        .map(|n| node_text(code, n))
        .unwrap_or_default();
    let signature = first_line(node_text(code, node));

    Some(raw_symbol(
        file_path,
        &format!("{macro_name}({device_name})"),
        SymbolKind::Class,
        &signature,
        node,
    ))
}

// ─── C++ symbol extraction ────────────────────────────────────────────────────

fn extract_cpp_symbols(code: &str, tree: &Tree, file_path: &str) -> Vec<RawSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        match node.kind() {
            // Free functions and methods
            "function_definition" => {
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    if let Some(name) = find_function_name(code, declarator) {
                        let signature = first_line(node_text(code, node));
                        let kind = if name.starts_with("test_") || name.starts_with("Test") {
                            SymbolKind::Test
                        } else {
                            SymbolKind::Function
                        };
                        symbols.push(raw_symbol(file_path, &name, kind, &signature, node));
                    }
                }
                continue;
            }

            // Classes and structs
            "class_specifier" | "struct_specifier" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Class, &signature, node));
                }
            }

            // Enums and enum classes
            "enum_specifier" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Class, &signature, node));
                }
            }

            // Templates
            "template_declaration" => {
                // Extract the inner class or function name
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "class_specifier" | "struct_specifier" => {
                            if let Some(name_node) = child.child_by_field_name("name") {
                                let name = node_text(code, name_node);
                                let signature = first_line(node_text(code, node));
                                symbols.push(raw_symbol(
                                    file_path,
                                    &format!("{name}<>"),
                                    SymbolKind::Class,
                                    &signature,
                                    node,
                                ));
                            }
                        }
                        "function_definition" => {
                            if let Some(declarator) = child.child_by_field_name("declarator") {
                                if let Some(name) = find_function_name(code, declarator) {
                                    let signature = first_line(node_text(code, node));
                                    symbols.push(raw_symbol(
                                        file_path,
                                        &format!("{name}<>"),
                                        SymbolKind::Function,
                                        &signature,
                                        node,
                                    ));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                continue;
            }

            // typedef
            "type_definition" => {
                let mut cursor = node.walk();
                let type_ids: Vec<_> = node
                    .children(&mut cursor)
                    .filter(|c| c.kind() == "type_identifier")
                    .collect();
                if let Some(name_node) = type_ids.last() {
                    let name = node_text(code, *name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Class, &signature, node));
                }
            }

            // Namespaces — index the name for context, don't skip children
            "namespace_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Module, &signature, node));
                }
            }

            // #include
            "preproc_include" => {
                let import = first_line(node_text(code, node));
                symbols.push(raw_symbol(file_path, &import, SymbolKind::Import, &import, node));
            }

            // #define
            "preproc_def" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Function, &signature, node));
                }
            }

            // #define with params
            "preproc_function_def" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(code, name_node);
                    let signature = first_line(node_text(code, node));
                    symbols.push(raw_symbol(file_path, &name, SymbolKind::Function, &signature, node));
                }
            }

            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    // Extract CONFIG_* Kconfig references and DT_* devicetree lookups
    extract_kconfig_refs(code, file_path, &mut symbols);
    extract_dt_refs(code, file_path, &mut symbols);

    symbols
}

// ─── Zephyr Kconfig and Devicetree reference extraction ──────────────────────

/// Extract CONFIG_* Kconfig symbol references from preprocessor directives
/// and identifier usage throughout the file.
fn extract_kconfig_refs(code: &str, file_path: &str, symbols: &mut Vec<RawSymbol>) {
    use regex::Regex;
    let re = Regex::new(r"\bCONFIG_[A-Z0-9_]+\b").expect("regex");

    let mut seen = std::collections::HashSet::new();
    for m in re.find_iter(code) {
        let name = m.as_str().to_string();
        if seen.contains(&name) {
            continue;
        }
        seen.insert(name.clone());
        let line = line_of_byte(code, m.start());
        symbols.push(RawSymbol {
            symbol: Symbol {
                file_path: file_path.to_string(),
                name: name.clone(),
                kind: SymbolKind::Import,
                signature: name,
            },
            start_byte: m.start(),
            end_byte: m.end(),
            start_line: line,
            end_line: line,
        });
    }
}

/// Extract DT_ALIAS, DT_NODELABEL, DT_PATH, DT_CHOSEN, DT_INST references.
/// These are devicetree lookup macros that identify hardware nodes.
fn extract_dt_refs(code: &str, file_path: &str, symbols: &mut Vec<RawSymbol>) {
    use regex::Regex;
    // Match DT_ALIAS(foo), DT_NODELABEL(bar), DT_PATH(...), DT_CHOSEN(...), DT_INST(n, compat)
    let re = Regex::new(
        r"\b(DT_ALIAS|DT_NODELABEL|DT_PATH|DT_CHOSEN|DT_INST|DT_PARENT|DT_CHILD)\s*\(([^)]{0,80})\)"
    ).expect("regex");

    let mut seen = std::collections::HashSet::new();
    for cap in re.captures_iter(code) {
        let full = cap.get(0).map(|m| m.as_str()).unwrap_or_default();
        let start = cap.get(0).map(|m| m.start()).unwrap_or(0);
        let end   = cap.get(0).map(|m| m.end()).unwrap_or(0);
        let name  = full.trim().to_string();

        if seen.contains(&name) {
            continue;
        }
        seen.insert(name.clone());

        let line = line_of_byte(code, start);
        symbols.push(RawSymbol {
            symbol: Symbol {
                file_path: file_path.to_string(),
                name: name.clone(),
                kind: SymbolKind::Import,
                signature: name,
            },
            start_byte: start,
            end_byte: end,
            start_line: line,
            end_line: line,
        });
    }
}


/// Full-tree scan for RTOS macro calls inside function bodies.
/// The main walker skips function body children with `continue`,
/// so this does a separate pass over the entire tree.
fn extract_rtos_calls_deep(code: &str, tree: &Tree, file_path: &str, symbols: &mut Vec<RawSymbol>) {
    let mut seen = std::collections::HashSet::new();
    let root = tree.root_node();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.kind() == "call_expression" {
            if let Some(sym) = extract_rtos_macro_call(code, file_path, node) {
                if seen.insert(sym.symbol.name.clone()) {
                    symbols.push(sym);
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}
