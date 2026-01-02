use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = None,
    disable_version_flag = true,
    disable_help_flag = true
)]

pub struct Args {
    #[arg(
        short,
        long,
        short = 'a',
        long = "add",
        name = "files",
        help = "specify files to stage"
    )]
    pub add: Option<Vec<String>>,

    #[arg(
        short = 'd',
        long = "dry-run",
        help = "runs meow without running commands"
    )]
    pub dryrun: bool,

    #[arg(
        name = "message",
        help = "commit message",
        required_unless_present_any = &["run", "meow", "help", "version"]
    )]
    pub commitmessage: Option<String>,

    #[arg(long = "version", short = 'V', help = "print version")]
    pub version: bool,

    #[arg(short = 'h', long = "help", help = "prints help")]
    pub help: bool,

    #[arg(long = "meow", hide(true))]
    pub meow: bool,

    #[arg(long = "run", short = 'r', help = "run git commands", hide(true))]
    pub run: bool,

    #[arg(long = "set-upstream", short = 'u', help = "sets upstream")]
    pub upstream: Option<String>,

    #[arg(
        long = "force",
        short = 'f',
        help = "adds --force-with-lease",
        action = clap::ArgAction::Count
    )]
    pub force: u8,

    #[arg(
        long = "verbose",
        short = 'v',
        help = "verbose output",
        action = clap::ArgAction::Count
    )]
    pub verbose: u8,

    #[arg(long = "exit", short = 'E', help = "exits meow on error")]
    pub exitonerror: bool,

    #[arg(long = "push", short = 'p', help = "pushes")]
    pub pushonly: bool,

    #[arg(long = "commit", short = 'c', help = "commits")]
    pub commitonly: bool,

    #[arg(long = "remote-add", aliases = ["radd"], help = "same as git remote add")]
    pub addremote: bool,

    #[arg(
        long = "remote-remove",
        aliases = ["rrm"],
        help = "same as git remote remove"
    )]
    pub removeremote: bool,
}
