// Side-by-side diff viewer for conflict resolution
// Implements: task 6.6

use std::fmt;

use anyhow::{Context, Result};
use console::style;
use inquire::Select;

use crate::i18n::Msg;

#[allow(unused_imports)]
use crate::t;

// ---------------------------------------------------------------------------
// Resolution enum
// ---------------------------------------------------------------------------

/// How the user wants to resolve a conflicting file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Keep the local version (ours).
    KeepLocal,
    /// Accept the remote version (theirs).
    UseRemote,
    /// Open the file in the user's $EDITOR for manual editing.
    OpenEditor,
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Resolution::KeepLocal => write!(f, "{}", t!(Msg::DiffKeepLocal)),
            Resolution::UseRemote => write!(f, "{}", t!(Msg::DiffUseRemote)),
            Resolution::OpenEditor => write!(f, "{}", t!(Msg::DiffOpenEditor)),
        }
    }
}

// ---------------------------------------------------------------------------
// show_diff
// ---------------------------------------------------------------------------

/// Display a unified-style diff of two file versions.
///
/// Lines unique to `local_content` are shown in red (prefixed with `-`),
/// lines unique to `remote_content` are shown in green (prefixed with `+`),
/// and common lines are shown normally.
pub fn show_diff(local_content: &str, remote_content: &str, filename: &str) {
    println!();
    println!(
        "{}",
        style(t!(Msg::DiffConflictHeader { file: filename.to_string() })).bold().red()
    );
    println!("{}", t!(Msg::DiffLocalRemote));
    println!();

    let local_lines: Vec<&str> = local_content.lines().collect();
    let remote_lines: Vec<&str> = remote_content.lines().collect();

    // Simple line-by-line diff using longest-common-subsequence approach.
    // For a CLI tool this is good enough; we don't need a full Myers diff.
    let lcs = compute_lcs(&local_lines, &remote_lines);

    let mut li = 0usize;
    let mut ri = 0usize;
    let mut ci = 0usize;

    while li < local_lines.len() || ri < remote_lines.len() {
        if ci < lcs.len()
            && li < local_lines.len()
            && ri < remote_lines.len()
            && local_lines[li] == lcs[ci]
            && remote_lines[ri] == lcs[ci]
        {
            // Common line.
            println!("  {}", local_lines[li]);
            li += 1;
            ri += 1;
            ci += 1;
        } else {
            // Emit removed lines from local until we hit the next common line.
            while li < local_lines.len()
                && (ci >= lcs.len() || local_lines[li] != lcs[ci])
            {
                println!("{}", style(format!("- {}", local_lines[li])).red());
                li += 1;
            }
            // Emit added lines from remote until we hit the next common line.
            while ri < remote_lines.len()
                && (ci >= lcs.len() || remote_lines[ri] != lcs[ci])
            {
                println!("{}", style(format!("+ {}", remote_lines[ri])).green());
                ri += 1;
            }
        }
    }

    println!();
}

/// Compute the longest common subsequence of two string-slice vectors.
fn compute_lcs<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    let m = a.len();
    let n = b.len();

    // Build the DP table.
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to recover the subsequence.
    let mut result = Vec::with_capacity(dp[m][n]);
    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push(a[i - 1]);
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

// ---------------------------------------------------------------------------
// choose_resolution
// ---------------------------------------------------------------------------

/// Present a prompt for the user to choose how to resolve a conflict.
pub fn choose_resolution() -> Result<Resolution> {
    let options = vec![
        Resolution::KeepLocal,
        Resolution::UseRemote,
        Resolution::OpenEditor,
    ];

    let prompt = t!(Msg::DiffResolvePrompt);
    let choice = Select::new(&prompt, options)
        .with_help_message("Choose a resolution strategy")
        .prompt()
        .with_context(|| t!(Msg::DiffResolveCancelled))?;

    Ok(choice)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_lcs_identical() {
        let a = vec!["a", "b", "c"];
        let b = vec!["a", "b", "c"];
        let lcs = compute_lcs(&a, &b);
        assert_eq!(lcs, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_compute_lcs_disjoint() {
        let a = vec!["a", "b"];
        let b = vec!["c", "d"];
        let lcs = compute_lcs(&a, &b);
        assert!(lcs.is_empty());
    }

    #[test]
    fn test_compute_lcs_partial() {
        let a = vec!["a", "b", "c", "d"];
        let b = vec!["a", "x", "c", "y"];
        let lcs = compute_lcs(&a, &b);
        assert_eq!(lcs, vec!["a", "c"]);
    }

    #[test]
    fn test_compute_lcs_empty() {
        let a: Vec<&str> = vec![];
        let b = vec!["a"];
        assert!(compute_lcs(&a, &b).is_empty());
        assert!(compute_lcs(&b, &a).is_empty());
        assert!(compute_lcs(&a, &a).is_empty());
    }
}
