use crate::{args::Args, loggers::*};
use clap::{CommandFactory, Parser};
use console::{Emoji, style};
use homedir::my_home;
use std::{
    env::args,
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
    // each pipeline stage runs by default; --push/--commit/--stage narrow this down later
    let mut runstagepipeline = true;
    let mut runcommitpipeline = true;
    let mut runpushpipeline = true;
    let interrupted = Arc::new(AtomicBool::new(false));
    let i = interrupted.clone();

    // trap ctrl-c so downstream checks can see it, rather than aborting mid-command
    ctrlc::set_handler(move || {
        error("\nexiting...");
        i.store(true, Ordering::SeqCst);
    })?;

    // bare invocation: show help and exit rather than error out on a missing commit message
    if args().len() <= 1 {
        printhelp();
        exit(0);
    }

    if args().collect::<Vec<String>>()[1] == "meow" {
        println!("meow meow :3");
    }

    let args = match Args::try_parse() {
        Ok(p) => p,
        Err(err) => {
            // custom error rendering: replace clap's default formatting with the tool's colour scheme
            important("\nmeow");
            important(&format!("version {}\n", env!("CARGO_PKG_VERSION")));

            let commandname = String::from(Args::command().get_name());
            let mut usage = Args::command().render_usage().to_string();

            // strip "Usage: <binname>" so the two halves can be styled separately below
            usage = String::from(usage.strip_prefix("Usage: ").unwrap());
            usage = String::from(usage.strip_prefix(&format!("{}", commandname)).unwrap());

            // clap appends a help hint after a blank line; keep only the message before it
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

    // pull flags into locals so `args` isn't borrowed for the rest of main
    let verbose = args.verbose;
    let run = args.run;
    debug("initializing flags", &verbose);
    let remoteadd = args.addremote;
    let remoteremove = args.removeremote;
    let dryrun = args.dryrun;
    let force = args.force;
    let exitonerror = args.exitonerror;

    important("\nmeow");
    important(&format!("version {}\n", env!("CARGO_PKG_VERSION")));

    // `--run` is reserved for future arbitrary git passthrough; bail out for now
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
    // absolute repo path used as cwd for all subsequent git invocations
    let reporoot = match getrootdir() {
        Ok(r) => r,
        Err(e) => {
            let errorstr = e.to_string();

            if errorstr.contains("not a git repository") {
                error("not a git repository. are you in the correct path?");
            } else {
                error(&errorstr);
            }

            // surface the full error only in verbose mode; otherwise just exit cleanly
            if verbose > 0 {
                return Err(Box::new(e));
            } else {
                exit(1);
            }
        }
    };

    // display-only path with `~` collapsed for home directory
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
    // version banner is already printed above; nothing more to do
    if args.version {
        return Ok(());
    }

    // each *only flag isolates a single pipeline stage by disabling the others
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

    // clap makes the commit message optional under certain flag combos; fall back to empty
    let message = match args.commitmessage {
        Some(message) => message,
        None => String::from(""),
    };

    if dryrun {
        info("dry run\n");
    }

    // remote management short-circuits the pipeline: add/remove is the whole task
    // todo: allow a configurable remote name instead of hardcoding "origin"
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
        // skip stage/commit/push when the user only wants to manage remotes
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

    // stage stage: `-a` selects specific files, otherwise stage everything with `git add .`
    if runstagepipeline {
        info("staging changes...");
        debug("checking if files were specified to be staged", &verbose);
        match args.add {
            Some(toadd) => match stage(&reporoot, &toadd, &dryrun, &verbose) {
                Err(e) => {
                    error(&e);
                    // by default we continue to the next stage; --exit makes any failure fatal
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

    // commit stage
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

    // push stage: `-u <branch>` sets upstream, `-f`/`-ff` selects the force level
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

    // final cat face; falls back to ascii when the terminal cannot render the emoji
    info(&format!("{}", Emoji("\n😼", "\n>:3")));
    Ok(())
}

// resolves the absolute path of the enclosing git repository, or errors if there isn't one
fn getrootdir() -> Result<PathBuf, std::io::Error> {
    // `git rev-parse --show-toplevel` prints the repo root to stdout
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
        // most likely cause: not inside a git repo. surface the raw stderr for other cases
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

// collapses the home-directory prefix into `~` for a shorter display path
fn getcleanroot(reporoot: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let homediropt = my_home()?;

    let cleanroot = if let Some(homedir) = homediropt {
        if reporoot.starts_with(&homedir) {
            let relpath = reporoot.strip_prefix(&homedir)?;
            format!("~/{}", relpath.display())
        } else {
            // repo lives outside home; show the absolute path as-is
            reporoot.to_string_lossy().into_owned()
        }
    } else {
        // couldn't detect a home directory; fall back to the absolute path
        reporoot.to_string_lossy().into_owned()
    };

    Ok(cleanroot)
}

// prefixes any git subcommand invocation with the literal "git"
fn createcommand<'a>(args: &[&'a str]) -> Vec<&'a str> {
    let mut command = vec!["git"];
    command.extend(args);

    command
}

// runs a git subcommand in `repopath` and returns the raw output or a formatted error string
fn runcommand(repopath: &Path, args: &[&str]) -> Result<Output, String> {
    let commandparts = createcommand(args);
    // echo the command so the user can see exactly what's being run
    printcommand(&commandparts);

    if commandparts.is_empty() {
        return Err("cannot execute an empty command.".to_string());
    }

    let command = commandparts[0];
    let commandargs = &commandparts[1..];

    let mut cmd = Command::new(command);
    cmd.args(commandargs);
    // run in the repo root regardless of the user's cwd
    cmd.current_dir(repopath);

    match cmd.output() {
        Ok(o) => {
            if o.status.success() {
                Ok(o)
            } else {
                // non-zero exit: surface stderr in the error string for the caller to inspect
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
        // spawn failed (e.g. git not installed)
        Err(e) => Err(format!(
            "failed to execute command `{}` in directory `{}`: {}",
            style(commandparts.join(" ")).yellow(),
            repopath.display(),
            style(e.to_string()).red()
        )),
    }
}

// `git add .` — stages every change under the repo root
fn stageall(repopath: &Path, dryrun: &bool, verbose: &u8) -> Result<(), String> {
    debug("no files were specified, staging all", verbose);
    let args = &["add", "."];

    // dry run: only print the command, do not touch the index
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

// `git add <files...>` — stages the caller-specified subset
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
            // distinguish "typo in filename" from other git failures for a clearer message
            if e.contains("did not match any files") {
                debug(&format!("error: {}", e), verbose);
                Err(String::from("    could not stage files: files not found"))
            } else {
                Err(String::from("    could not stage files"))
            }
        }
    }
}

// `git commit -m <message>` — commits whatever is currently staged
fn commit(repopath: &Path, message: &str, dryrun: &bool, verbose: &u8) -> Result<(), String> {
    let args = &["commit", "-m", message];

    if *dryrun {
        debug("dry run was specified, not committing", verbose);
        printcommand(&args.to_vec());
        return Ok(());
    }

    match runcommand(repopath, args) {
        Ok(o) => {
            // pretty-print the commit summary rather than dumping raw git output
            printcommitoutput(o, verbose);
            Ok(())
        }
        Err(e) => {
            debug(&format!("    error: {}", e), verbose);
            // most common cause is an empty index; hint at that in the error message
            Err(format!(
                "    could not commit files. are there any changes to commit?"
            ))
        }
    }
}

// `git push` — optionally sets upstream and escalates the force level based on -f count.
// force==1 -> --force-with-lease (safer, checks upstream), force>=2 -> --force (unconditional).
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
        // todo: allow a configurable remote name instead of hardcoding "origin"
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
            // parse push output into a compact "To ...", "branch -> branch" summary
            printpushoutput(o, verbose);
            Ok(())
        }
        Err(e) => {
            debug(&format!("error: {}", e), verbose);
            Err(String::from("could not push to remote"))
        }
    }
}

// `git remote add <name> <url>` — currently always called with name="origin"
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
            // git's usage output contains "[<options>]" when the url is missing
            if e.contains("[<options>]") {
                Err("could not add remote: url not specified".to_string())
            } else {
                Err("could not add remote".to_string())
            }
        }
    }
}

// `git remote remove <name>` — currently always called with name="origin"
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
