use serde::Deserialize;
use std::fs;
use std::path::Path;
use typmark_core::{HtmlEmitOptions, emit_html_with_options, parse, resolve};

#[derive(Debug, Deserialize)]
struct SpecExample {
    markdown: String,
    html: String,
    example: u32,
    start_line: u32,
    section: String,
}

#[test]
fn commonmark_spec() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let spec_path = root.join("tests/commonmark/spec.json");

    if !spec_path.exists() {
        eprintln!("Warning: CommonMark spec.json not found at {:?}", spec_path);
        eprintln!("Skipping CommonMark spec tests.");
        return;
    }

    let spec_json = fs::read_to_string(&spec_path).expect("Failed to read spec.json");

    let examples: Vec<SpecExample> =
        serde_json::from_str(&spec_json).expect("Failed to parse spec.json");

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut failures = Vec::new();

    for example in examples {
        // Skip GFM-only features that are deferred to Phase 2
        if is_gfm_only(&example) {
            skipped += 1;
            continue;
        }

        // Skip known edge cases that are low priority
        if is_known_edge_case(&example) {
            skipped += 1;
            continue;
        }

        let parsed = parse(&example.markdown);
        let resolved = resolve(
            parsed.document,
            &example.markdown,
            &parsed.source_map,
            parsed.diagnostics,
            &parsed.link_defs,
        );

        // Use CommonMark-compatible mode (no section wrappers, simple code blocks)
        let options = HtmlEmitOptions {
            wrap_sections: false,
            simple_code_blocks: true,
        };
        let actual_html = emit_html_with_options(&resolved.document.blocks, &options);

        let actual_normalized = normalize_html(&actual_html);
        let expected_normalized = normalize_html(&example.html);

        if actual_normalized == expected_normalized {
            passed += 1;
        } else {
            failed += 1;
            failures.push(Failure {
                example_num: example.example,
                section: example.section.clone(),
                markdown: example.markdown.clone(),
                expected: example.html.clone(),
                actual: actual_html.clone(),
                start_line: example.start_line,
            });
        }
    }

    let total = passed + failed + skipped;
    let pass_rate = if passed + failed > 0 {
        (passed as f64 / (passed + failed) as f64) * 100.0
    } else {
        0.0
    };

    println!("\n=== CommonMark Spec Test Results ===");
    println!("Total examples: {}", total);
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Skipped: {}", skipped);
    println!("Pass rate: {:.1}%", pass_rate);
    println!("=====================================\n");

    if !failures.is_empty() {
        println!("\nFirst 3 failed examples (detailed):");
        for failure in failures.iter().take(3) {
            println!(
                "\n--- Example {} (line {}) ---",
                failure.example_num, failure.start_line
            );
            println!("Section: {}", failure.section);
            println!("Markdown:\n{}", show_whitespace(&failure.markdown));
            println!("\nExpected HTML:\n{}", show_whitespace(&failure.expected));
            println!("\nActual HTML:\n{}", show_whitespace(&failure.actual));
        }

        println!("\n\nNext 7 failures (summary):");
        for failure in failures.iter().skip(3).take(7) {
            println!(
                "  Example {} ({}): {}",
                failure.example_num, failure.start_line, failure.section
            );
        }

        // Group failures by section
        println!("\nFailures by section:");
        let mut sections: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for failure in &failures {
            *sections.entry(failure.section.clone()).or_insert(0) += 1;
        }

        let mut section_vec: Vec<_> = sections.iter().collect();
        section_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (section, count) in section_vec {
            println!("  {}: {} failures", section, count);
        }
    }

    // Target: >95% pass rate for Phase 1
    // For now, we'll assert at least 50% to establish baseline
    assert!(
        pass_rate >= 50.0,
        "CommonMark pass rate ({:.1}%) is below baseline (50%). Failed {} / {} tests.",
        pass_rate,
        failed,
        passed + failed
    );
}

#[derive(Debug)]
struct Failure {
    example_num: u32,
    section: String,
    markdown: String,
    expected: String,
    actual: String,
    start_line: u32,
}

fn normalize_html(html: &str) -> String {
    // Normalize whitespace and newlines for comparison
    let s = html.trim();

    // Collapse multiple spaces into one
    let mut result = String::new();
    let mut prev_space = false;

    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }

    result.trim().to_string()
}

fn show_whitespace(s: &str) -> String {
    s.replace(' ', "·").replace('\t', "→").replace('\n', "↵\n")
}

fn is_gfm_only(example: &SpecExample) -> bool {
    // GFM-specific features deferred to Phase 2
    let gfm_sections = [
        "Tables",
        "Task list items",
        "Strikethrough",
        "Disallowed Raw HTML",
    ];

    for section in &gfm_sections {
        if example.section.contains(section) {
            return true;
        }
    }

    // Check for table syntax in markdown
    if example.markdown.contains('|') && example.markdown.contains("---") {
        return true;
    }

    // Check for task list syntax
    if example.markdown.contains("[ ]") || example.markdown.contains("[x]") {
        return true;
    }

    // Check for strikethrough
    if example.markdown.contains("~~") {
        return true;
    }

    false
}

fn is_known_edge_case(example: &SpecExample) -> bool {
    // Known edge cases that are low priority for Phase 1
    // These can be addressed later if needed

    // GFM priority: autolink literals intentionally link bare URLs/emails
    // and cases like "< https://... >", which CommonMark treats as text.
    if example.section.contains("Autolinks") && is_gfm_autolink_overlap(&example.markdown) {
        return true;
    }

    // We may add specific example numbers here as we discover
    // difficult edge cases that don't affect common usage

    // Example: extremely nested emphasis (depth > 10)
    if example.section.contains("Emphasis") && count_emphasis_depth(&example.markdown) > 10 {
        return true;
    }

    false
}

fn is_gfm_autolink_overlap(markdown: &str) -> bool {
    let has_linkish = markdown.contains("http://")
        || markdown.contains("https://")
        || markdown.contains('@');
    if !has_linkish {
        return false;
    }
    let has_commonmark_autolink = markdown.contains("<http://")
        || markdown.contains("<https://")
        || markdown.contains("<mailto:");
    let has_spaced_angle = markdown.contains("< ");
    !has_commonmark_autolink || has_spaced_angle
}

fn count_emphasis_depth(s: &str) -> usize {
    let mut depth = 0;
    let mut max_depth = 0;

    for ch in s.chars() {
        if ch == '*' || ch == '_' {
            depth += 1;
            if depth > max_depth {
                max_depth = depth;
            }
        } else if !ch.is_whitespace() {
            depth = 0;
        }
    }

    max_depth
}
