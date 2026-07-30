use clap::Parser;

// command-line surface for meow. clap's built-in --help and --version are disabled
// so main.rs can render them through the tool's own styled output helpers instead.
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

    // positional commit message. required unless one of the flags in the list makes
    // the commit step unnecessary (help/version banners, push-only, remote management).
    #[arg(
        name = "message",
        help = "commit message",
        required_unless_present_any = &[
            "run",
            "meow",
            "help",
            "pushonly",
            "version",
            "addremote",
            "removeremote"
        ]
    )]
    pub commitmessage: Option<String>,

    #[arg(long = "version", short = 'V', help = "print version")]
    pub version: bool,

    #[arg(short = 'h', long = "help", help = "prints help")]
    pub help: bool,

    #[arg(long = "meow", hide = true)]
    pub meow: bool,

    // todo: `--run` is a placeholder for arbitrary git passthrough; still unimplemented
    #[arg(long = "run", short = 'r', help = "run git commands", hide = true)]
    pub run: bool,

    #[arg(long = "set-upstream", short = 'u', help = "sets upstream")]
    pub upstream: Option<String>,

    // -f enables --force-with-lease, -ff escalates to --force
    #[arg(
        long = "force",
        short = 'f',
        help = "adds --force-with-lease",
        action = clap::ArgAction::Count
    )]
    pub force: u8,

    // count-based so -v, -vv, -vvv can gate progressively noisier debug output
    #[arg(
        long = "verbose",
        short = 'v',
        help = "verbose output",
        action = clap::ArgAction::Count
    )]
    pub verbose: u8,

    #[arg(long = "exit", short = 'E', help = "exits meow on error")]
    pub exitonerror: bool,

    #[arg(long = "push", short = 'p', help = "pushes only")]
    pub pushonly: bool,

    #[arg(long = "commit", short = 'c', help = "commits only")]
    pub commitonly: bool,

    #[arg(long = "stage", short = 's', help = "stages only")]
    pub stageonly: bool,

    // todo: expose the remote name as an argument instead of hardcoding "origin"
    #[arg(
        long = "add-remote",
        aliases = ["radd"],
        help = "EXPERIMENTAL: same as git remote add (remote name is 'origin' for now)"
    )]
    pub addremote: Option<String>,

    #[arg(
        long = "remove-remote",
        aliases = ["rem"],
        help = "EXPERIMENTAL: same as git remote remove (remote name is 'origin' for now)"
    )]
    pub removeremote: Option<String>,
}
