//! Test binary for AI diff analysis.
//!
//! Usage:
//!   cargo run --bin test_ai -- <base>..<head>         # Real diff (current dir)
//!   cargo run --bin test_ai -- <base>..<head> <repo>  # Real diff (specific repo)
//!
//! Examples:
//!   cargo run --bin test_ai -- HEAD~1..HEAD
//!   cargo run --bin test_ai -- main..HEAD ./my-repo

use staged_lib::{ai, git};
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    println!("=== Smart Diff AI Test ===\n");

    // Check for AI tool
    match ai::find_ai_tool() {
        Some(tool) => println!("✓ Found AI tool: {}\n", tool.name()),
        None => {
            eprintln!("✗ No AI tool found. Install goose or claude.");
            std::process::exit(1);
        }
    }

    match args.first().map(|s| s.as_str()) {
        None | Some("--help") | Some("-h") => print_help(),
        Some(range) => {
            let repo_path = args.get(1).map(|s| s.as_str()).unwrap_or(".");
            test_real_diff(range, repo_path);
        }
    }
}

fn print_help() {
    println!(
        r#"Usage:
  cargo run --bin test_ai -- <base>..<head>         # Real diff (current dir)
  cargo run --bin test_ai -- <base>..<head> <repo>  # Real diff (specific repo)

Examples:
  cargo run --bin test_ai -- HEAD~1..HEAD           # Last commit
  cargo run --bin test_ai -- HEAD~3..HEAD           # Last 3 commits
  cargo run --bin test_ai -- main..HEAD             # Changes since main
  cargo run --bin test_ai -- main..feature ~/repo   # Branch diff in specific repo
"#
    );
}

fn test_real_diff(range: &str, repo_path: &str) {
    // Parse base..head
    let parts: Vec<&str> = range.split("..").collect();
    if parts.len() != 2 {
        eprintln!("✗ Invalid range format. Use: base..head (e.g., HEAD~1..HEAD)");
        std::process::exit(1);
    }
    let (base, head) = (parts[0], parts[1]);

    let repo = Path::new(repo_path);
    println!(
        "Repository: {}",
        repo.canonicalize().unwrap_or(repo.to_path_buf()).display()
    );
    println!("Diff range: {}..{}\n", base, head);

    // Build DiffSpec
    let spec = git::DiffSpec {
        base: git::GitRef::Rev(base.to_string()),
        head: git::GitRef::Rev(head.to_string()),
    };

    // Run analysis - the backend handles file listing and content loading
    println!("Analyzing diff with AI (this may take a few seconds)...\n");

    match ai::analyze_diff(repo, &spec) {
        Ok(result) => {
            println!("═══════════════════════════════════════════════════════════════");
            println!("                     CHANGESET ANALYSIS");
            println!("═══════════════════════════════════════════════════════════════\n");
            println!("{}\n", result.summary);

            println!("Key Changes:");
            for change in &result.key_changes {
                println!("  • {}", change);
            }

            if !result.concerns.is_empty() {
                println!("\nConcerns:");
                for concern in &result.concerns {
                    println!("  ⚠ {}", concern);
                }
            }

            println!("\n───────────────────────────────────────────────────────────────");
            println!("                     FILE ANNOTATIONS");
            println!("───────────────────────────────────────────────────────────────\n");

            for (path, annotations) in &result.file_annotations {
                if annotations.is_empty() {
                    println!("{}: (no annotations)", path);
                } else {
                    println!("{}:", path);
                    for ann in annotations {
                        println!("\n  [{}] {:?}", ann.id, ann.category);
                        if let Some(ref span) = ann.before_span {
                            println!("    Before: lines {}-{}", span.start, span.end);
                        }
                        if let Some(ref span) = ann.after_span {
                            println!("    After: lines {}-{}", span.start, span.end);
                        }
                        println!("    {}", ann.content);
                    }
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("✗ Analysis failed: {}", e);
            std::process::exit(1);
        }
    }
}
