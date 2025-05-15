use crate::args::Args;
use clap::CommandFactory;
use console::{Term, style};
use std::{num::ParseIntError, process::Output};

pub fn printhelp() {
    let mut cmd = Args::command();
    let helptext = cmd.render_help().to_string();

    for line in helptext.lines() {
        if line.starts_with("Usage:") {
            important(&format!(
                "usage: {}",
                line.strip_prefix("Usage:").unwrap_or(line)
            ));
        } else if line.starts_with("Arguments:") {
            important(&format!(
                "arguments: {}",
                line.strip_prefix("Arguments:").unwrap_or(line)
            ));
        } else if line.starts_with("Options:") {
            important(&format!(
                "options: {}",
                line.strip_prefix("Options:").unwrap_or(line)
            ));
        } else {
            info(line);
        }
    }
}

pub fn printcommand(command: &Vec<&str>) {
    println!("  {}", style(command.join(" ")).cyan());
}

pub fn printcommandoutput(output: Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        for line in stdout.lines() {
            info(&format!("    {}", line));
        }
    }
}

fn parsecount(s: &str) -> Result<i32, ParseIntError> {
    let trimmed = s.trim();
    let parts = trimmed.split_whitespace().collect::<Vec<&str>>();
    if let Some(countstr) = parts.first() {
        countstr.parse::<i32>()
    } else {
        Ok(0)
    }
}

pub fn printcommitoutput(output: Output, verbose: &u8) {
    debug("parsing commit command output", verbose);
    let rawstdout = output.stdout.clone();
    let stdout = String::from_utf8_lossy(&rawstdout);
    let mut lines = stdout.lines();

    let firstline = lines.next().map(|s| s.trim()).unwrap_or("");
    let firstlineparts: Vec<&str> = firstline.split(' ').collect();

    let branchhashinfo = if firstlineparts.len() >= 2 {
        let branchpart = firstlineparts[0]
            .trim_start_matches('[')
            .trim_end_matches(']');
        let hashpart = firstlineparts[1].trim();
        format!("[branch: {}, hash: {}", branchpart, hashpart)
    } else {
        String::new()
    };

    let mut fileschangedline: Option<&str> = None;
    let mut modeline: Option<&str> = None;

    debug("searching output lines", verbose);
    for line in lines {
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

    let partsfileschanged: Vec<&str> = fileschangedline.split(", ").collect();
    let mut fileschangedcount = "0";

    if let Some(fileschangedpart) = partsfileschanged.first() {
        let parts = fileschangedpart.split_whitespace().collect::<Vec<&str>>();
        if parts.len() >= 1 {
            fileschangedcount = parts[0];
        }
    }

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
    println!(
        "{}",
        format!(
            "    {}{}{}",
            style(format!("{} insertions (+)", insertions)).green(),
            style(", ").magenta(),
            style(format!("{} deletions (-)", deletions)).red()
        )
    );

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

pub fn debug(text: &str, verbose: &u8) -> String {
    if verbose.to_owned() >= 1 {
        format!("[DEBUG] {}", style(text).blue())
    } else {
        String::new()
    }
}

pub fn success(text: &str) -> String {
    format!("{}", style(text).green())
}
