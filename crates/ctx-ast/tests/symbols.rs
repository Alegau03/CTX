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

#[test]
fn extracts_markdown_headings_as_symbols() {
    let code = r#"
# Docker Compose

Intro text.

## Services

- api
- redis
"#;

    let symbols = extract_symbols(code, "docs/runbook.md");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "Docker Compose" && s.kind == SymbolKind::Module)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "Services" && s.kind == SymbolKind::Module)
    );
}

// ─── Swift / iOS / macOS ─────────────────────────────────────────────────────

#[test]
fn extracts_swift_class_struct_enum_protocol() {
    let code = r#"
import Foundation

protocol Greeter {
    func greet() -> String
}

class HomeViewController: UIViewController {
    func viewDidLoad() {}
}

struct User {
    let name: String
}

enum AuthState {
    case signedIn
    case signedOut
}

extension User {
    func displayName() -> String { name }
}
"#;

    let symbols = extract_symbols(code, "App/Home.swift");

    assert!(
        symbols
            .iter()
            .any(|s| s.name == "Greeter" && s.kind == SymbolKind::Class),
        "expected protocol Greeter, got {:?}",
        symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "HomeViewController" && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "User" && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "AuthState" && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Import && s.signature.contains("import Foundation"))
    );
}

#[test]
fn extracts_swift_functions_and_init() {
    let code = r#"
class TokenStore {
    init(name: String) {}
    deinit {}
    func refresh() -> Bool { true }
}

func freeStanding(value: Int) -> Int { value + 1 }
"#;

    let symbols = extract_symbols(code, "App/TokenStore.swift");

    assert!(
        symbols
            .iter()
            .any(|s| s.name == "init" && s.kind == SymbolKind::Function)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "deinit" && s.kind == SymbolKind::Function)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "refresh" && s.kind == SymbolKind::Function)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "freeStanding" && s.kind == SymbolKind::Function)
    );
}

#[test]
fn extracts_swift_xctest_methods_as_tests() {
    let code = r#"
import XCTest

class AuthServiceTests: XCTestCase {
    func testRefreshSucceedsWithValidToken() {}
    func testSignOutClearsState() {}
    func helperNotATest() {}
}
"#;

    let symbols = extract_symbols(code, "Tests/AuthServiceTests.swift");

    assert!(
        symbols
            .iter()
            .any(|s| s.name == "testRefreshSucceedsWithValidToken" && s.kind == SymbolKind::Test)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "testSignOutClearsState" && s.kind == SymbolKind::Test)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "helperNotATest" && s.kind == SymbolKind::Function)
    );
}

#[test]
fn extracts_apple_framework_imports() {
    let code = r#"
import UIKit
import SwiftUI
import Combine
import CoreData
"#;

    let symbols = extract_symbols(code, "App/Frameworks.swift");

    for fw in ["UIKit", "SwiftUI", "Combine", "CoreData"] {
        assert!(
            symbols.iter().any(|s| {
                s.kind == SymbolKind::Import && s.signature.contains(&format!("import {fw}"))
            }),
            "expected import {fw} as Import symbol"
        );
    }
}

#[test]
fn extracts_ios_macos_platform_gates() {
    let code = r#"
import Foundation

#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

#if canImport(SwiftUI)
import SwiftUI
#endif

@available(iOS 16, macOS 13, *)
func newAPI() {}
"#;

    let symbols = extract_symbols(code, "App/Platform.swift");

    assert!(
        symbols.iter().any(|s| {
            s.kind == SymbolKind::Import && s.name.contains("os(iOS)")
        }),
        "expected #if os(iOS) gate"
    );
    assert!(
        symbols.iter().any(|s| {
            s.kind == SymbolKind::Import && s.name.contains("os(macOS)")
        }),
        "expected #elseif os(macOS) gate (matched as #if os pattern? regex only matches #if)"
    );
    assert!(
        symbols.iter().any(|s| {
            s.kind == SymbolKind::Import && s.name.contains("canImport(SwiftUI)")
        }),
        "expected canImport(SwiftUI) gate"
    );
    assert!(
        symbols.iter().any(|s| {
            s.kind == SymbolKind::Import && s.name.starts_with("@available") && s.name.contains("iOS 16")
        }),
        "expected @available(iOS 16, macOS 13, *) gate"
    );
}

