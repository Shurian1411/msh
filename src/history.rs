#[derive(Debug, Default)]
pub struct History {
    entries: Vec<String>,
}

impl History {
    pub fn push(&mut self, command: String) {
        self.entries.push(command);
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}
