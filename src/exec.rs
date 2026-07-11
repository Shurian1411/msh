use std::{
    io::{self, Write},
    ops::ControlFlow,
    os::unix::process::CommandExt,
    path::PathBuf,
    process::Stdio,
};

use crate::{Builtin, Fd, Redirect, command::Pipeline, resolve::find_external, state::ShellState};

#[derive(Debug)]
enum CommandKind {
    Builtin(Builtin),
    External(PathBuf),
}

fn command_kind(command: &str) -> Option<CommandKind> {
    if let Ok(builtin) = command.parse::<Builtin>() {
        return Some(CommandKind::Builtin(builtin));
    }

    if let Some(path) = find_external(command) {
        return Some(CommandKind::External(path));
    }

    None
}

fn resolve_stdio(redirect: Option<&Redirect>, default: Stdio) -> anyhow::Result<Stdio> {
    Ok(match redirect {
        Some(r) => Stdio::from(r.open()?),
        None => default,
    })
}

fn resolve_writer(
    redirect: Option<&Redirect>,
    default: impl Write + 'static,
) -> anyhow::Result<Box<dyn Write>> {
    Ok(match redirect {
        Some(r) => Box::new(r.open()?),
        None => Box::new(default),
    })
}

pub fn run(pipeline: &Pipeline, state: &mut ShellState) -> anyhow::Result<ControlFlow<()>> {
    let stdout_redirect = pipeline.stages.last().and_then(|c| c.redirect(Fd::Stdout));
    let stderr_redirect = pipeline.stages.last().and_then(|c| c.redirect(Fd::Stderr));
    let last = pipeline.stages.len() - 1;
    let background = pipeline.background;

    let mut kinds = Vec::with_capacity(pipeline.stages.len());
    for command in &pipeline.stages {
        match command_kind(&command.name) {
            Some(kind) => kinds.push(kind),
            None => {
                eprintln!("{}: command not found", command.name);
                return Ok(ControlFlow::Continue(()));
            }
        }
    }

    let mut children = Vec::new();
    let mut next_stdin = Stdio::inherit();

    for (i, command) in pipeline.stages.iter().enumerate() {
        let is_last = i == last;
        let next_is_builtin = matches!(kinds.get(i + 1), Some(CommandKind::Builtin(_)));
        let args = &command.args;

        match &kinds[i] {
            CommandKind::External(path) => {
                let stdout = if is_last {
                    resolve_stdio(stdout_redirect, Stdio::inherit())?
                } else if next_is_builtin {
                    Stdio::null()
                } else {
                    Stdio::piped()
                };
                let stderr = resolve_stdio(stderr_redirect, Stdio::inherit())?;

                let mut child = std::process::Command::new(path)
                    .arg0(&command.name)
                    .args(args)
                    .stdin(next_stdin)
                    .stdout(stdout)
                    .stderr(stderr)
                    .spawn()?;

                next_stdin = match child.stdout.take() {
                    Some(out) => Stdio::from(out),
                    None => Stdio::inherit(),
                };
                children.push(child);
            }
            CommandKind::Builtin(builtin) => {
                if is_last {
                    let mut stdout = resolve_writer(stdout_redirect, io::stdout())?;
                    let mut stderr = resolve_writer(stderr_redirect, io::stderr())?;
                    if builtin
                        .execute(args, &mut stdout, &mut stderr, state)?
                        .is_break()
                    {
                        return Ok(ControlFlow::Break(()));
                    };
                } else if next_is_builtin {
                    if builtin
                        .execute(args, &mut io::sink(), &mut io::stderr(), state)?
                        .is_break()
                    {
                        return Ok(ControlFlow::Break(()));
                    };
                } else {
                    let (reader, mut writer) = io::pipe()?;
                    if builtin
                        .execute(args, &mut writer, &mut io::stderr(), state)?
                        .is_break()
                    {
                        return Ok(ControlFlow::Break(()));
                    };
                    drop(writer);
                    next_stdin = Stdio::from(reader);
                }
            }
        }
    }

    if background && !children.is_empty() {
        let pids: Vec<u32> = children.iter().map(|c| c.id()).collect();
        let full_command = pipeline
            .stages
            .iter()
            .map(|c| format!("{} {}", c.name, c.args.join(" ")))
            .collect::<Vec<_>>()
            .join(" | ");
        let job = state.jobs.insert(children, full_command);
        println!(
            "[{}] {}",
            job,
            pids.iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        );
    } else {
        for mut child in children {
            child.wait()?;
        }
    }

    Ok(ControlFlow::Continue(()))
}
