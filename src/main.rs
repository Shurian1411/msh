use rustyline::{Editor, config::Configurer, history::DefaultHistory};

use msh::{
    command::Pipeline, exec, history::History, job::JobTable, shell::ShellHelper, state::ShellState,
};

fn main() -> anyhow::Result<()> {
    let mut rl = Editor::<ShellHelper, DefaultHistory>::new()?;
    rl.set_helper(Some(ShellHelper::default()));
    rl.set_completion_type(rustyline::CompletionType::List);
    let mut job_table = JobTable::default();
    let mut history = History::default();

    loop {
        for line in job_table.reap_done() {
            println!("{}", line);
        }

        let input = rl.readline("$ ")?;
        if !input.trim().is_empty() {
            history.push(input.clone());
            rl.add_history_entry(&input)?;
        }

        let registry = rl.helper_mut().unwrap().completion_registry_mut();
        let mut state = ShellState {
            completion_registry: registry,
            jobs: &mut job_table,
            history: &mut history,
        };
        let Some(pipeline) = Pipeline::parse(&input) else {
            continue;
        };

        if exec::run(&pipeline, &mut state)?.is_break() {
            break;
        }
    }

    Ok(())
}
