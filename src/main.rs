use clap::{Parser, arg, command};
use console::{Term, style};
use std::{
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
    process::{Command, Output},
    str,
};

const VERSION: &str = "0.0.0a-rs";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    add: Option<Vec<String>>,

    #[arg(short = 'd', long = "dry-run")]
    dryrun: bool,

    #[arg(name = "commitmessage")]
    commitmessage: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("meower");
    println!("version {}", VERSION);

    let reporoot = match getrootdir() {
        Ok(r) => r,
        Err(_e) => {
            eprintln!("\nroot directory not detected");
            eprintln!("please run meow in a git repository");
            std::process::exit(1);
        }
    };
    println!("\nrepository root: {}\n", reporoot.to_string_lossy());

    let args = Args::parse();
    let dryrun = args.dryrun;
    let message = match args.commitmessage {
        Some(message) => message,
        None => {
            printerror("\ncommit message not specified");
            std::process::exit(1);
        }
    };
    let message: &str = &message;

    if dryrun {
        println!("dry run")
    }

    println!("{}", style("staging changes...").magenta());
    match args.add {
        Some(toadd) => match stage(&reporoot, &toadd, &dryrun) {
            _ => (),
        },
        None => match stageall(&reporoot) {
            _ => (),
        },
    }

    println!("{}", style("\ncommitting...").magenta());
    commit(&reporoot, &message, &dryrun)?;

    println!("{}", style("\npushing...").magenta());
    push(&reporoot, Some("main"), &dryrun)?;

    if dryrun {
        println!("\ndry run complete")
    }

    println!("\n😼");
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

fn printcommand(command: &Vec<&str>) {
    println!("  {}", style(command.join(" ")).cyan());
}

fn printcommandoutput(output: Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if !trimmed.is_empty() {
        println!("    {}", style(trimmed).green());
    }
}

fn printerror(error: &str) {
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
                    "command `{:?}` executed in `{}` failed with: {}",
                    style(commandparts.join(" ")).yellow(),
                    repopath.display(),
                    style(stderr).red()
                ))
            }
        }
        Err(e) => Err(format!(
            "failed to execute command `{:?}` in directory `{}`: {}",
            commandparts,
            repopath.display(),
            e
        )),
    }
}

fn stageall(repopath: &Path) -> Result<(), String> {
    let args = &["add", "*"];
    match runcommand(repopath, args) {
        Ok(o) => {
            printcommandoutput(o);
            println!("{}", style("staged all files").magenta());
            Ok(())
        }
        Err(e) => Err(format!("could not stage all files: {}", e)),
    }
}

fn stage(repopath: &Path, files: &[String], dryrun: &bool) -> Result<(), String> {
    let mut args = vec!["add".to_owned()];
    args.extend(files.iter().map(|s| s.to_owned()).collect::<Vec<String>>());

    if !dryrun.to_owned() {
        match runcommand(
            repopath,
            &args.iter().map(|a| a.as_str()).collect::<Vec<&str>>(),
        ) {
            Ok(o) => {
                printcommandoutput(o);
                println!("{}", style("staged files").magenta());
            }
            Err(e) => panic!("could not stage files {}: {}", files.join(""), e),
        }
    } else {
        printcommand(&args.iter().map(|a| a.as_str()).collect::<Vec<&str>>())
    }

    Ok(())
}

fn commit(repopath: &Path, message: &str, dryrun: &bool) -> Result<(), String> {
    let args = &["commit", "-m", message];

    if !dryrun.to_owned() {
        match runcommand(repopath, args) {
            Ok(o) => {
                printcommandoutput(o);
                println!("{}", style("commited all changes").magenta());
            }
            Err(e) => panic!("could not commit files: {}", e),
        }
    } else {
        printcommand(&args.iter().map(|a| a.to_owned()).collect::<Vec<&str>>())
    }

    Ok(())
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
                    println!(
                        "{}",
                        style(format!("pushed to remote {}", branch)).magenta()
                    );
                } else {
                    println!("{}", style("pushed to remote").magenta());
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
