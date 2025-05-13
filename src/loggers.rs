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
    let countstr = if trimmed.ends_with('+') || trimmed.ends_with('-') {
        &trimmed[..trimmed.len() - 1]
    } else {
        trimmed
    };
    //parse
    countstr.parse::<i32>()
}

pub fn printcommitoutput(output: Output, verbose: &u8) {
    let stdout_raw = output.stdout.clone();
    let stdout = String::from_utf8_lossy(&stdout_raw);
    let lines = stdout.lines();

    let mut fileschangedline: Option<&str> = None;
    let mut modeline: Option<&str> = None;

    for line in lines {
        if line.contains("files changed") {
            fileschangedline = Some(line);
        } else if line.contains("create mode") || line.contains("delete mode") {
            modeline = Some(line);
        }
    }

    if let (Some(fileschangedline), Some(modeline)) = (fileschangedline, modeline) {
        let parts1 = fileschangedline.split(", ").collect::<Vec<&str>>();
        if parts1.len() > 1 {
            let branchinfo = parts1[0];
            let fileschangedpart = parts1[1];
            let fileschangedcount = fileschangedpart
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse::<i32>()
                .unwrap_or(0);

            let insertionspart = if parts1.len() > 2 {
                parts1[2]
            } else {
                "0 insertions(+)"
            };
            let deletionspart = if parts1.len() > 3 {
                parts1[3]
            } else {
                "0 deletions(-)"
            };
            let insertions = parsecount(insertionspart);
            let deletions = parsecount(deletionspart);

            match insertions {
                Ok(insertions) => match deletions {
                    Ok(deletions) => {
                        info(&format!(
                            "{} {} , {}",
                            branchinfo, fileschangedpart, modeline,
                        ));
                        info(&format!(
                            "{} insertions, {} deletions",
                            insertions, deletions
                        ));
                    }
                    Err(e) => {
                        debug(
                            &format!("raw stdout on deletion parse error: {}", stdout),
                            verbose,
                        );
                        debug(&format!("error: {}", e), verbose);
                        debug("falling back to printcommandoutput()", verbose);
                        printcommandoutput(output);
                    }
                },
                Err(e) => {
                    debug(
                        &format!("raw stdout on insertion parse error: {}", stdout),
                        verbose,
                    );
                    debug(&format!("error: {}", e), verbose);
                    debug("falling back to printcommandoutput()", verbose);
                    printcommandoutput(output);
                }
            }
        } else {
            debug(
                &format!("raw stdout on incomplete files changed line: {}", stdout),
                verbose,
            );
            debug(
                &format!("files changed line: {}", fileschangedline),
                verbose,
            );
            debug("falling back to printcommandoutput()", verbose);
            printcommandoutput(output);
        }
    } else {
        debug(
            &format!("raw stdout when required lines not found: {}", stdout),
            verbose,
        );
        println!("{}", stdout);
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

pub fn debug(text: &str, verbose: &u8) {
    if verbose.to_owned() >= 1 {
        println!("[DEBUG] {}", style(text).blue());
    }
}

pub fn success(text: &str) {
    println!("{}", style(text).green());
}
