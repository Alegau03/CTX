use crate::Candidate;
use regex::Regex;

pub(crate) fn parse_log_candidates(input: &str) -> Vec<Candidate> {
    let lines = input.lines().collect::<Vec<_>>();
    let mut candidates = Vec::new();

    parse_python_tracebacks(&lines, &mut candidates);
    parse_pytest(&lines, &mut candidates);
    parse_vitest_and_jest(&lines, &mut candidates);
    parse_typescript_and_eslint(&lines, &mut candidates);
    parse_python_linters(&lines, &mut candidates);
    parse_cargo(&lines, &mut candidates);
    parse_go_test(&lines, &mut candidates);
    parse_npm(&lines, &mut candidates);
    parse_git_status(&lines, &mut candidates);
    parse_generic_diagnostics(&lines, &mut candidates);

    candidates.sort_by_key(|candidate| candidate.order);
    candidates
}

fn parse_python_tracebacks(lines: &[&str], candidates: &mut Vec<Candidate>) {
    let exception = Regex::new(
        r"^\s*(?:[\w.]+(?:Error|Exception|Warning|Interrupt|Exit)|AssertionError|KeyboardInterrupt|SystemExit)(?::|$)",
    )
    .expect("valid regex");
    let mut in_traceback = false;
    let mut keep_next_code_line = false;

    for (idx, raw) in lines.iter().enumerate() {
        let line = raw.trim_end();
        let trimmed = line.trim_start();

        if trimmed.starts_with("Traceback (most recent call last):")
            || trimmed.starts_with("Traceback:")
        {
            in_traceback = true;
            keep_next_code_line = false;
            candidates.push(Candidate::new(idx, trimmed, "python_traceback:header", 100));
            continue;
        }

        if !in_traceback {
            continue;
        }

        if trimmed.starts_with("File ") {
            keep_next_code_line = true;
            candidates.push(Candidate::new(idx, trimmed, "python_traceback:frame", 95));
            continue;
        }

        if exception.is_match(trimmed) || trimmed.starts_with("E   ") {
            candidates.push(Candidate::new(
                idx,
                trimmed,
                "python_traceback:root_cause",
                100,
            ));
            in_traceback = false;
            keep_next_code_line = false;
            continue;
        }

        if keep_next_code_line && !trimmed.is_empty() {
            candidates.push(Candidate::new(idx, trimmed, "python_traceback:code", 70));
            keep_next_code_line = false;
        }
    }
}

fn parse_pytest(lines: &[&str], candidates: &mut Vec<Candidate>) {
    let failed_node =
        Regex::new(r"^(FAILED|ERROR)\s+[^\s]+::[^\s]+(?:\s+-\s+.*)?$").expect("valid regex");
    let section =
        Regex::new(r"^=+\s+(FAILURES|ERRORS|short test summary info)\s+=+$").expect("valid regex");
    let failure_header = Regex::new(r"^_+\s+[^\s].*\s+_+$").expect("valid regex");

    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let reason_priority = if failed_node.is_match(trimmed) {
            Some(("pytest:failed_node", 100))
        } else if section.is_match(trimmed) {
            Some(("pytest:section", 80))
        } else if failure_header.is_match(trimmed) && trimmed.to_lowercase().contains("test") {
            Some(("pytest:failure_header", 90))
        } else if trimmed.starts_with("E   ") || trimmed.starts_with("E       ") {
            Some(("pytest:assertion", 100))
        } else if trimmed.contains("ERROR at setup") || trimmed.contains("ERROR at teardown") {
            Some(("pytest:fixture_error", 100))
        } else if trimmed.contains("assert ") && !trimmed.starts_with("PASS") {
            Some(("pytest:assert_context", 85))
        } else {
            None
        };

        if let Some((reason, priority)) = reason_priority {
            candidates.push(Candidate::new(idx, trimmed, reason, priority));
        }
    }
}

fn parse_vitest_and_jest(lines: &[&str], candidates: &mut Vec<Candidate>) {
    let vitest_failed_test = Regex::new(r"^\s*[×x]\s+.+\d+ms$").expect("valid regex");
    let vitest_assertion = Regex::new(r"^\s*→\s+").expect("valid regex");
    let jest_fail = Regex::new(r"^\s*FAIL\s+.+").expect("valid regex");
    let jest_detail = Regex::new(r"^\s*(●|expect\(|Received:)").expect("valid regex");

    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim_end();
        let compact = trimmed.trim();

        if vitest_failed_test.is_match(compact) {
            candidates.push(Candidate::new(idx, compact, "vitest:failed_test", 100));
        } else if vitest_assertion.is_match(compact) {
            candidates.push(Candidate::new(idx, compact, "vitest:assertion", 95));
        } else if jest_fail.is_match(compact) {
            candidates.push(Candidate::new(idx, compact, "jest:failed_suite", 100));
        } else if jest_detail.is_match(compact) {
            candidates.push(Candidate::new(idx, compact, "jest:detail", 90));
        }
    }
}

fn parse_typescript_and_eslint(lines: &[&str], candidates: &mut Vec<Candidate>) {
    let tsc = Regex::new(r"^[^\s].*\.(?:ts|tsx|js|jsx)\(\d+,\d+\):\s+error\s+TS\d+:")
        .expect("valid regex");
    let eslint_row =
        Regex::new(r"^\s*\d+:\d+\s+(error|warning)\s+.+\s+[@\w/-]+$").expect("valid regex");
    let eslint_summary = Regex::new(r"^\s*\d+\s+problems?\s+\(\d+\s+errors?,\s+\d+\s+warnings?\)")
        .expect("valid regex");
    let mut previous_was_js_file = false;

    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim_end();
        let compact = trimmed.trim();

        if tsc.is_match(compact) {
            candidates.push(Candidate::new(idx, compact, "tsc:error", 100));
            previous_was_js_file = false;
            continue;
        }

        if compact.starts_with("Found ") && compact.contains(" error") {
            candidates.push(Candidate::new(idx, compact, "tsc:summary", 80));
            previous_was_js_file = false;
            continue;
        }

        if compact.ends_with(".js")
            || compact.ends_with(".jsx")
            || compact.ends_with(".ts")
            || compact.ends_with(".tsx")
        {
            previous_was_js_file = true;
            candidates.push(Candidate::new(idx, compact, "eslint:file", 70));
            continue;
        }

        if eslint_row.is_match(trimmed) {
            let priority = if trimmed.contains(" error ") { 100 } else { 60 };
            candidates.push(Candidate::new(idx, compact, "eslint:diagnostic", priority));
            continue;
        }

        if previous_was_js_file && eslint_summary.is_match(compact) {
            candidates.push(Candidate::new(idx, compact, "eslint:summary", 85));
            previous_was_js_file = false;
        }
    }
}

fn parse_python_linters(lines: &[&str], candidates: &mut Vec<Candidate>) {
    let ruff = Regex::new(r"^[^\s].*\.py:\d+:\d+:\s+[A-Z]+\d+\s+").expect("valid regex");
    let mypy = Regex::new(r"^[^\s].*\.py:\d+(?::\d+)?:\s+(error|note):\s+").expect("valid regex");

    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if ruff.is_match(trimmed) {
            candidates.push(Candidate::new(idx, trimmed, "ruff:diagnostic", 95));
        } else if mypy.is_match(trimmed) {
            let priority = if trimmed.contains(": error:") {
                100
            } else {
                50
            };
            candidates.push(Candidate::new(idx, trimmed, "mypy:diagnostic", priority));
        } else if trimmed.starts_with("Found ") && trimmed.contains(" error") {
            candidates.push(Candidate::new(idx, trimmed, "python_linter:summary", 80));
        }
    }
}

fn parse_cargo(lines: &[&str], candidates: &mut Vec<Candidate>) {
    let location = Regex::new(r"^\s*-->\s+[^\s]+:\d+:\d+").expect("valid regex");

    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if trimmed.starts_with("error[") || trimmed.starts_with("error:") {
            candidates.push(Candidate::new(idx, trimmed, "cargo:error", 100));
        } else if trimmed.starts_with("warning:") {
            candidates.push(Candidate::new(idx, trimmed, "cargo:warning", 60));
        } else if location.is_match(raw) {
            candidates.push(Candidate::new(idx, trimmed, "cargo:location", 90));
        } else if trimmed.starts_with("=")
            && (trimmed.contains("note:") || trimmed.contains("help:"))
        {
            candidates.push(Candidate::new(idx, trimmed, "cargo:note", 60));
        } else if trimmed.contains("panicked at") || trimmed.starts_with("thread '") {
            candidates.push(Candidate::new(idx, trimmed, "cargo:panic", 100));
        }
    }
}

fn parse_go_test(lines: &[&str], candidates: &mut Vec<Candidate>) {
    let fail_header = Regex::new(r"^---\s+FAIL:\s+").expect("valid regex");
    let go_location = Regex::new(r"^\s*[^\s]+\.go:\d+:\s+").expect("valid regex");

    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if fail_header.is_match(trimmed) {
            candidates.push(Candidate::new(idx, trimmed, "go_test:failed_test", 100));
        } else if trimmed.starts_with("FAIL") || trimmed.starts_with("panic:") {
            candidates.push(Candidate::new(idx, trimmed, "go_test:root_cause", 100));
        } else if go_location.is_match(trimmed) {
            candidates.push(Candidate::new(idx, trimmed, "go_test:location", 90));
        } else if trimmed.starts_with("Error Trace:")
            || trimmed.starts_with("Error:")
            || trimmed.starts_with("Messages:")
        {
            candidates.push(Candidate::new(idx, trimmed, "go_test:testify", 85));
        }
    }
}

fn parse_npm(lines: &[&str], candidates: &mut Vec<Candidate>) {
    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if trimmed.starts_with("npm ERR!") || trimmed.starts_with("ERR!") {
            candidates.push(Candidate::new(idx, trimmed, "npm:error", 100));
        } else if trimmed.starts_with("yarn error") {
            candidates.push(Candidate::new(idx, trimmed, "yarn:error", 100));
        } else if trimmed.starts_with("pnpm ERR!") || trimmed.contains("ERR_PNPM_") {
            candidates.push(Candidate::new(idx, trimmed, "pnpm:error", 100));
        } else if trimmed.starts_with("error: GET http") || trimmed.starts_with("error: GET https")
        {
            candidates.push(Candidate::new(idx, trimmed, "bun:error", 100));
        } else if trimmed.contains("ERESOLVE") || trimmed.contains("ELIFECYCLE") {
            candidates.push(Candidate::new(idx, trimmed, "npm:resolution", 100));
        } else if trimmed.to_lowercase().contains("deprecated") {
            candidates.push(Candidate::new(idx, trimmed, "npm:deprecated", 40));
        }
    }
}

fn parse_git_status(lines: &[&str], candidates: &mut Vec<Candidate>) {
    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if trimmed.starts_with("On branch ")
            || trimmed.starts_with("Your branch ")
            || trimmed.starts_with("Changes to be committed")
            || trimmed.starts_with("Changes not staged")
            || trimmed.starts_with("Untracked files")
            || trimmed.starts_with("both modified:")
            || trimmed.starts_with("modified:")
            || trimmed.starts_with("deleted:")
            || trimmed.starts_with("renamed:")
            || trimmed.starts_with("new file:")
            || trimmed.starts_with("unmerged:")
        {
            candidates.push(Candidate::new(idx, trimmed, "git_status:state", 75));
        }
    }
}

fn parse_generic_diagnostics(lines: &[&str], candidates: &mut Vec<Candidate>) {
    let generic = Regex::new(r"(?i)\b(error|failed|failure|exception|panic|traceback)\b")
        .expect("valid regex");
    let warning = Regex::new(r"(?i)\bwarning\b").expect("valid regex");

    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        if generic.is_match(trimmed) {
            candidates.push(Candidate::new(idx, trimmed, "generic:diagnostic", 70));
        } else if warning.is_match(trimmed) {
            candidates.push(Candidate::new(idx, trimmed, "generic:warning", 45));
        }
    }
}
