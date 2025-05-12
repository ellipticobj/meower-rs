use crate::{args::Args, loggers::*};
use clap::Parser;
use console::{Emoji, style};
use homedir::my_home;
use std::{
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
    process::{Command, Output, exit},
    str,
};

mod args;
mod loggers;

const VERSION: &str = "0.0.0a-rs";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("{}", style("meower").cyan());
    if args.version {
        println!("{}", style(format!("version {}", VERSION)).cyan());
        return Ok(());
    }

    let reporoot = getrootdir()?;
    let root = getcleanroot(&reporoot)?;

    println!(
        "\n{} {}\n",
        style("repository root:").cyan(),
        style(root).magenta()
    );

    let dryrun = args.dryrun;
    let message = match args.commitmessage {
        Some(message) => message,
        None => {
            fatalerror("\ncommit message not specified");
            exit(1);
        }
    };

    if dryrun {
        info("dry run\n");
    }

    info("staging changes...");
    match args.add {
        Some(toadd) => match stage(&reporoot, &toadd, &dryrun) {
            Err(e) => {
                error(&e);
                exit(1);
            }
            _ => (),
        },
        None => match stageall(&reporoot) {
            Err(e) => {
                error(&e);
                exit(1);
            }
            _ => (),
        },
    }
    success("done");

    info("\ncommitting...");
    commit(&reporoot, &message, &dryrun)?;
    success("done");

    info("\npushing...");
    push(&reporoot, Some("main"), &dryrun)?;
    success("done");

    if dryrun {
        info("\ndry run complete")
    }

    info(&format!("{}", Emoji("\n😼", "\n:3")));
    Ok(())
}

fn getrootdir() -> Result<PathBuf, std::io::Error> {
    // git rev-parse --show-toplevel
    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--show-toplevel");

    let output = command.output()?;

    if output.status.success() {
        let stdout = str::from_utf8(&output.stdout).map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("invalid utf-8 in git output: {}", e),
            )
        })?;
        let root = PathBuf::from(stdout.trim());
        Ok(root)
    } else {
        let stderr = str::from_utf8(&output.stderr).map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("invalid utf-8 in git error output: {}", e),
            )
        })?;
        Err(Error::new(
            ErrorKind::Other,
            format!(
                "not a git repository or failed to find toplevel: {}",
                stderr
            ),
        ))
    }
}

fn getcleanroot(reporoot: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let homediropt = my_home()?;

    let cleanroot = if let Some(homedir) = homediropt {
        if reporoot.starts_with(&homedir) {
            let relpath = reporoot.strip_prefix(&homedir)?;
            format!("~/{}", relpath.display())
        } else {
            reporoot.to_string_lossy().into_owned()
        }
    } else {
        reporoot.to_string_lossy().into_owned()
    };

    Ok(cleanroot)
}

fn createcommand<'a>(args: &[&'a str]) -> Vec<&'a str> {
    let mut command = vec!["git"];
    command.extend(args);

    command
}

fn runcommand(repopath: &Path, args: &[&str]) -> Result<Output, String> {
    let commandparts = createcommand(args);
    printcommand(&commandparts);

    if commandparts.is_empty() {
        return Err("cannot execute an empty command.".to_string());
    }

    let command = commandparts[0];
    let commandargs = &commandparts[1..];

    let mut cmd = Command::new(command);
    cmd.args(commandargs);
    cmd.current_dir(repopath);

    match cmd.output() {
        Ok(o) => {
            if o.status.success() {
                Ok(o)
            } else {
                let stderr = str::from_utf8(&o.stderr)
                    .unwrap_or("failed to read stderr (non-utf8)")
                    .trim();
                Err(format!(
                    "command `{}` executed in `{}` failed with: {}",
                    style(commandparts.join(" ")).yellow(),
                    repopath.display(),
                    style(stderr).red()
                ))
            }
        }
        Err(e) => Err(format!(
            "failed to execute command `{}` in directory `{}`: {}",
            style(commandparts.join(" ")).yellow(),
            repopath.display(),
            style(e.to_string()).red()
        )),
    }
}

fn stageall(repopath: &Path) -> Result<(), String> {
    let args = &["add", "*"];
    match runcommand(repopath, args) {
        Ok(o) => {
            printcommandoutput(o);
            Ok(())
        }
        Err(e) => Err(format!("could not stage all: {}", e)),
    }
}

fn stage(repopath: &Path, files: &[String], dryrun: &bool) -> Result<(), String> {
    let mut args = vec!["add".to_owned()];
    args.extend(files.iter().cloned());

    if !*dryrun {
        match runcommand(
            repopath,
            &args.iter().map(|a| a.as_str()).collect::<Vec<&str>>(),
        ) {
            Ok(o) => {
                printcommandoutput(o);
                Ok(())
            }
            Err(e) => {
                if e.contains("did not match any files") {
                    Err(format!(
                        "    could not stage files: \n      `{}`\n    files not found",
                        files.join(", ")
                    ))
                } else {
                    Err(format!(
                        "    could not stage files: \n      `{}`",
                        files.join(", ")
                    ))
                }
            }
        }
    } else {
        printcommand(&args.iter().map(|a| a.as_str()).collect::<Vec<&str>>());
        Ok(())
    }
}

fn commit(repopath: &Path, message: &str, dryrun: &bool) -> Result<(), String> {
    let args = &["commit", "-m", message];

    if *dryrun {
        printcommand(&args.to_vec());
        Ok(())
    } else {
        match runcommand(repopath, args) {
            Ok(o) => {
                printcommandoutput(o);
                Ok(())
            }
            Err(e) => Err(format!("could not commit files: {}", e)),
        }
    }
}

fn push(repopath: &Path, upstream: Option<&str>, dryrun: &bool) -> Result<(), String> {
    let mut args = vec!["push"];
    if let Some(upstream_val) = upstream {
        // Changed variable name for clarity, original was fine too
        args.extend(["--set-upstream", "origin", upstream_val]);
    }

    if !dryrun.to_owned() {
        match runcommand(repopath, &args) {
            Ok(o) => {
                printcommandoutput(o);
                if let Some(branch) = upstream {
                    success(&format!("  pushed to remote {}", branch));
                } else {
                    success("  pushed to remote");
                }
                Ok(())
            }
            Err(e) => Err(format!("could not push to remote: {:?}", style(e).red())),
        }
    } else {
        printcommand(&args);
        Ok(())
    }
}
