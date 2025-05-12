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

    #[arg(name = "message", help = "commit message")]
    pub commitmessage: Option<String>,

    #[arg(long = "version", short = 'V', help = "print version")]
    pub version: bool,

    #[arg(short = 'h', long = "help", help = "prints help")]
    pub help: bool,

    #[arg(long = "meow", hide(true))]
    pub meow: bool,
}
