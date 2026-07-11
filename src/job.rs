use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum JobStatus {
    Running,
    Done,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Running => "Running",
            Self::Done => "Done",
        };
        f.pad(s)
    }
}

#[derive(Debug)]
pub struct Job {
    children: Vec<std::process::Child>,
    pub status: JobStatus,
    pub command: String,
}

impl Job {
    pub(crate) fn new(children: Vec<std::process::Child>, command: String) -> Self {
        Self {
            children,
            status: JobStatus::Running,
            command,
        }
    }

    pub fn pids(&self) -> Vec<u32> {
        self.children.iter().map(|c| c.id()).collect()
    }

    fn mark_exited(&mut self) {
        let exited: Vec<bool> = self
            .children
            .iter_mut()
            .map(|child| matches!(child.try_wait(), Ok(Some(_))))
            .collect();
        if exited.into_iter().all(|done| done) {
            self.status = JobStatus::Done;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub u32);

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default)]
pub struct JobTable {
    jobs: HashMap<JobId, Job>,
}

impl JobTable {
    pub fn insert(&mut self, children: Vec<std::process::Child>, command: String) -> JobId {
        let next_job_id = self.next_available_id();
        self.jobs.insert(next_job_id, Job::new(children, command));
        next_job_id
    }

    pub fn list(&mut self) -> Vec<String> {
        self.mark_exited();
        let lines = self
            .snapshot()
            .into_iter()
            .map(|(id, marker, job)| format_job_line(id, marker, job))
            .collect();
        self.remove_done();
        lines
    }

    /// Marks exited jobs as done, returns their formatted `Done` lines, and removes
    /// them from the table. Used both before each prompt and inside the `jobs` builtin
    pub fn reap_done(&mut self) -> Vec<String> {
        self.mark_exited();
        let lines = self
            .snapshot()
            .into_iter()
            .filter(|(_, _, job)| job.status == JobStatus::Done)
            .map(|(id, marker, job)| format_job_line(id, marker, job))
            .collect();
        self.remove_done();
        lines
    }

    fn mark_exited(&mut self) {
        for job in self.jobs.values_mut() {
            job.mark_exited();
        }
    }

    /// Job numbers are recycled: the smallest number not currently in the table.
    fn next_available_id(&self) -> JobId {
        let mut id = 1;
        while self.jobs.contains_key(&JobId(id)) {
            id += 1;
        }
        JobId(id)
    }

    fn remove_done(&mut self) {
        self.jobs.retain(|_, job| job.status != JobStatus::Done);
    }

    /// All jobs sorted by id, paired with their current job marker.
    /// The marker is computed over the full table so that a job being
    /// reaped still gets the same marker `jobs` would have shown.
    fn snapshot(&self) -> Vec<(JobId, char, &Job)> {
        let mut jobs: Vec<(JobId, &Job)> = self.jobs.iter().map(|(id, job)| (*id, job)).collect();
        jobs.sort_by_key(|(id, _)| id.0);
        let max_id = jobs.last().map(|(id, _)| *id);
        let second_id = jobs.len().checked_sub(2).map(|i| jobs[i].0);
        jobs.into_iter()
            .map(|(id, job)| {
                let marker = if Some(id) == max_id {
                    '+'
                } else if Some(id) == second_id {
                    '-'
                } else {
                    ' '
                };
                (id, marker, job)
            })
            .collect()
    }
}

/// Formats a single job-table entry the way `jobs` and automatic reaping display it,
/// e.g. `[1]+  Running                        sleep 100 &`
fn format_job_line(id: JobId, marker: char, job: &Job) -> String {
    format!(
        "[{id}]{marker}  {:<24}{}{}",
        job.status,
        job.command,
        if job.status == JobStatus::Running {
            " &"
        } else {
            ""
        }
    )
}
