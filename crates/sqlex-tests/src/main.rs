use clap::Parser;
use colored::*;
use sqlex_tests::{runner::TestSuiteResult, Result, TestRunner};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path relative to the fixtures directory (e.g., "sqlite" or "sqlite/select/basic.toml")
    /// Defaults to running all tests
    #[arg(default_value = "")]
    path: String,

    /// PostgreSQL connection URL
    #[arg(long, env = "SQLEX_TEST_POSTGRES_URL")]
    postgres_url: Option<String>,

    /// MySQL connection URL
    #[arg(long, env = "SQLEX_TEST_MYSQL_URL")]
    mysql_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut runner = TestRunner::new();
    
    // 1. Locate the fixtures root directory
    let mut fixtures_root = PathBuf::from("crates/sqlex-tests/fixtures");
    if !fixtures_root.exists() {
        fixtures_root = PathBuf::from("fixtures");
    }
    if !fixtures_root.exists() {
        // Try CARGO_MANIFEST_DIR
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if manifest.join("fixtures").exists() {
             fixtures_root = manifest.join("fixtures");
        }
    }

    if !fixtures_root.exists() {
        eprintln!("{}: Fixtures root directory not found. Expected at 'crates/sqlex-tests/fixtures' or 'fixtures'", "Error".red().bold());
        std::process::exit(1);
    }

    // 2. Resolve the target path relative to fixtures root
    let target_path = fixtures_root.join(&args.path);

    if !target_path.exists() {
        eprintln!("{}: Path not found: {:?} (resolved from fixtures root)", "Error".red().bold(), target_path);
        std::process::exit(1);
    }

    // println!("{} {}", "Running tests from:".green(), target_path.display());

    let mut results = Vec::new();
    
    // Collect test files
    let test_files: Vec<PathBuf> = if target_path.is_file() {
        vec![target_path]
    } else {
        walkdir::WalkDir::new(&target_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "toml"))
            .map(|e| e.path().to_path_buf())
            .collect()
    };

    if test_files.is_empty() {
        println!("{}", "No test files found.".yellow());
        return Ok(());
    }

    for path in test_files {
        // Calculate relative path for display
        let display_path = path.strip_prefix(&fixtures_root).unwrap_or(&path);
        println!("{} {}", "Running suite:".cyan().bold(), display_path.display());
        
        match runner.run_file(&path).await {
            Ok(result) => {
                print_suite_result(&result);
                results.push(result);
            }
            Err(e) => {
                eprintln!("  {} Failed to run suite: {}", "ERROR".red().bold(), e);
            }
        }
    }

    let stats = calculate_stats(&results);
    println!("\n{}", "Test Summary".bold().underline());
    println!("  Passed: {}", stats.passed.to_string().green());
    println!("  Failed: {}", stats.failed.to_string().red());
    
    if stats.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn print_suite_result(result: &TestSuiteResult) {
    for test_case in &result.results {
        if !test_case.passed {
            println!("  {} {}", "FAIL".red().bold(), test_case.name);
            
            // Print error message
            if let Some(err) = &test_case.error {
                println!("    {} {}", "Error:".red(), err.as_str());
            }

            // Print detailed diff for mismatch
            if let (Some(db), Some(sqlex)) = (&test_case.db_metadata, &test_case.sqlex_metadata) {
                 println!("\n    {}", "Metadata Mismatch Details".yellow().bold());
                 println!("    {:<20} {:<20} {:<20}", "Column", "Real DB", "Sqlex Analyzer");
                 println!("    {:<20} {:<20} {:<20}", "------", "-------", "--------------");
                 
                 let max_cols = std::cmp::max(db.columns.len(), sqlex.columns.len());
                 for i in 0..max_cols {
                     let db_col = db.columns.get(i);
                     let sqlex_col = sqlex.columns.get(i);
                     
                     match (db_col, sqlex_col) {
                         (Some(d), Some(s)) => {
                             let name_match = d.name.to_lowercase() == s.name.to_lowercase();
                             // let type_match = d.type_name == s.type_name; // Strict type matching not yet implemented
                             let null_match = d.nullable == s.nullable;
                             
                             let status = if name_match && null_match { "OK".green() } else { "DIFF".red() };
                             
                             println!("    {:<20} {:<20} {:<20} {}", 
                                 d.name,
                                 format!("{} ({})", d.type_name, if d.nullable { "NULL" } else { "NOT NULL" }),
                                 format!("{} ({})", s.type_name, if s.nullable { "NULL" } else { "NOT NULL" }),
                                 status
                             );
                         }
                         (Some(d), None) => {
                             println!("    {:<20} {:<20} {:<20} {}", d.name, "Present", "Missing", "DIFF".red());
                         }
                         (None, Some(s)) => {
                             println!("    {:<20} {:<20} {:<20} {}", s.name, "Missing", "Present", "DIFF".red());
                         }
                         (None, None) => unreachable!(),
                     }
                 }
            }
            
            // Print SQL context on failure or verbose
            println!("\n    {}", "Query context:".dimmed());
            // Note: We don't have easy access to the query string here in TestCaseResult unless we add it
            // For now, we rely on the error message which usually contains context
        } else {
            println!("  {} {}", "PASS".green(), test_case.name);
        }
    }
}

struct Stats {
    passed: usize,
    failed: usize,
}

fn calculate_stats(results: &[TestSuiteResult]) -> Stats {
    let mut passed = 0;
    let mut failed = 0;
    for suite in results {
        passed += suite.passed_count();
        failed += suite.failed_count();
    }
    Stats { passed, failed }
}
