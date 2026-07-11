use std::collections::HashMap;

use crate::{history::History, job::JobTable};

pub struct ShellState<'a> {
    pub completion_registry: &'a mut HashMap<String, String>,
    pub jobs: &'a mut JobTable,
    pub history: &'a mut History,
}
