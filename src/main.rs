use crate::{args::Args, loggers::*};
use clap::{CommandFactory, Parser};
use console::{Emoji, style};
use homedir::my_home;
use std::{
    io::{BufRead, BufReader, Error, ErrorKind},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio, exit},
    str,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

mod args;
mod loggers;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            important(&format!("version {}\n", VERSION));

            let commandname = String::from(Args::command().get_name());
            let mut usage = Args::command().render_usage().to_string();
            // error("error");

            usage = String::from(usage.strip_prefix("Usage: ").unwrap());
            usage = String::from(usage.strip_prefix(&format!("{}", commandname)).unwrap());

            let erroroutput = format!("{}", err);
            let errormsg = if let Some((before, _)) = erroroutput.split_once("\n\n") {
                before
            } else {
                &erroroutput
            };

            match err.kind() {
                _ => println!("{}", &format!("{}\n", style(errormsg).red())),
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
    let dryrun = args.dryrun;
    let force = args.force;
    let exitonerror = args.exitonerror;
    let mut steps = vec![
        String::from("stage"),
        String::from("commit"),
        String::from("push"),
    ];

    if args.meow {
        info("meow meow :3");
        return Ok(());
    }

    important("\nmeow");
    important(&format!("version {}\n", VERSION));

    if run {
        debug("run flag was specified, hijacking pipeline", &verbose);
        error("run is not implemented yet.");
        return Ok(());
    }

    debug("checking if help flag was specified", &verbose);
    if args.help {
        println!();
        printhelp();
        debug("help printed, exiting", &verbose);
        return Ok(());
    }

    debug("getting repository root", &verbose);
    let reporoot = getrootdir()?;
    let root = getcleanroot(&reporoot)?;
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

    let message = match args.commitmessage {
        Some(message) => message,
        None => String::from(""),
    };

    if dryrun {
        info("dry run\n");
    }

    if steps.contains(&String::from("stage")) {
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

    info("\npushing...");
    if args.livepush {
        if let Some(upstream) = args.upstream {
            match push(&reporoot, Some(&upstream), &dryrun, &force, &verbose) {
                Err(e) => {
                    error(&e);
                    if exitonerror {
                        exit(1);
                    }
                }
                _ => (),
            }
        } else {
            match push(&reporoot, None, &dryrun, &force, &verbose) {
                Err(e) => {
                    error(&e);
                    if exitonerror {
                        exit(1);
                    }
                }
                _ => (),
            }
        }
    } else {
        if let Some(upstream) = args.upstream {
            match livepush(&reporoot, Some(&upstream), &dryrun, &force, &verbose) {
                Err(e) => {
                    error(&e);
                    if exitonerror {
                        exit(1);
                    }
                }
                _ => (),
            }
        } else {
            match livepush(&reporoot, None, &dryrun, &force, &verbose) {
                Err(e) => {
                    error(&e);
                    if exitonerror {
                        exit(1);
                    }
                }
                _ => (),
            }
        }
    }
    success("done");

    if dryrun {
        info("\ndry run complete");
        return Ok(());
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
            printcommandoutput(o, Some(2));
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
            printcommandoutput(o, Some(2));
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
            debug(&format!("error: {}", e), verbose);
            Err(format!(
                "    could not commit files. are there any changes to commit?"
            ))
        }
    }
}

fn parsecommiterror(e: String) -> String {
    if e.contains("fatal: unable to auto-detect email address") {
        String::from(
            "could not detect email address. use git config --global user.email \"you@email.com\"",
        )
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
            printcommandoutput(o, Some(2));
            if let Some(branch) = upstream {
                success(&format!("  pushed to remote {}", branch));
            } else {
                success("  pushed to remote");
            }
            Ok(())
        }
        Err(e) => {
            debug(&format!("error: {}", e), verbose);
            Err(String::from("could not push to remote"))
        }
    }
}

fn pushlite(
    repopath: &Path,
    args: Vec<&str>,
    upstream: Option<&str>,
    verbose: &u8,
) -> Result<(), String> {
    match runcommand(repopath, &args) {
        Ok(o) => {
            printcommandoutput(o, Some(2));
            if let Some(branch) = upstream {
                success(&format!("  pushed to remote {}", branch));
            } else {
                success("  pushed to remote");
            }
            Ok(())
        }
        Err(e) => {
            debug(&format!("error: {}", e), verbose);
            Err(String::from("could not push to remote"))
        }
    }
}

fn livepush(
    repopath: &Path,
    upstream: Option<&str>,
    dryrun: &bool,
    force: &u8,
    verbose: &u8,
) -> Result<(), String> {
    let mut command = Command::new("git");
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

    command.args(&args);

    debug("dry run was not specified, pushing", verbose);
    debug("piping stdout and stderr", verbose);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    debug("spawning child process", verbose);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            return Err(format!(
                "{}",
                style(format!("failed to spawn child process: {}", e)).red()
            ));
        }
    };

    debug("taking ownership of stdout and stderr", verbose);
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            debug(
                "failed to capture stdout, falling back to pushlite",
                verbose,
            );
            pushlite(repopath, args, upstream, verbose)?;
            return Ok(());
        }
    };
    let stderr = match child.stderr.take() {
        Some(stdout) => stdout,
        None => {
            debug(
                "failed to capture stderr, falling back to pushlite",
                verbose,
            );
            pushlite(repopath, args, upstream, verbose)?;
            return Ok(());
        }
    };

    let mut stdoutreader = BufReader::new(stdout);
    let mut stderrreader = BufReader::new(stderr);

    let stdoutthread = thread::spawn(move || {
        let mut line = String::new();
        while stdoutreader.read_line(&mut line).unwrap() > 0 {
            info(&format!("  {}", line));
            // TODO: line.clear();
        }
    });

    let stderrthread = thread::spawn(move || {
        let mut line = String::new();
        while stderrreader.read_line(&mut line).unwrap() > 0 {
            print!("{}", line);
            line.clear();
        }
    });

    stdoutthread.join().expect("stdout thread panicked");
    stderrthread.join().expect("stderr thread panicked");

    let status = match child.wait() {
        Ok(child) => child,
        Err(e) => {
            return Err(format!(
                "{}",
                style(format!("failed to exit child process: {}", e)).red()
            ));
        }
    };

    if status.success() {
        return Ok(());
    } else {
        eprintln!("git push failed with status: {}", status);
    }

    Ok(())
}
