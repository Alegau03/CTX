use ctx_telemetry::{
    AuditEvent, PrivacyAuditEvent, append_audit_event, append_audit_line,
    append_privacy_audit_event,
};
use tempfile::tempdir;

#[test]
fn appends_human_readable_audit_line() {
    let tmp = tempdir().expect("tempdir");
    let audit_path = tmp.path().join(".ctx/audit.log");

    append_audit_line(&audit_path, "run_pack query=\"fix auth\" packed_tokens=200")
        .expect("append line");

    let body = std::fs::read_to_string(audit_path).expect("read audit");
    assert!(body.contains("run_pack"));
    assert!(body.contains("packed_tokens=200"));
}

#[test]
fn appends_structured_audit_event_as_json_line() {
    let tmp = tempdir().expect("tempdir");
    let audit_path = tmp.path().join(".ctx/audit.log");

    append_audit_event(
        &audit_path,
        &AuditEvent {
            kind: "adapter_invocation".to_string(),
            message: "ctx served opencode command".to_string(),
            agent: Some("opencode".to_string()),
            command: Some("/ctx-pack \"fix auth\"".to_string()),
            status: Some("succeeded".to_string()),
            fallback_used: false,
            pack_path: Some(".ctx/packs/1.json".to_string()),
        },
    )
    .expect("append event");

    let body = std::fs::read_to_string(audit_path).expect("read audit");
    assert!(body.contains("adapter_invocation"));
    assert!(body.contains("ctx served opencode command"));
    assert!(body.contains("\"fallback_used\":false"));
}

#[test]
fn appends_privacy_audit_event_as_json_line() {
    let tmp = tempdir().expect("tempdir");
    let audit_path = tmp.path().join(".ctx/audit.log");

    append_privacy_audit_event(
        &audit_path,
        &PrivacyAuditEvent {
            kind: "privacy_decision".to_string(),
            decision: "excluded".to_string(),
            path: Some(".env".to_string()),
            reason: "sensitive_pattern".to_string(),
            local_only: true,
            remote_upload_enabled: false,
            message: "blocked sensitive attachment".to_string(),
        },
    )
    .expect("append privacy event");

    let body = std::fs::read_to_string(audit_path).expect("read audit");
    assert!(body.contains("privacy_decision"));
    assert!(body.contains("\"decision\":\"excluded\""));
    assert!(body.contains("\"path\":\".env\""));
    assert!(body.contains("\"local_only\":true"));
    assert!(body.contains("\"remote_upload_enabled\":false"));
}
