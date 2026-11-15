use std::io::{self, Write};

use clap::CommandFactory;
use clap_complete::{Generator, Shell};

use crate::arguments::Arguments;

pub fn print(shell: Shell, standard_output: &mut impl Write) -> io::Result<()> {
    let mut command = Arguments::command();
    command.set_bin_name(command.get_name().to_owned());
    command.build();
    shell.try_generate(&command, standard_output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(shell: Shell) -> String {
        let mut printed = Vec::new();
        print(shell, &mut printed).unwrap();
        String::from_utf8(printed).unwrap()
    }

    #[test]
    fn every_shell_completes_the_commands_and_their_options() {
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
            let script = script(shell);

            for expected in ["tillingctl", "workspace", "topics", "if-revision"] {
                assert!(script.contains(expected), "{shell}: {expected}");
            }
        }
    }
}
