use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter, Validator};
use strum::VariantNames;

use crate::{Builtin, resolve::find_all_executables};

#[derive(Default, Helper, Completer, Validator, Highlighter, Hinter)]
pub struct ShellHelper {
    #[rustyline(Completer)]
    completer: ShellCompleter,
}

impl ShellHelper {
    pub fn completion_registry_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.completer.completion_registry
    }
}

#[derive(Default)]
struct ExecutableCache {
    path_var: String,
    names: HashSet<String>,
}

#[derive(Default)]
pub struct ShellCompleter {
    filename_completer: FilenameCompleter,
    pub completion_registry: HashMap<String, String>,
    executable_cache: RefCell<ExecutableCache>,
}

impl ShellCompleter {
    fn complete_registered(
        &self,
        script: &str,
        cmd: &str,
        cur: &str,
        prev: &str,
        line: &str,
        pos: usize,
    ) -> Option<Vec<Pair>> {
        let output = std::process::Command::new(script)
            .args([cmd, cur, prev])
            .env("COMP_LINE", line)
            .env("COMP_POINT", pos.to_string())
            .output()
            .ok()?;
        // split stdout lines -> Pair { display, replacement }
        let stdout = String::from_utf8(output.stdout).ok()?;
        let lines = stdout.split('\n');
        Some(
            lines
                .filter(|s| !s.is_empty())
                .map(|s| Pair {
                    display: s.to_string(),
                    replacement: format!("{} ", s),
                })
                .collect(),
        )
    }

    fn complete_filename(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (pos, pairs) = self.filename_completer.complete(line, pos, ctx)?;
        Ok((
            pos,
            pairs
                .into_iter()
                .map(|mut p| {
                    if p.replacement.ends_with('/') {
                        p.display = p.replacement.clone();
                    } else {
                        p.replacement = format!("{} ", p.replacement);
                    }
                    p
                })
                .collect(),
        ))
    }

    fn complete_command_name(&self, prefix: &str) -> Vec<Pair> {
        let path_var = std::env::var("PATH").unwrap_or_default();
        {
            let mut cache = self.executable_cache.borrow_mut();
            if cache.path_var != path_var {
                cache.names = find_all_executables(&path_var).unwrap_or_default();
                cache.path_var = path_var;
            }
        }
        let cache = self.executable_cache.borrow();
        let mut candidates: Vec<Pair> = Builtin::VARIANTS
            .iter()
            .filter(|&b| {
                let prefix = prefix.trim_end_matches(' ');
                b.starts_with(prefix)
            })
            .map(|b| Pair {
                display: b.to_string(),
                replacement: format!("{} ", b),
            })
            .collect();

        let external_executables = cache.names.iter().filter(|name| name.starts_with(prefix));

        candidates.extend(external_executables.map(|s| Pair {
            display: s.clone(),
            replacement: format!("{} ", s),
        }));
        candidates.sort_by(|a, b| a.display.cmp(&b.display));
        candidates.dedup_by(|a, b| a.display == b.display);

        candidates
    }

    fn word_context<'a>(&self, prefix: &'a str) -> (&'a str, &'a str, &'a str) {
        let (command_name, rest) = prefix.split_once(' ').unwrap_or_default();
        let mut tokens = rest.split(' ');
        let cur = tokens.next_back().unwrap_or_default();
        let prev = tokens.next_back().unwrap_or(command_name);
        (command_name, cur, prev)
    }
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let prefix = &line[..pos];
        let (command_name, cur, prev) = self.word_context(prefix);

        if let Some(script) = self.completion_registry.get(command_name)
            && let Some(candidates) =
                self.complete_registered(script, command_name, cur, prev, line, pos)
            && !candidates.is_empty()
        {
            return Ok((pos - cur.len(), candidates));
        }

        if prefix.contains(" ") {
            self.complete_filename(line, pos, ctx)
        } else {
            let candidates = self.complete_command_name(prefix);
            Ok((0, candidates))
        }
    }
}
