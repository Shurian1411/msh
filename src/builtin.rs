use std::{io::Write, ops::ControlFlow, path::PathBuf};

use strum::{Display, EnumString, VariantNames};

use crate::{resolve::find_external, state::ShellState};

#[derive(Debug, EnumString, Display, VariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum Builtin {
    Cd,
    Complete,
    Exit,
    Echo,
    History,
    Jobs,
    Pwd,
    Type,
}

impl Builtin {
    pub fn execute(
        &self,
        args: &[String],
        stdout: &mut dyn Write,
        stderr: &mut dyn Write,
        state: &mut ShellState,
    ) -> anyhow::Result<ControlFlow<()>> {
        match self {
            Self::Cd => self.exec_cd(args, stderr)?,
            Self::Complete => self.exec_complete(args, stdout, stderr, state)?,
            Self::Exit => return Ok(ControlFlow::Break(())),
            Self::Echo => writeln!(stdout, "{}", args.join(" "))?,
            Self::History => self.exec_history(args, stdout, stderr, state)?,
            Self::Jobs => self.exec_jobs(stdout, state)?,
            Self::Pwd => {
                let current_dir = std::env::current_dir()?;
                writeln!(stdout, "{}", current_dir.display())?;
            }
            Self::Type => self.exec_type(args, stdout, stderr)?,
        }
        Ok(ControlFlow::Continue(()))
    }

    fn exec_cd(&self, args: &[String], stderr: &mut dyn Write) -> anyhow::Result<()> {
        let target = match args.first().map(String::as_str) {
            None | Some("~") => {
                std::env::home_dir().ok_or(anyhow::anyhow!("Failed to get home directory"))?
            }
            Some(path) => PathBuf::from(path),
        };

        if std::env::set_current_dir(&target).is_err() {
            writeln!(
                stderr,
                "cd: {}: No such file or directory",
                target.display()
            )?
        }
        Ok(())
    }

    fn exec_complete(
        &self,
        args: &[String],
        stdout: &mut dyn Write,
        stderr: &mut dyn Write,
        state: &mut ShellState,
    ) -> anyhow::Result<()> {
        match args {
            [flag, script, cmd] if flag == "-C" => {
                state
                    .completion_registry
                    .insert(cmd.clone(), script.clone());
            }
            [flag, cmd] if flag == "-p" => {
                if let Some(completion) = state.completion_registry.get(cmd.as_str()) {
                    writeln!(stdout, "complete -C '{}' {cmd}", completion)?;
                } else {
                    writeln!(stderr, "complete: {cmd}: no completion specification")?;
                }
            }
            [flag, cmd] if flag == "-r" => {
                state.completion_registry.remove(cmd);
            }
            _ => {}
        }

        Ok(())
    }

    fn exec_history(
        &self,
        args: &[String],
        stdout: &mut dyn Write,
        stderr: &mut dyn Write,
        state: &mut ShellState,
    ) -> anyhow::Result<()> {
        match args {
            [flag, file_path] if flag == "-r" => unimplemented!(),
            [] => {
                let skip = args
                    .first()
                    .and_then(|n| n.parse::<usize>().ok())
                    .map(|n| state.history.entries().len().saturating_sub(n))
                    .unwrap_or(0);

                for (i, line) in state.history.entries().iter().enumerate().skip(skip) {
                    writeln!(stdout, "    {}  {}", i + 1, line)?;
                }
            }
            _ => writeln!(stderr, "history: unknown option")?,
        }
        Ok(())
    }

    fn exec_jobs(&self, stdout: &mut dyn Write, state: &mut ShellState) -> anyhow::Result<()> {
        let lines = state.jobs.list();
        for line in lines {
            writeln!(stdout, "{}", line)?;
        }
        Ok(())
    }

    fn exec_type(
        &self,
        args: &[String],
        stdout: &mut dyn Write,
        stderr: &mut dyn Write,
    ) -> anyhow::Result<()> {
        let arg = args
            .first()
            .ok_or_else(|| anyhow::anyhow!("type: missing argument"))?;

        if arg.parse::<Self>().is_ok() {
            writeln!(stdout, "{} is a shell builtin", arg)?;
        } else if let Some(path) = find_external(arg) {
            writeln!(stdout, "{} is {}", arg, path.display())?;
        } else {
            writeln!(stderr, "{}: not found", arg)?;
        }
        Ok(())
    }
}
