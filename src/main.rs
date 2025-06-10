use crate::{args::Args, loggers::*};
use clap::{CommandFactory, Parser};
use console::{Emoji, style};
use homedir::my_home;
use indicatif::ProgressBar;
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
        println!("{}", error("\nexiting..."));
        i.store(true, Ordering::SeqCst);
    })?;

    let args = match Args::try_parse() {
        Ok(p) => p,
        Err(err) => {
            println!("{}", important("\nmeow"));
            println!("{}", important(&format!("version {}\n", VERSION)));

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
    print!("{}", debug("initializing flags", &verbose));
    let dryrun = args.dryrun;
    let force = args.force;
    let exitonerror = args.exitonerror;
    let steps = vec![
        String::from("stage"),
        String::from("commit"),
        String::from("push"),
    ];

    if args.meow {
        println!("{}", info("meow meow :3"));
        return Ok(());
    }

    println!("{}", important("\nmeow"));
    println!("{}", important(&format!("version {}\n", VERSION)));

    if run {
        println!(
            "{}",
            important("run flag was specified, hijacking pipeline")
        );
        println!("{}", important("run is not implemented yet."));
        return Ok(());
    }

    print!("{}", debug("checking if help flag was specified", &verbose));
    if args.help {
        println!();
        println!("{}", printhelp());
        print!("{}", debug("help printed, exiting", &verbose));
        return Ok(());
    }

    print!("{}", debug("getting repository root", &verbose));
    let reporoot = getrootdir()?;
    let root = getcleanroot(&reporoot)?;
    print!("{}", debug(&format!("root is {}", root), &verbose));

    println!(
        "{} {}\n",
        style("repository root:").cyan(),
        style(root).magenta()
    );

    print!(
        "{}",
        debug("checking if version flag was specified", &verbose)
    );
    if args.version {
        return Ok(());
    }

    let message = match args.commitmessage {
        Some(message) => message,
        None => String::from(""),
    };

    if dryrun {
        println!("{}", info("dry run\n"));
    }

    let mainbar = ProgressBar::new(steps.len() as u64);
    
    // Switch debug prints to use mainbar from this point
    let print_debug = |text: &str, verbose: &u8| {
        if *verbose >= 1 {
            mainbar.println(&debug(text, verbose));
        }
    };
    if steps.contains(&String::from("stage")) {
        mainbar.println(&info("staging changes..."));
        print_debug("checking if files were specified to be staged", &verbose);
        match args.add {
            Some(toadd) => match stage(&reporoot, &toadd, &dryrun, &verbose) {
                Err(e) => {
                    mainbar.println(&error(&e));
                    if exitonerror {
                        exit(1);
                    }
                }
                _ => (),
            },
            None => match stageall(&reporoot, &dryrun, &verbose) {
                Err(e) => {
                    mainbar.println(&error(&e));
                    if exitonerror {
                        exit(1);
                    }
                }
                Ok(_) => {}
            },
        }
        mainbar.println(&success("done"));
        mainbar.inc(1);
    }

    if steps.contains(&String::from("commit")) {
        mainbar.println(&info("\ncommitting..."));
        match commit(&reporoot, &message, &dryrun, &verbose) {
            Err(e) => {
                mainbar.println(&error(&e));
                if exitonerror {
                    exit(1);
                }
            }
            _ => (),
        }
        mainbar.println(&success("done"));
        mainbar.inc(1);
    }

    if steps.contains(&String::from("push")) {
        mainbar.println(&info("\npushing..."));
        if verbose >= 1 {
            mainbar.println(&debug("preparing push command", &verbose));
        }
        if args.livepush {
            if let Some(upstream) = args.upstream {
                match push(&reporoot, Some(&upstream), &dryrun, &force, &verbose) {
                    Err(e) => {
                        mainbar.println(&error(&e));
                        if exitonerror {
                            exit(1);
                        }
                    }
                    _ => (),
                }
            } else {
                match push(&reporoot, None, &dryrun, &force, &verbose) {
                    Err(e) => {
                        mainbar.println(&error(&e));
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
                        mainbar.println(&error(&e));
                        if exitonerror {
                            exit(1);
                        }
                    }
                    _ => (),
                }
            } else {
                match livepush(&reporoot, None, &dryrun, &force, &verbose) {
                    Err(e) => {
                        mainbar.println(&error(&e));
                        if exitonerror {
                            exit(1);
                        }
                    }
                    _ => (),
                }
            }
        }
        mainbar.println(&success("done"));
        mainbar.inc(1);
    }

    if dryrun {
        mainbar.println(&info("\ndry run complete"));
        return Ok(());
    }

    mainbar.println(&info(&format!("{}", Emoji("\n😼", "\n:3"))));
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
    println!("{}", getcommand(&commandparts));

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
    println!("{}", debug("no files were specified, staging all", verbose));
    let args = &["add", "."];

    if *dryrun {
        println!("{}", debug("dry run was specified, not staging", verbose));
        println!("{}", getcommand(&args.to_vec()));
        return Ok(());
    }

    match runcommand(repopath, args) {
        Ok(o) => {
            getcommandoutput(o, Some(2));
            Ok(())
        }
        Err(e) => {
            println!("{}", debug(&format!("error: {}", e), verbose));
            Err(String::from("could not stage all"))
        }
    }
}

fn stage(repopath: &Path, files: &[String], dryrun: &bool, verbose: &u8) -> Result<(), String> {
    println!(
        "{}",
        debug(&format!("files {:#?} were specified", files), verbose)
    );
    let mut args = vec!["add".to_owned()];
    args.extend(files.iter().cloned());

    if *dryrun {
        println!("{}", debug("debug was specified, not staging", verbose));
        println!("{}", getcommand(&args.iter().map(|a| a.as_str()).collect::<Vec<&str>>()));
        return Ok(());
    }

    match runcommand(
        repopath,
        &args.iter().map(|a| a.as_str()).collect::<Vec<&str>>(),
    ) {
        Ok(o) => {
            getcommandoutput(o, Some(2));
            Ok(())
        }
        Err(e) => {
            if e.contains("did not match any files") {
                println!("{}", debug(&format!("error: {}", e), verbose));
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
        println!(
            "{}",
            debug("dry run was specified, not committing", verbose)
        );
        println!("{}", getcommand(&args.to_vec()));
        return Ok(());
    }

    match runcommand(repopath, args) {
        Ok(o) => {
            printcommitoutput(o, verbose);
            Ok(())
        }
        Err(e) => {
            println!("{}", debug(&format!("error: {}", e), verbose));
            Err(parsecommiterror(e, verbose))
        }
    }
}

fn parsecommiterror(e: String, verbose: &u8) -> String {
    println!("{}", debug("parsing commit error", verbose));
    if e.contains("fatal: unable to auto-detect email address") {
        println!(
            "{}",
            debug("email address couldnt be auto-detected", verbose)
        );
        String::from(
            "could not detect email address. use git config --global user.email \"you@email.com\"",
        )
    } else if e.contains("No changes to commit")
        || e.contains("nothing to commit, working tree clean")
    {
        println!("{}", debug("no changes to commit", verbose));
        String::from("    nothing to commit :3 meow")
    } else if e.contains("empty commit message") {
        println!("{}", debug("commit message is empty", verbose));
        String::from("    provide a valid commit message")
    } else {
        println!("{}", debug("could not detect error type", verbose));
        String::from("    could not commit files. are there any changes to commit?")
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
        println!(
            "{}",
            debug(&format!("upstream {} was specified", upstreamval), verbose)
        );
        args.extend(["--set-upstream", "origin", upstreamval]);
    }
    if force.to_owned() == 1 {
        println!(
            "{}",
            debug("force was specified, using force-with-lease", verbose)
        );
        args.extend(["--force-with-lease"])
    }
    if force.to_owned() >= 2 {
        println!(
            "{}",
            debug("force was specified twice, using force", verbose)
        );
        args.extend(["--force"])
    }

    if *dryrun {
        println!("{}", debug("dry run was specified, not pushing", verbose));
        println!("{}", getcommand(&args));
        return Ok(());
    }

    println!("{}", debug("dry run was not specified, pushing", verbose));
    match runcommand(repopath, &args) {
        Ok(o) => {
            getcommandoutput(o, Some(2));
            if let Some(branch) = upstream {
                println!("{}", success(&format!("  pushed to remote {}", branch)));
            } else {
                println!("{}", success("  pushed to remote"));
            }
            Ok(())
        }
        Err(e) => {
            println!("{}", debug(&format!("error: {}", e), verbose));
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
            getcommandoutput(o, Some(2));
            if let Some(branch) = upstream {
                println!("{}", success(&format!("  pushed to remote {}", branch)));
            } else {
                println!("{}", success("  pushed to remote"));
            }
            Ok(())
        }
        Err(e) => {
            println!("{}", debug(&format!("error: {}", e), verbose));
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
        print!(
            "{}",
            debug(&format!("upstream {} was specified", upstreamval), verbose)
        );
        args.extend(["--set-upstream", "origin", upstreamval]);
    }
    if force.to_owned() == 1 {
        print!(
            "{}",
            debug("force was specified, using force-with-lease", verbose)
        );
        args.extend(["--force-with-lease"])
    }
    if force.to_owned() >= 2 {
        print!(
            "{}",
            debug("force was specified twice, using force", verbose)
        );
        args.extend(["--force"])
    }

    if *dryrun {
        print!("{}", debug("dry run was specified, not pushing", verbose));
        println!("{}", getcommand(&args));
        return Ok(());
    }

    command.args(&args);

    println!("{}", debug("dry run was not specified, pushing", verbose));
    println!("{}", debug("piping stdout and stderr", verbose));
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    println!("{}", debug("spawning child process", verbose));
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            return Err(format!(
                "{}",
                style(format!("failed to spawn child process: {}", e)).red()
            ));
        }
    };

    println!(
        "{}",
        debug("taking ownership of stdout and stderr", verbose)
    );
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            println!(
                "{}",
                debug(
                    "failed to capture stdout, falling back to pushlite",
                    verbose,
                )
            );
            pushlite(repopath, args, upstream, verbose)?;
            return Ok(());
        }
    };
    let stderr = match child.stderr.take() {
        Some(stdout) => stdout,
        None => {
            println!(
                "{}",
                debug(
                    "failed to capture stderr, falling back to pushlite",
                    verbose,
                )
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
            println!("{}", info(&format!("  {}", line)));
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
