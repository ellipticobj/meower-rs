use std::env;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str;

use clap::{Parser, arg, command};

const VERSION: &str = "0.0.0a-rs";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    add: Option<Vec<String>>,

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
    println!("\nrepository root: {}", reporoot.to_string_lossy());

    let args = Args::parse();
    let message = match args.commitmessage {
        Some(message) => message,
        None => {
            eprintln!("\ncommit message not specified");
            std::process::exit(1);
        }
    };
    let message: &str = &message;

    println!("\nstaging changes...");
    match args.add {
        Some(toadd) => match stage(&reporoot, &toadd) {
            _ => (),
        },
        None => match stageall(&reporoot) {
            _ => (),
        },
    }

    println!("\ncommitting...");
    commit(&reporoot, &message)?;

    println!("\npushing...");
    push(&reporoot, Some("main"))?;

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

fn rungitcommand(repopath: &Path, args: &[&str]) -> Result<Output, String> {
    let mut command = Command::new("git");
    command.current_dir(repopath);
    command.args(args);

    println!("{:?}", command);

    match command.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(output)
            } else {
                let stderr = str::from_utf8(&output.stderr).unwrap_or("failed to read stderr");
                Err(format!("git command failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("failed to execute git command: {}", e)),
    }
}

fn stageall(repopath: &Path) -> Result<(), String> {
    let args = &["add", "*"];
    match rungitcommand(repopath, args) {
        Ok(_o) => {
            println!("staged all files");
        }
        Err(e) => panic!("could not stage all files: {}", e),
    }
    Ok(())
}

fn stage(repopath: &Path, files: &[String]) -> Result<(), String> {
    let mut args = vec!["add".to_owned()];
    args.extend(files.iter().map(|s| s.to_owned()).collect::<Vec<String>>());

    match rungitcommand(
        repopath,
        &args.iter().map(|a| a.as_str()).collect::<Vec<&str>>(),
    ) {
        Ok(_o) => {
            println!("staged files");
        }
        Err(e) => panic!("could not stage files {}: {}", files.join(""), e),
    }
    Ok(())
}

fn commit(repopath: &Path, message: &str) -> Result<(), String> {
    let args = &["commit", "-m", message];

    match rungitcommand(repopath, args) {
        Ok(_o) => {
            println!("commited all changes");
        }
        Err(e) => panic!("could not commit files: {}", e),
    }
    Ok(())
}

fn push(repopath: &Path, upstream: Option<&str>) -> Result<(), String> {
    let mut args = vec!["push"];
    if let Some(upstream) = upstream {
        args.extend(["--set-upstream", "origin", upstream]);
    }

    match rungitcommand(repopath, &args) {
        Ok(_o) => {
            if let Some(branch) = upstream {
                println!("pushed to remote {}", branch);
            } else {
                println!("pushed to remote");
            }
            Ok(())
        }
        Err(e) => Err(format!("could not push to remote: {:?}", e)),
    }
}
