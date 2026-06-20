use crate::actions::Cause;

#[derive(Default)]
pub struct RunSummary {
    packages_installed: usize,
    files_created: usize,
    files_updated: usize,
    links_created: usize,
    dirs_created: usize,
    ownership_changes: usize,
    mode_changes: usize,
    post_commands: usize,
    errors: usize,
    skipped: usize,
}

impl RunSummary {
    pub fn record(&mut self, cause: Cause) {
        match cause {
            Cause::Pkg => self.packages_installed += 1,
            Cause::Create => self.files_created += 1,
            Cause::Update => self.files_updated += 1,
            Cause::Link => self.links_created += 1,
            Cause::Dir => self.dirs_created += 1,
            Cause::Owner => self.ownership_changes += 1,
            Cause::Mode => self.mode_changes += 1,
            Cause::Post => self.post_commands += 1,
            Cause::None => {}
        }
    }

    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    pub fn record_skipped(&mut self, count: usize) {
        self.skipped = count;
    }

    pub fn log(&self) {
        let counts = [
            (self.files_created, "created"),
            (self.files_updated, "updated"),
            (self.links_created, "links created"),
            (self.dirs_created, "dirs created"),
            (self.packages_installed, "packages installed"),
            (self.ownership_changes, "ownership changes"),
            (self.mode_changes, "mode changes"),
            (self.post_commands, "post commands"),
            (self.skipped, "already in sync"),
            (self.errors, "errors"),
        ];

        let parts: Vec<String> = counts
            .iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, label)| format!("{n} {label}"))
            .collect();

        if parts.is_empty() {
            log::info!("Summary: no changes");
        } else {
            log::info!("Summary: {}", parts.join(", "));
        }
    }

    #[cfg(test)]
    pub fn as_parts(&self) -> Vec<String> {
        let counts = [
            (self.files_created, "created"),
            (self.files_updated, "updated"),
            (self.links_created, "links created"),
            (self.dirs_created, "dirs created"),
            (self.packages_installed, "packages installed"),
            (self.ownership_changes, "ownership changes"),
            (self.mode_changes, "mode changes"),
            (self.post_commands, "post commands"),
            (self.skipped, "already in sync"),
            (self.errors, "errors"),
        ];

        counts
            .iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, label)| format!("{n} {label}"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_summary_is_empty() {
        let summary = RunSummary::default();
        assert!(summary.as_parts().is_empty());
    }

    #[test]
    fn record_each_cause() {
        let mut s = RunSummary::default();
        s.record(Cause::Create);
        s.record(Cause::Create);
        s.record(Cause::Update);
        s.record(Cause::Link);
        s.record(Cause::Dir);
        s.record(Cause::Pkg);
        s.record(Cause::Owner);
        s.record(Cause::Mode);
        s.record(Cause::Post);

        let parts = s.as_parts();
        assert!(parts.contains(&"2 created".to_string()));
        assert!(parts.contains(&"1 updated".to_string()));
        assert!(parts.contains(&"1 links created".to_string()));
        assert!(parts.contains(&"1 dirs created".to_string()));
        assert!(parts.contains(&"1 packages installed".to_string()));
        assert!(parts.contains(&"1 ownership changes".to_string()));
        assert!(parts.contains(&"1 mode changes".to_string()));
        assert!(parts.contains(&"1 post commands".to_string()));
    }

    #[test]
    fn record_none_is_noop() {
        let mut s = RunSummary::default();
        s.record(Cause::None);
        assert!(s.as_parts().is_empty());
    }

    #[test]
    fn record_error() {
        let mut s = RunSummary::default();
        s.record_error();
        s.record_error();
        assert_eq!(s.as_parts(), vec!["2 errors"]);
    }

    #[test]
    fn record_skipped() {
        let mut s = RunSummary::default();
        s.record_skipped(5);
        assert_eq!(s.as_parts(), vec!["5 already in sync"]);
    }

    #[test]
    fn only_nonzero_counts_appear() {
        let mut s = RunSummary::default();
        s.record(Cause::Create);
        s.record_error();
        let parts = s.as_parts();
        assert_eq!(parts.len(), 2);
        assert!(parts.contains(&"1 created".to_string()));
        assert!(parts.contains(&"1 errors".to_string()));
    }
}
