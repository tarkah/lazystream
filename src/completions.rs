use crate::opt::{Command, Opt};

use structopt::{clap::Shell, StructOpt};

pub fn run(opts: Opt) {
    if let Command::Completions { shell, target } = opts.command {
        let _shell = match shell.as_str() {
            "bash" => Shell::Bash,
            "fish" => Shell::Fish,
            "zsh" => Shell::Zsh,
            _ => Shell::Bash,
        };

        Opt::clap().gen_completions(env!("CARGO_PKG_NAME"), _shell, &target);

        println!("{}lazystream.{} saved!", target.display(), shell);
    }
}
