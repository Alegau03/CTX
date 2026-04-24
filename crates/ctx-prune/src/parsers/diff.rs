use crate::{Candidate, PruneReport, heuristic, tokenize_query};

#[derive(Debug, Clone)]
struct DiffFile {
    order: usize,
    header: Vec<(usize, String)>,
    hunks: Vec<DiffHunk>,
    path_text: String,
}

#[derive(Debug, Clone)]
struct DiffHunk {
    header: (usize, String),
    lines: Vec<(usize, String)>,
}

pub(crate) fn prune_git_diff(input: &str, query: &str, max_lines: usize) -> PruneReport {
    let terms = tokenize_query(query);
    let files = parse_diff_files(input);
    let mut candidates = Vec::new();
    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let mut any_relevant = false;

    for file in &files {
        let mut relevant_hunks = Vec::new();
        for hunk in &file.hunks {
            if terms.is_empty() || score_hunk(file, hunk, &terms) > 0 {
                relevant_hunks.push(hunk.clone());
                any_relevant = true;
            }
        }

        if relevant_hunks.is_empty() {
            excluded.push(format!(
                "excluded diff file with no query overlap: {}",
                file.path_text
            ));
            continue;
        }

        push_file_header(file, &mut candidates);
        for hunk in relevant_hunks {
            candidates.push(Candidate::new(
                hunk.header.0,
                hunk.header.1,
                "git_diff:hunk_header_query_match",
                90,
            ));
            for (order, line) in hunk.lines {
                let priority = if is_changed_line(&line) { 85 } else { 50 };
                candidates.push(Candidate::new(
                    order,
                    line,
                    "git_diff:hunk_context",
                    priority,
                ));
            }
        }
        included.push(format!(
            "kept diff hunk due to query match: {}",
            file.path_text
        ));
    }

    if !any_relevant && !files.is_empty() {
        excluded.push(
            "no query-matching diff hunks found; keeping first hunk as fallback context"
                .to_string(),
        );
        let file = &files[0];
        push_file_header(file, &mut candidates);
        if let Some(hunk) = file.hunks.first() {
            candidates.push(Candidate::new(
                hunk.header.0,
                hunk.header.1.clone(),
                "git_diff:fallback_hunk_header",
                70,
            ));
            for (order, line) in hunk.lines.iter().take(max_lines.saturating_sub(1)) {
                candidates.push(Candidate::new(
                    *order,
                    line.clone(),
                    "git_diff:fallback_hunk_context",
                    50,
                ));
            }
        }
    }

    let mut report = heuristic::finalize_report(input, candidates, max_lines, "diff");
    report.included.extend(included);
    report.excluded.extend(excluded);
    report
}

fn parse_diff_files(input: &str) -> Vec<DiffFile> {
    let mut files = Vec::new();
    let mut current_file: Option<DiffFile> = None;
    let mut current_hunk: Option<DiffHunk> = None;

    for (order, raw) in input.lines().enumerate() {
        let line = raw.to_string();

        if line.starts_with("diff --git ") {
            flush_hunk(&mut current_file, &mut current_hunk);
            if let Some(file) = current_file.take() {
                files.push(file);
            }

            current_file = Some(DiffFile {
                order,
                header: vec![(order, line.clone())],
                hunks: Vec::new(),
                path_text: line,
            });
            continue;
        }

        if current_file.is_none() {
            continue;
        }

        if line.starts_with("@@ ") {
            flush_hunk(&mut current_file, &mut current_hunk);
            current_hunk = Some(DiffHunk {
                header: (order, line),
                lines: Vec::new(),
            });
            continue;
        }

        if let Some(hunk) = current_hunk.as_mut() {
            hunk.lines.push((order, line));
        } else if let Some(file) = current_file.as_mut() {
            if line.starts_with("--- ") || line.starts_with("+++ ") {
                file.path_text.push(' ');
                file.path_text.push_str(line.trim());
            }
            file.header.push((order, line));
        }
    }

    flush_hunk(&mut current_file, &mut current_hunk);
    if let Some(file) = current_file {
        files.push(file);
    }

    files
}

fn flush_hunk(current_file: &mut Option<DiffFile>, current_hunk: &mut Option<DiffHunk>) {
    if let (Some(file), Some(hunk)) = (current_file.as_mut(), current_hunk.take()) {
        file.hunks.push(hunk);
    }
}

fn push_file_header(file: &DiffFile, candidates: &mut Vec<Candidate>) {
    for (order, line) in &file.header {
        candidates.push(Candidate::new(
            *order,
            line.clone(),
            "git_diff:file_header",
            if *order == file.order { 95 } else { 80 },
        ));
    }
}

fn score_hunk(file: &DiffFile, hunk: &DiffHunk, terms: &[String]) -> usize {
    let mut haystack = String::new();
    haystack.push_str(&file.path_text.to_lowercase());
    haystack.push('\n');
    haystack.push_str(&hunk.header.1.to_lowercase());
    haystack.push('\n');
    for (_, line) in &hunk.lines {
        if is_changed_line(line) || line.starts_with(' ') {
            haystack.push_str(&line.to_lowercase());
            haystack.push('\n');
        }
    }

    terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count()
}

fn is_changed_line(line: &str) -> bool {
    (line.starts_with('+') && !line.starts_with("+++"))
        || (line.starts_with('-') && !line.starts_with("---"))
}
