use crate::args::Args;
use clap::CommandFactory;
use console::{Term, style};
use std::{num::ParseIntError, process::Output};

// renders clap's help text with the tool's own colour scheme by walking each line
// and recolouring the section headings ("Usage:", "Arguments:", "Options:") in place.
pub fn printhelp() {
    let mut cmd = Args::command();
    let helptext = cmd.render_help().to_string();
    let mut usagetext = String::new();

    for line in helptext.lines() {
        if line.starts_with("Usage:") {
            // capture usage once so we can repeat it at the bottom for quick reference
            usagetext = String::from(line.strip_prefix("Usage:").unwrap_or(line));
            important(&format!("usage: {}", usagetext));
        } else if line.starts_with("Arguments:") {
            important(&format!(
                "arguments: {}",
                line.strip_prefix("Arguments:").unwrap_or(line)
            ));
        } else if line.starts_with("Options:") {
            // options lines get extra formatting to bold the short flag portion
            println!(
                "{}",
                &format!(
                    "{} {}",
                    style("options:").cyan(),
                    style(formatoptionsline(
                        line.strip_prefix("Options:").unwrap_or(line).to_string()
                    ))
                )
            );
        } else {
            info(line);
        }
    }

    important(&format!("\nusage: {}", usagetext));
}

// bolds the short flag (up to the comma if present) and leaves the rest un-emphasised.
// example: "  -v, --verbose  verbose output" -> "**v**, --verbose  verbose output"
// todo: this parser is fragile — it assumes the first `-` starts the short flag
pub fn formatoptionsline(line: String) -> String {
    let mut result = String::new();
    let startidx: usize;
    let endidx: usize;
    if let Some(dashidx) = line.find('-') {
        if let Some(commaidx) = line.find(",") {
            startidx = dashidx + 1;
            endidx = startidx + commaidx - 1;
        } else {
            // no comma means only a long flag; fall through to bolding the whole line
            startidx = dashidx + 1;
            endidx = line.len();
        }
    } else {
        startidx = 0;
        endidx = 0;
    }

    result.push_str(&format!(
        "{}{}",
        style(line[startidx..endidx].trim().to_string()).bold(),
        style(line[endidx..].trim().to_string())
    ));

    result
}

// echoes the exact command about to run, so the user has a paper trail
pub fn printcommand(command: &Vec<&str>) {
    let msg = format!("  {}", style(command.join(" ")).cyan());
    println!("{}", msg);
}

// fallback renderer for command stdout — indented, line by line, styled as info
pub fn printcommandoutput(output: Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        for line in stdout.lines() {
            info(&format!("    {}", line));
        }
    }
}

// extracts the leading integer from strings like "3 files changed" or " 12 insertions(+)"
fn parsecount(s: &str) -> Result<i32, ParseIntError> {
    let trimmed = s.trim();
    let parts = trimmed.split_whitespace().collect::<Vec<&str>>();
    if let Some(countstr) = parts.first() {
        countstr.parse::<i32>()
    } else {
        // empty input: treat as zero rather than surfacing a parse error
        Ok(0)
    }
}

// parses `git commit -m` output into a compact summary line. falls back to raw output
// on any unexpected format so the user still sees something useful.
//
// git commit output looks roughly like:
//   [main abc1234] my commit message
//    2 files changed, 5 insertions(+), 3 deletions(-)
//    create mode 100644 path/to/newfile
pub fn printcommitoutput(output: Output, verbose: &u8) {
    debug("parsing commit command output", verbose);
    // clone stdout so we can still hand `output` to the fallback path if parsing fails
    let rawstdout = output.stdout.clone();
    let stdout = String::from_utf8_lossy(&rawstdout);
    let mut lines = stdout.lines();

    // first line contains "[branch hash] message" — extract branch and hash for the summary
    let firstline = lines.next().map(|s| s.trim()).unwrap_or("");
    let firstlineparts: Vec<&str> = firstline.split(' ').collect();

    let branchhashinfo = if firstlineparts.len() >= 2 {
        let branchpart = firstlineparts[0]
            .trim_start_matches('[')
            .trim_end_matches(']');
        let hashpart = firstlineparts[1].trim();
        format!("[branch: {}, hash: {}", branchpart, hashpart)
    } else {
        // unexpected first line; drop the branch/hash info rather than fail the whole render
        String::new()
    };

    // scan the remaining lines for the diffstat and file-mode lines
    let mut fileschangedline: Option<&str> = None;
    let mut modeline: Option<&str> = None;

    debug("searching output lines", verbose);
    for line in lines {
        // matches "N file(s) changed", "N insertion(s)", "N deletion(s)" in any combination
        if line.contains("files changed")
            || line.contains("file changed")
            || line.contains("insertions")
            || line.contains("insertion")
            || line.contains("deletions")
            || line.contains("deletion")
        {
            fileschangedline = Some(line);
        } else if line.contains("create mode") || line.contains("delete mode") {
            modeline = Some(line);
        }
    }
    debug("done", verbose);

    // without the diffstat line we can't produce a summary; hand off to the raw renderer
    if fileschangedline.is_none() {
        debug(
            &format!("raw stdout when required lines not found: {}", stdout),
            verbose,
        );
        debug(
            &format!("fileschangedline: {:?}", fileschangedline),
            verbose,
        );
        debug(&format!("modeline: {:?}", modeline), verbose);
        debug("falling back to printcommandoutput()", verbose);
        printcommandoutput(output);
        return;
    }

    debug("splitting files changed line", verbose);
    // defensive: the None case above already returned, but rebinding lets us use the inner &str
    let Some(fileschangedline) = fileschangedline else {
        debug(
            "fileschangedline was None unexpectedly (should have been caught by prior check). falling back.",
            verbose,
        );
        debug(&format!("modeline (Option): {:?}", modeline), verbose);
        debug(&format!("raw stdout: {}", stdout), verbose);
        printcommandoutput(output);
        return;
    };

    // diffstat pieces are comma-separated: "2 files changed, 5 insertions(+), 3 deletions(-)"
    let partsfileschanged: Vec<&str> = fileschangedline.split(", ").collect();
    let mut fileschangedcount = "0";

    if let Some(fileschangedpart) = partsfileschanged.first() {
        let parts = fileschangedpart.split_whitespace().collect::<Vec<&str>>();
        if parts.len() >= 1 {
            fileschangedcount = parts[0];
        }
    }

    // insertions/deletions may be absent (e.g. rename-only commits); default to zero
    let insertionspart = partsfileschanged
        .iter()
        .find(|&s| s.contains("insertion"))
        .unwrap_or(&"0 insertions(+/-)");
    let deletionspart = partsfileschanged
        .iter()
        .find(|&s| s.contains("deletion"))
        .unwrap_or(&"0 deletions(+/-)");

    let insertionsres = parsecount(insertionspart);
    let deletionsres = parsecount(deletionspart);

    // if either count fails to parse, drop back to the raw renderer instead of showing junk
    debug("checking errors", verbose);
    if let Err(e) = insertionsres {
        debug(
            &format!("raw stdout on insertion parse error: {}", stdout),
            verbose,
        );
        debug(&format!("error: {}", e), verbose);
        debug("falling back to printcommandoutput()", verbose);
        printcommandoutput(output);
        return;
    }

    if let Err(e) = deletionsres {
        debug(
            &format!("raw stdout on deletion parse error: {}", stdout),
            verbose,
        );
        debug(&format!("error: {}", e), verbose);
        debug("falling back to printcommandoutput()", verbose);
        printcommandoutput(output);
        return;
    }

    debug("getting insertions and deletions", verbose);
    let insertions = insertionsres.unwrap_or(0);
    let deletions = deletionsres.unwrap_or(0);

    debug("printing custom commit output", verbose);
    // first summary line: branch, hash, file count, and mode change (if any)
    info(&format!(
        "    {} {} file(s) changed{}",
        branchhashinfo,
        fileschangedcount,
        if let Some(modeline) = modeline {
            format!(", {}", modeline.trim())
        } else {
            String::new()
        }
    ));
    // second summary line: green insertions and red deletions, matching git's own colouring
    println!(
        "{}",
        format!(
            "    {}{}{}",
            style(format!("{} insertions (+)", insertions)).green(),
            style(", ").magenta(),
            style(format!("{} deletions (-)", deletions)).red()
        )
    );

    // trailing line: full "create mode ... <path>" details for newly added/removed files
    if let Some(modeline) = modeline {
        let modeparts = modeline.split_whitespace().collect::<Vec<&str>>();
        if modeparts.len() >= 3 {
            info(&format!(
                "    {} {} {}",
                modeparts[0],
                modeparts[1],
                modeparts[2..].join(" ")
            ));
        }
    }
}

// parses `git push` output into the three lines that actually matter to the user:
//   "To <remote-url>"     -> which remote was pushed to
//   "<local> -> <remote>" -> which refs moved
//   "Branch ... upstream" -> new upstream tracking info (only on --set-upstream)
// git writes most of this to stderr, so both streams are merged before scanning.
pub fn printpushoutput(output: Output, verbose: &u8) {
    debug("parsing push command output", verbose);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combinedoutput = format!("{}{}", stdout, stderr);

    let mut remoteline = None;
    let mut branchline = None;
    let mut upstreamline = None;
    let mut uptodate = false;

    for line in combinedoutput.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("To ") {
            remoteline = Some(trimmed.to_string());
        } else if trimmed.contains("->") {
            branchline = Some(trimmed.to_string());
        } else if trimmed.starts_with("Branch ") {
            upstreamline = Some(trimmed.to_string());
        } else if trimmed == "Everything up-to-date" {
            uptodate = true;
        }
    }

    // nothing to push; short-circuit so the summary doesn't look empty
    if uptodate {
        info("    Everything up-to-date");
        return;
    }

    // none of the expected lines matched (e.g. unusual push output); fall back to raw
    if remoteline.is_none() && branchline.is_none() && upstreamline.is_none() {
        debug("Could not parse push output, falling back", verbose);
        printcommandoutput(output);
        return;
    }

    if let Some(line) = remoteline {
        info(&format!("    {}", line));
    }
    if let Some(line) = branchline {
        info(&format!("    {}", line));
    }
    if let Some(line) = upstreamline {
        info(&format!("    {}", line));
    }
}

// unused error variant kept for future use; the leading underscore silences dead-code warnings
// todo: either wire this up or delete it
pub fn _fatalerror(error: &str) {
    let term = Term::stderr();
    term.write_line(&format!("{}", style("error: ").red()))
        .unwrap();
    term.write_line(&format!("  {}", style(error).red()))
        .unwrap();
    term.write_line(&format!(
        "{}",
        style("run `meow -h` for detailed help").red()
    ))
    .unwrap();
}

// styled output helpers. severity roughly corresponds to colour:
//   red   -> error, written to stderr
//   cyan  -> important (headings, banners)
//   magenta -> info (default running output)
//   blue  -> debug (only shown when -v is set at least once)
//   green -> success (stage completions)

pub fn error(text: &str) {
    let term = Term::stderr();
    term.write_line(&format!("{}", style(text).red())).unwrap();
}

pub fn important(text: &str) {
    println!("{}", style(text).cyan());
}

pub fn info(text: &str) {
    println!("{}", style(text).magenta());
}

// gated by verbosity so callers can leave debug! sprinkled through hot paths for free
pub fn debug(text: &str, verbose: &u8) {
    if verbose.to_owned() >= 1 {
        println!("[DEBUG] {}", style(text).blue())
    }
}

pub fn success(text: &str) {
    println!("{}", style(text).green());
}
