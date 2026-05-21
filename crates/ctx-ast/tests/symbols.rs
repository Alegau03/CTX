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


// ─── C tests ─────────────────────────────────────────────────────────────────

#[test]
fn extracts_c_functions() {
    let code = r#"
#include <zephyr/kernel.h>

int gnss_init(void) {
    return 0;
}

static void gnss_callback(int event) {
    (void)event;
}

void *gnss_get_handle(void) {
    return NULL;
}
"#;

    let symbols = extract_symbols(code, "src/gnss.c");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss_init" && s.kind == SymbolKind::Function)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss_callback" && s.kind == SymbolKind::Function)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss_get_handle" && s.kind == SymbolKind::Function)
    );
}

#[test]
fn extracts_c_structs_and_enums() {
    let code = r#"
struct gnss_config {
    int interval_ms;
    int timeout_ms;
};

typedef struct {
    float lat;
    float lon;
    float alt;
} gnss_fix_t;

enum gnss_state {
    GNSS_IDLE,
    GNSS_SEARCHING,
    GNSS_FIXED,
};
"#;

    let symbols = extract_symbols(code, "src/gnss.h");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss_config" && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss_fix_t" && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss_state" && s.kind == SymbolKind::Class)
    );
}

#[test]
fn extracts_c_includes() {
    let code = r#"
#include <zephyr/kernel.h>
#include <zephyr/device.h>
#include "gnss.h"
"#;

    let symbols = extract_symbols(code, "src/main.c");
    assert!(
        symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Import && s.signature.contains("zephyr/kernel.h"))
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Import && s.signature.contains("gnss.h"))
    );
}

#[test]
fn extracts_c_pointer_function() {
    let code = r#"
int (*callback_fn)(void *ctx, int event);

static int *get_buffer(size_t len) {
    return NULL;
}
"#;

    let symbols = extract_symbols(code, "src/driver.c");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "get_buffer" && s.kind == SymbolKind::Function)
    );
}

#[test]
fn extracts_c_header_prototypes_and_inline_definitions() {
    let code = r#"
#ifndef GNSS_H
#define GNSS_H

int gnss_init(void);
void gnss_shutdown(void);

static inline int gnss_is_ready(int handle) {
    return handle > 0;
}

#endif
"#;

    let symbols = extract_symbols(code, "include/gnss.h");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss_init" && s.kind == SymbolKind::Function),
        "expected prototype gnss_init"
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss_shutdown" && s.kind == SymbolKind::Function),
        "expected prototype gnss_shutdown"
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss_is_ready" && s.kind == SymbolKind::Function),
        "expected inline definition gnss_is_ready"
    );
}

#[test]
fn extracts_cpp_header_inline_template_definitions() {
    let code = r#"
#pragma once

namespace ring {

template<typename T>
class Buffer {
public:
    bool push(const T &item) { return true; }
    inline size_t size() const { return m_size; }
private:
    size_t m_size = 0;
};

inline int clamp(int v, int lo, int hi) {
    return v < lo ? lo : (v > hi ? hi : v);
}

}
"#;

    let symbols = extract_symbols(code, "include/ring.hpp");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "push" && s.kind == SymbolKind::Function),
        "expected inline method push"
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "size" && s.kind == SymbolKind::Function),
        "expected inline method size"
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "clamp" && s.kind == SymbolKind::Function),
        "expected free inline function clamp"
    );
}

// ─── Zephyr tests ─────────────────────────────────────────────────────────────

#[test]
fn extracts_zephyr_kernel_objects() {
    let code = r#"
K_THREAD_DEFINE(gnss_thread, 1024, gnss_thread_fn, NULL, NULL, NULL, 5, 0, 0);
K_SEM_DEFINE(gnss_fix_sem, 0, 1);
K_MUTEX_DEFINE(gnss_mutex);
K_MSGQ_DEFINE(gnss_msgq, sizeof(struct gnss_fix), 10, 4);
"#;

    let symbols = extract_symbols(code, "src/gnss.c");
    // Kernel object macros are detected as function-like macro definitions
    assert!(symbols.iter().any(|s| s.name.starts_with("K_THREAD_DEFINE(") && s.kind == SymbolKind::Class));
    assert!(symbols.iter().any(|s| s.name.starts_with("K_SEM_DEFINE(") && s.kind == SymbolKind::Class));
    assert!(symbols.iter().any(|s| s.name.starts_with("K_MUTEX_DEFINE(") && s.kind == SymbolKind::Class));
    assert!(symbols.iter().any(|s| s.name.starts_with("K_MSGQ_DEFINE(") && s.kind == SymbolKind::Class));
}

#[test]
fn extracts_zephyr_device_registration() {
    let code = r#"
DEVICE_DT_DEFINE(DT_NODELABEL(gnss0),
    gnss_init,
    NULL,
    &gnss_data,
    &gnss_config,
    POST_KERNEL,
    CONFIG_GNSS_INIT_PRIORITY,
    &gnss_api);

SYS_INIT(board_init, PRE_KERNEL_1, 0);
"#;

    let symbols = extract_symbols(code, "src/gnss_driver.c");
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("DEVICE_DT_DEFINE") && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("SYS_INIT") && s.kind == SymbolKind::Class)
    );
}

#[test]
fn extracts_kconfig_references() {
    let code = r#"
#ifdef CONFIG_GNSS_ENABLE
    int interval = CONFIG_GNSS_POLL_INTERVAL_MS;
#endif

#if defined(CONFIG_MQTT_LIB_TLS)
    bool use_tls = true;
#endif
"#;

    let symbols = extract_symbols(code, "src/main.c");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "CONFIG_GNSS_ENABLE" && s.kind == SymbolKind::Import)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "CONFIG_GNSS_POLL_INTERVAL_MS" && s.kind == SymbolKind::Import)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "CONFIG_MQTT_LIB_TLS" && s.kind == SymbolKind::Import)
    );
}

#[test]
fn extracts_dt_devicetree_lookups() {
    let code = r#"
#define GNSS_NODE DT_ALIAS(gnss)
#define LED_NODE  DT_NODELABEL(led0)
#define SPI_NODE  DT_PATH(soc, spi_40003000)

static const struct device *gnss_dev = DEVICE_DT_GET(DT_ALIAS(gnss));
"#;

    let symbols = extract_symbols(code, "src/board.c");
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("DT_ALIAS(gnss)") && s.kind == SymbolKind::Import)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("DT_NODELABEL(led0)") && s.kind == SymbolKind::Import)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("DT_PATH(") && s.kind == SymbolKind::Import)
    );
}

// ─── FreeRTOS tests ───────────────────────────────────────────────────────────

#[test]
fn extracts_freertos_task_creation() {
    let code = r#"
void app_main(void) {
    xTaskCreate(gnss_task, "gnss", 4096, NULL, 5, &gnss_handle);
    xTaskCreateStatic(wifi_task, "wifi", 4096, NULL, 4, wifi_stack, &wifi_tcb);
}
"#;

    let symbols = extract_symbols(code, "src/main.c");
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("xTaskCreate(") && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("xTaskCreateStatic(") && s.kind == SymbolKind::Class)
    );
}

#[test]
fn extracts_freertos_sync_primitives() {
    let code = r#"
void init_sync(void) {
    gnss_sem   = xSemaphoreCreateBinary();
    gnss_mutex = xSemaphoreCreateMutex();
    gnss_queue = xQueueCreate(10, sizeof(gnss_fix_t));
    gnss_timer = xTimerCreate("gnss_timer", pdMS_TO_TICKS(1000), pdTRUE, 0, gnss_timer_cb);
}
"#;

    let symbols = extract_symbols(code, "src/sync.c");
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("xSemaphoreCreateBinary(") && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("xSemaphoreCreateMutex(") && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("xQueueCreate(") && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name.starts_with("xTimerCreate(") && s.kind == SymbolKind::Class)
    );
}

// ─── C++ tests ────────────────────────────────────────────────────────────────

#[test]
fn extracts_cpp_class_and_methods() {
    let code = r#"
class GnssManager {
public:
    GnssManager();
    ~GnssManager();
    bool init();
    bool getFix(float *lat, float *lon);
private:
    int m_handle;
};

bool GnssManager::init() {
    return true;
}

bool GnssManager::getFix(float *lat, float *lon) {
    return false;
}
"#;

    let symbols = extract_symbols(code, "src/gnss_manager.hpp");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "GnssManager" && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "init" && s.kind == SymbolKind::Function)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "getFix" && s.kind == SymbolKind::Function)
    );
}

#[test]
fn extracts_cpp_namespace() {
    let code = r#"
namespace wiskerwatcher {
namespace gnss {

class Fix {
public:
    float lat;
    float lon;
};

bool acquire(Fix &fix);

} // namespace gnss
} // namespace wiskerwatcher
"#;

    let symbols = extract_symbols(code, "src/gnss.hpp");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "wiskerwatcher" && s.kind == SymbolKind::Module)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "gnss" && s.kind == SymbolKind::Module)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "Fix" && s.kind == SymbolKind::Class)
    );
}

#[test]
fn extracts_cpp_template() {
    let code = r#"
template<typename T>
class RingBuffer {
public:
    bool push(const T &item);
    bool pop(T &item);
    size_t size() const;
private:
    T m_buf[64];
    size_t m_head = 0;
    size_t m_tail = 0;
};
"#;

    let symbols = extract_symbols(code, "src/ring_buffer.hpp");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "RingBuffer<>" && s.kind == SymbolKind::Class)
    );
}

#[test]
fn extracts_cpp_with_kconfig_refs() {
    let code = r#"
#include <zephyr/kernel.h>

namespace ww {

class MqttClient {
public:
    bool connect() {
#ifdef CONFIG_MQTT_LIB_TLS
        return connectTls(CONFIG_MQTT_BROKER_HOSTNAME, CONFIG_MQTT_BROKER_PORT_TLS);
#else
        return connectPlain(CONFIG_MQTT_BROKER_HOSTNAME, CONFIG_MQTT_BROKER_PORT);
#endif
    }
};

} // namespace ww
"#;

    let symbols = extract_symbols(code, "src/mqtt_client.cpp");
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "MqttClient" && s.kind == SymbolKind::Class)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "CONFIG_MQTT_LIB_TLS" && s.kind == SymbolKind::Import)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "CONFIG_MQTT_BROKER_HOSTNAME" && s.kind == SymbolKind::Import)
    );
}