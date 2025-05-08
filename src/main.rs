use std::env;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str;

const VERSION: &str = "0.0.0";

fn main() {
    let reporoot = match getrootdir() {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            panic!("please run meow in a git repository");
        }
    };

    println!("meower rust beta");
    println!("version {}", VERSION);
    println!("\nrepository root: {}", reporoot.to_string_lossy());

    let args: Vec<String> = env::args().collect();
    if args.is_empty() {
        println!("no commit message");
        panic!("please run meow <commit message>");
    }

    println!("\nstaging changes...");
    stageall(&reporoot);

    println!("\ncommitting...");
    commit(&reporoot, "testing");

    println!("\npushing...");
    push(&reporoot, Some("main"));

    println!("\n😼");
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
        Ok(o) => {
            println!("staged all files");
        }
        Err(e) => panic!("could not stage all files: {}", e),
    }
    Ok(())
}

fn commit(repopath: &Path, message: &str) -> Result<(), String> {
    let args = &["commit", "-m", message];

    match rungitcommand(repopath, args) {
        Ok(o) => {
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
        Ok(o) => {
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
