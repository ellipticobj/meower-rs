use crate::{args::Args, loggers::*};
use clap::{CommandFactory, Parser};
use console::{Emoji, style};
use homedir::my_home;
use std::{
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
    process::{Command, Output, exit},
    str,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

mod args;
mod loggers;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runstagepipeline = true;
    let mut runcommitpipeline = true;
    let mut runpushpipeline = true;
    let interrupted = Arc::new(AtomicBool::new(false));
    let i = interrupted.clone();

    ctrlc::set_handler(move || {
        error("\nexiting...");
        i.store(true, Ordering::SeqCst);
    })?;

    let args = match Args::try_parse() {
        Ok(p) => p,
        Err(err) => {
            important("\nmeow");
            important(&format!("version {}\n", env!("CARGO_PKG_VERSION")));

            let commandname = String::from(Args::command().get_name());
            let mut usage = Args::command().render_usage().to_string();

            usage = String::from(usage.strip_prefix("Usage: ").unwrap());
            usage = String::from(usage.strip_prefix(&format!("{}", commandname)).unwrap());

            let erroroutput = format!("{}", err);
            let errormsg = if let Some((before, _)) = erroroutput.split_once("\n\n") {
                before
            } else {
                &erroroutput
            };

            match err.kind() {
                _ => println!("{}\n", style(errormsg).red()),
            }

            println!("{}", style("usage: ").cyan());
            print!("{}", style(&commandname).magenta());
            println!("{}", style(usage).magenta().dim());

            exit(1);
        }
    };

    let verbose = args.verbose;
    let run = args.run;
    debug("initializing flags", &verbose);
    let remoteadd = args.addremote;
    let remoteremove = args.removeremote;
    let dryrun = args.dryrun;
    let force = args.force;
    let exitonerror = args.exitonerror;

    if args.meow {
        info("meow meow :3");
        // return Ok(());
    }

    important("\nmeow");
    important(&format!("version {}\n", env!("CARGO_PKG_VERSION")));

    if run {
        debug("run flag was specified, hijacking pipeline", &verbose);
        error("run is not implemented yet.");
        return Ok(());
    }

    debug("checking if help flag was specified", &verbose);
    if args.help {
        printhelp();
        debug("help printed, exiting", &verbose);
        return Ok(());
    }

    debug("getting repository root", &verbose);
    let reporoot = match getrootdir() {
        Ok(r) => r,
        Err(e) => {
            let errorstr = e.to_string();

            if errorstr.contains("not a git repository") {
                error("not a git repository. are you in the correct path?");
            } else {
                error(&errorstr);
            }

            if verbose > 0 {
                return Err(Box::new(e));
            } else {
                exit(1);
            }
        }
    };

    let root = match getcleanroot(&reporoot) {
        Ok(r) => r,
        Err(e) => {
            error("unexpected error while getting clean root");
            return Err(e);
        }
    };
    debug(&format!("root is {}", root), &verbose);

    println!(
        "{} {}\n",
        style("repository root:").cyan(),
        style(root).magenta()
    );

    debug("checking if version flag was specified", &verbose);
    if args.version {
        return Ok(());
    }

    debug("checking if pushonly was specified", &verbose);
    if args.pushonly {
        debug("pushonly flag was specified", &verbose);
        runstagepipeline = false;
        runcommitpipeline = false;
    }

    debug("checking if commitonly was specified", &verbose);
    if args.commitonly {
        debug("commitonly flag was specified", &verbose);
        runstagepipeline = false;
        runpushpipeline = false;
    }

    debug("checking if stageonly was specified", &verbose);
    if args.stageonly {
        debug("stageonly flag was specified", &verbose);
        runcommitpipeline = false;
        runpushpipeline = false;
    }

    let message = match args.commitmessage {
        Some(message) => message,
        None => String::from(""),
    };

    if dryrun {
        info("dry run\n");
    }

    debug("checking if add remote was specified", &verbose);
    if remoteadd.is_some() {
        debug("add remote flag was specified", &verbose);
        info("  EXPERIMENTAL: adding remote 'origin'...");
        match addremote(
            &reporoot,
            "origin",
            remoteadd.unwrap_or_default().as_str(),
            &dryrun,
            &verbose,
        ) {
            Ok(r) => r,
            Err(e) => {
                info("");
                error(&e);
                exit(1);
            }
        };
        runstagepipeline = false;
        runcommitpipeline = false;
        runpushpipeline = false;
    }

    debug("checking if remove remote was specified", &verbose);
    if remoteremove {
        debug("remove remote flag was specified", &verbose);
        info("  EXPERIMENTAL: removing remote 'origin'...");
        match removeremote(&reporoot, "origin", &dryrun, &verbose) {
            Ok(r) => r,
            Err(e) => {
                info("");
                error(&e);
                exit(1);
            }
        };
        runstagepipeline = false;
        runcommitpipeline = false;
        runpushpipeline = false;
    }

    if runstagepipeline {
        info("staging changes...");
        debug("checking if files were specified to be staged", &verbose);
        match args.add {
            Some(toadd) => match stage(&reporoot, &toadd, &dryrun, &verbose) {
                Err(e) => {
                    error(&e);
                    if exitonerror {
                        exit(1);
                    }
                }
                _ => (),
            },
            None => match stageall(&reporoot, &dryrun, &verbose) {
                Err(e) => {
                    error(&e);
                    if exitonerror {
                        exit(1);
                    }
                }
                _ => (),
            },
        }
        success("done");
    }

    if runcommitpipeline {
        info("\ncommitting...");
        match commit(&reporoot, &message, &dryrun, &verbose) {
            Err(e) => {
                error(&e);
                if exitonerror {
                    exit(1);
                }
            }
            _ => (),
        }
        success("done");
    }

    if runpushpipeline {
        info("\npushing...");
        match push(
            &reporoot,
            args.upstream.as_deref(),
            &dryrun,
            &force,
            &verbose,
        ) {
            Err(e) => {
                error(&e);
                if exitonerror {
                    exit(1);
                }
            }
            _ => (),
        }
    }
    success("done");

    if dryrun {
        info("\ndry run complete");
        return Ok(());
    }

    info(&format!("{}", Emoji("\n😼", "\n>:3")));
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

fn stageall(repopath: &Path, dryrun: &bool, verbose: &u8) -> Result<(), String> {
    debug("no files were specified, staging all", verbose);
    let args = &["add", "."];

    if *dryrun {
        debug("debug was specified, not staging", verbose);
        printcommand(&args.to_vec());
        return Ok(());
    }

    match runcommand(repopath, args) {
        Ok(o) => {
            printcommandoutput(o);
            Ok(())
        }
        Err(e) => {
            debug(&format!("error: {}", e), verbose);
            Err(String::from("could not stage all"))
        }
    }
}

fn stage(repopath: &Path, files: &[String], dryrun: &bool, verbose: &u8) -> Result<(), String> {
    debug(&format!("files {:#?} were specified", files), verbose);
    let mut args = vec!["add".to_owned()];
    args.extend(files.iter().cloned());

    if *dryrun {
        debug("debug was specified, not staging", verbose);
        printcommand(&args.iter().map(|a| a.as_str()).collect::<Vec<&str>>());
        return Ok(());
    }

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
                debug(&format!("error: {}", e), verbose);
                Err(String::from("    could not stage files: files not found"))
            } else {
                Err(String::from("    could not stage files"))
            }
        }
    }
}

fn commit(repopath: &Path, message: &str, dryrun: &bool, verbose: &u8) -> Result<(), String> {
    let args = &["commit", "-m", message];

    if *dryrun {
        debug("dry run was specified, not committing", verbose);
        printcommand(&args.to_vec());
        return Ok(());
    }

    match runcommand(repopath, args) {
        Ok(o) => {
            printcommitoutput(o, verbose);
            Ok(())
        }
        Err(e) => {
            debug(&format!("    error: {}", e), verbose);
            Err(format!(
                "    could not commit files. are there any changes to commit?"
            ))
        }
    }
}

fn push(
    repopath: &Path,
    upstream: Option<&str>,
    dryrun: &bool,
    force: &u8,
    verbose: &u8,
) -> Result<(), String> {
    let mut args = vec!["push"];
    if let Some(upstreamval) = upstream {
        debug(&format!("upstream {} was specified", upstreamval), verbose);
        args.extend(["--set-upstream", "origin", upstreamval]);
    }
    if force.to_owned() == 1 {
        debug("force was specified, using force-with-lease", verbose);
        args.extend(["--force-with-lease"])
    }
    if force.to_owned() >= 2 {
        debug("force was specified twice, using force", verbose);
        args.extend(["--force"])
    }

    if *dryrun {
        debug("dry run was specified, not pushing", verbose);
        printcommand(&args);
        return Ok(());
    }

    debug("dry run was not specified, pushing", verbose);
    match runcommand(repopath, &args) {
        Ok(o) => {
            printpushoutput(o, verbose);
            Ok(())
        }
        Err(e) => {
            debug(&format!("error: {}", e), verbose);
            Err(String::from("could not push to remote"))
        }
    }
}

fn addremote(
    repopath: &Path,
    remotename: &str,
    remoteurl: &str,
    dryrun: &bool,
    verbose: &u8,
) -> Result<(), String> {
    let args = vec!["remote", "add", remotename, remoteurl];

    if *dryrun {
        debug("dry run was specified, not adding remote", verbose);
        printcommand(&args);
        return Ok(());
    }

    debug("dry run was not specified, adding remote", verbose);
    match runcommand(repopath, &args) {
        Ok(o) => {
            printcommandoutput(o);
            Ok(())
        }
        Err(e) => {
            debug(&format!("error: {}", e), verbose);
            if e.contains("[<options>]") {
                Err("could not add remote: url not specified".to_string())
            } else {
                Err("could not add remote".to_string())
            }
        }
    }
}

fn removeremote(
    repopath: &Path,
    remotename: &str,
    dryrun: &bool,
    verbose: &u8,
) -> Result<(), String> {
    let args = vec!["remote", "remove", remotename];

    if *dryrun {
        debug("dry run was specified, not adding remote", verbose);
        printcommand(&args);
        return Ok(());
    }

    debug("dry run was not specified, adding remote", verbose);
    match runcommand(repopath, &args) {
        Ok(o) => {
            printcommandoutput(o);
            Ok(())
        }
        Err(e) => {
            debug(&format!("error: {}", e), verbose);
            Err(String::from("could not add remote"))
        }
    }
}
