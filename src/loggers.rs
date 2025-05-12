use console::{Term, style};
use std::process::Output;

pub fn printcommand(command: &Vec<&str>) {
    println!("  {}", style(command.join(" ")).cyan());
}

pub fn printcommandoutput(output: Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if !trimmed.is_empty() {
        println!("    {}", style(trimmed).green());
    }
}

pub fn fatalerror(error: &str) {
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

pub fn info(text: &str) {
    println!("{}", style(text).magenta());
}

pub fn debug(text: &str) {
    println!("{}", style(text).blue());
}

pub fn success(text: &str) {
    println!("{}", style(text).green());
}
