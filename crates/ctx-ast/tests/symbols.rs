use ctx_ast::{SymbolKind, extract_symbols, slice_symbols};

#[test]
fn extracts_rust_functions() {
    let code = r#"
fn validate_refresh_token() {}
pub fn decode_token(input: &str) -> bool { true }
"#;

    let symbols = extract_symbols(code, "src/auth.rs");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "validate_refresh_token" && s.kind == SymbolKind::Function)
    );
    assert!(symbols.iter().any(|s| s.name == "decode_token"));
}

#[test]
fn extracts_python_classes_and_methods() {
    let code = r#"
class AuthService:
    def validate(self):
        pass
"#;

    let symbols = extract_symbols(code, "src/auth.py");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "AuthService" && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "validate" && s.kind == SymbolKind::Function)
    );
}

#[test]
fn extracts_imports_and_tests_from_rust_with_tree_sitter() {
    let code = r#"
use crate::auth::decode_token;

struct AuthService;

#[test]
fn test_refresh_expired_token() {}
"#;

    let symbols = extract_symbols(code, "src/auth.rs");

    assert!(symbols.iter().any(|s| {
        s.kind == SymbolKind::Import && s.signature.contains("use crate::auth::decode_token")
    }));
    assert!(
        symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Class && s.name == "AuthService")
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Test && s.name == "test_refresh_expired_token")
    );
}

#[test]
fn structural_slices_keep_symbol_boundaries() {
    let code = r#"
fn first() {
    println!("first");
}

fn second() {
    println!("second");
}
"#;

    let slices = slice_symbols(code, "src/lib.rs", &["second"]);
    assert_eq!(slices.len(), 1);
    assert_eq!(slices[0].symbol_name, "second");
    assert!(slices[0].content.contains("fn second()"));
    assert!(!slices[0].content.contains("fn first()"));
}

#[test]
fn extracts_typescript_symbols_imports_and_tests() {
    let code = r#"
import { renderLogin } from "./auth";

export class AuthService {
  validateRefreshToken(input: string): boolean {
    return input.length > 0;
  }
}

export const helper = (value: string) => value.trim();

test("refresh token stays valid", () => {
  expect(renderLogin()).toBeTruthy();
});
"#;

    let symbols = extract_symbols(code, "src/auth.ts");
    assert!(symbols.iter().any(|s| {
        s.kind == SymbolKind::Import && s.signature.contains("import { renderLogin }")
    }));
    assert!(
        symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Class && s.name == "AuthService")
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Function && s.name == "validateRefreshToken")
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Function && s.name == "helper")
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Test && s.name == "refresh token stays valid")
    );
}

#[test]
fn extracts_javascript_slices_for_arrow_functions() {
    let code = r#"
import { fetchToken } from "./client.js";

const hydrateSession = () => {
  return fetchToken();
};

const cleanupSession = () => {
  return null;
};
"#;

    let slices = slice_symbols(code, "src/session.js", &["hydrateSession"]);
    assert_eq!(slices.len(), 1);
    assert_eq!(slices[0].symbol_name, "hydrateSession");
    assert!(slices[0].content.contains("const hydrateSession = () =>"));
    assert!(!slices[0].content.contains("cleanupSession"));
}
