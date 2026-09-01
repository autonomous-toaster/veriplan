use std::io::Write;

use clap::{Parser, Subcommand};

// Re-exported at the bin root because `mod cli` references them via
// `crate::checker` / `crate::input`.
use veriplan::checker;
use veriplan::input;
use veriplan::lsp;

mod cli;
mod cmd_check;
mod cmd_init;
mod cmd_visualize;

/// Supported plan formats.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum Format {
    /// OpenSpec specification format (default).
    Openspec,
}

impl std::str::FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openspec" => Ok(Format::Openspec),
            other => Err(format!(
                "unknown format '{}'. Supported formats: openspec",
                other
            )),
        }
    }
}

#[derive(Parser)]
#[command(name = "veriplan", about = "Formal verification for OpenSpec plans")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run convertibility + model checking on a plan
    Check {
        /// Change name, file path, or directory. Use '-' for stdin. Omit to auto-detect.
        #[arg(required = false)]
        change: Option<String>,
        /// Alias for CHANGE (e.g., --change my-change)
        #[arg(long = "change", required = false)]
        change_alias: Option<String>,
        /// Stop after convertibility check (Phase 1)
        #[arg(long)]
        phase: Option<String>,
        /// Output format: human, json
        #[arg(long, default_value = "human")]
        format: Option<String>,
        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
        /// Pre-commit mode: missing SPIN is non-blocking, blockers exit 1, warnings exit 0
        #[arg(long)]
        pre_commit: bool,
        /// Read plan from stdin instead of a file
        #[arg(long)]
        stdin: bool,
        /// Strict checking: ungrounded patterns are blockers (default)
        #[arg(long)]
        strict: bool,
        /// Moderate checking: ungrounded patterns are warnings
        #[arg(long)]
        moderate: bool,
        /// Lax checking: ungrounded patterns are info
        #[arg(long)]
        lax: bool,
        /// Checker backend: spin (default) or spin-rs
        #[arg(long)]
        checker: Option<String>,
        /// Run both backends and compare results
        #[arg(long)]
        compare: bool,
        /// Apply machine-applicable (`local`) findings automatically
        #[arg(long)]
        fix: bool,
    },
    /// Init openspec/config.yaml with formal-verification-friendly rules
    Init {
        /// Project root (defaults to cwd)
        #[arg(long)]
        project_root: Option<String>,
    },
    /// Visualize the plan as a state-machine diagram
    Visualize {
        /// Change name (e.g., "veriplan-plan-verifier")
        #[arg(required = false)]
        change: Option<String>,
        /// Output format: mermaid, dot, or markdown (default: mermaid)
        #[arg(long, default_value = "mermaid")]
        format: Option<String>,
        /// Output file (omit for stdout)
        #[arg(short)]
        output: Option<String>,
    },
    /// Run the LSP server over stdio (for editor integration)
    Lsp {
        /// Use stdio transport
        #[arg(long)]
        stdio: bool,
    },
}

fn main() -> anyhow::Result<()> {
    if is_veriplan_disabled() {
        eprintln!("veriplan: disabled by VERIPLAN_DISABLE");
        return Ok(());
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Check {
            change,
            change_alias,
            phase,
            format,
            verbose: _verbose,
            pre_commit,
            stdin,
            strict,
            moderate,
            lax,
            checker,
            compare,
            fix,
        } => cmd_check::run_check(
            change.or(change_alias),
            phase.as_deref(),
            format.as_deref(),
            _verbose,
            pre_commit,
            stdin,
            strict,
            moderate,
            lax,
            checker,
            compare,
            fix,
        ),
        Commands::Init { project_root } => cmd_init::run_init(project_root.as_deref()),
        Commands::Visualize {
            change,
            format,
            output,
        } => cmd_visualize::run_visualize(change, format.as_deref(), output.as_deref()),
        Commands::Lsp { stdio: _stdio } => lsp::run_lsp(),
    };

    // Flush stdio before exiting to avoid losing buffered output
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    result
}

/// Check if veriplan should be disabled by the VERIPLAN_DISABLE env var.
/// Truthy values (1, true, yes, or any non-falsy non-empty string) disable.
/// Falsy values (0, false, no, empty) do not disable.
fn is_veriplan_disabled() -> bool {
    match std::env::var("VERIPLAN_DISABLE").as_deref() {
        Ok(v) => !matches!(v, "0" | "false" | "no" | ""),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_unset() {
        unsafe { std::env::remove_var("VERIPLAN_DISABLE") };
        assert!(!is_veriplan_disabled());
    }

    #[test]
    fn test_disabled_empty() {
        unsafe { std::env::set_var("VERIPLAN_DISABLE", "") };
        assert!(!is_veriplan_disabled());
    }

    #[test]
    fn test_disabled_zero() {
        unsafe { std::env::set_var("VERIPLAN_DISABLE", "0") };
        assert!(!is_veriplan_disabled());
    }

    #[test]
    fn test_disabled_false() {
        unsafe { std::env::set_var("VERIPLAN_DISABLE", "false") };
        assert!(!is_veriplan_disabled());
    }

    #[test]
    fn test_disabled_no() {
        unsafe { std::env::set_var("VERIPLAN_DISABLE", "no") };
        assert!(!is_veriplan_disabled());
    }

    #[test]
    fn test_disabled_one() {
        unsafe { std::env::set_var("VERIPLAN_DISABLE", "1") };
        assert!(is_veriplan_disabled());
    }

    #[test]
    fn test_disabled_true() {
        unsafe { std::env::set_var("VERIPLAN_DISABLE", "true") };
        assert!(is_veriplan_disabled());
    }

    #[test]
    fn test_disabled_yes() {
        unsafe { std::env::set_var("VERIPLAN_DISABLE", "yes") };
        assert!(is_veriplan_disabled());
    }

    #[test]
    fn test_disabled_arbitrary() {
        unsafe { std::env::set_var("VERIPLAN_DISABLE", "anything") };
        assert!(is_veriplan_disabled());
    }
}
