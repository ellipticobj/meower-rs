use crate::args::Args;
use clap::CommandFactory;
use console::style;
use std::{num::ParseIntError, process::Output};

pub fn printhelp() -> String {
    let mut cmd = Args::command();
    let helptext = cmd.render_help().to_string();

    let mut out = vec![];
    let indentation = 0;

    for line in helptext.lines() {
        if line.starts_with("Usage:") {
            println!("{}", important(&format!(
                "usage: {}",
                line.strip_prefix("Usage:").unwrap_or(line)
            )));
        } else if line.starts_with("Arguments:") {
            println!("{}", important(&format!(
                "arguments: {}",
                line.strip_prefix("Arguments:").unwrap_or(line)
            )));
        } else if line.starts_with("Options:") {
            println!("{}", important(&format!(
                "options: {}",
                line.strip_prefix("Options:").unwrap_or(line)
            )));
        } else {
            out.push(format!("{}{}", " ".repeat(indentation as usize), line));
        }
    }

    out.join("\n")
}

pub fn getcommand(command: &Vec<&str>) -> String {
    format!("  {}", style(command.join(" ")).cyan())
}

pub fn getcommandoutput(output: Output, spaces: Option<u8>) -> String {
    let mut out = vec![];
    let indentation = if let Some(indentation) = spaces {
        indentation
    } else {
        0
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        for line in stdout.lines() {
            out.push(format!("{}{}", " ".repeat(indentation as usize), line));
        }
    }

    out.join("\n")
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
    println!("{}", debug("parsing commit command output", verbose));
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

    println!("{}", debug("searching output lines", verbose));
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
    println!("{}", debug("done", verbose));

    if fileschangedline.is_none() {
        println!("{}", debug(
            &format!("raw stdout when required lines not found: {}", stdout),
            verbose,
        ));
        println!("{}", debug(
            &format!("fileschangedline: {:?}", fileschangedline),
            verbose,
        ));
        println!("{}", debug(&format!("modeline: {:?}", modeline), verbose));
        println!("{}", debug("falling back to printcommandoutput()", verbose));
        getcommandoutput(output, Some(2));
        return;
    }

    println!("{}", debug("splitting files changed line", verbose));
    let Some(fileschangedline) = fileschangedline else {
        println!("{}", debug(
            "fileschangedline was None unexpectedly (should have been caught by prior check). falling back.",
            verbose,
        ));
        println!("{}", debug(&format!("modeline (Option): {:?}", modeline), verbose));
        println!("{}", debug(&format!("raw stdout: {}", stdout), verbose));
        getcommandoutput(output, Some(2));
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

    println!("{}", debug("checking errors", verbose));
    if let Err(e) = insertionsres {
        println!("{}", debug(
            &format!("raw stdout on insertion parse error: {}", stdout),
            verbose,
        ));
        println!("{}", debug(&format!("error: {}", e), verbose));
        println!("{}", debug("falling back to printcommandoutput()", verbose));
        getcommandoutput(output, Some(2));
        return;
    }

    if let Err(e) = deletionsres {
        println!("{}", debug(
            &format!("raw stdout on deletion parse error: {}", stdout),
            verbose,
        ));
        println!("{}", debug(&format!("error: {}", e), verbose));
        println!("{}", debug("falling back to printcommandoutput()", verbose));
        getcommandoutput(output, Some(2));
        return;
    }

    println!("{}", debug("getting insertions and deletions", verbose));
    let insertions = insertionsres.unwrap_or(0);
    let deletions = deletionsres.unwrap_or(0);

    println!("{}", debug("printing custom commit output", verbose));
    println!("{}", info(&format!(
        "    {} {} file(s) changed{}",
        branchhashinfo,
        fileschangedcount,
        if let Some(modeline) = modeline {
            format!(", {}", modeline.trim())
        } else {
            String::new()
        }
    )));
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
            println!("{}", info(&format!(
                "    {} {} {}",
                modeparts[0],
                modeparts[1],
                modeparts[2..].join(" ")
            )));
        }
    }
}

pub fn error(text: &str) -> String {
    style(text).red().to_string()
}

pub fn important(text: &str) -> String {
    style(text).cyan().to_string()
}

pub fn info(text: &str) -> String {
    style(text).magenta().to_string()
}

pub fn debug(text: &str, verbose: &u8) -> String {
    if verbose.to_owned() >= 1 {
        format!("{}\n", style(text).blue())
    } else {
        String::new()
    }
}

pub fn success(text: &str) -> String {
    style(text).green().to_string()
}
