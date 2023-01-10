mod macros;
mod pacman;
mod rust;
mod todo_per_backend;

use std::collections::HashSet;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::{Group, Package};

pub(crate) use todo_per_backend::ToDoPerBackend;

pub use pacman::Pacman;
pub use rust::Rust;
pub(in crate::backend) type Switches = &'static [&'static str];
pub(in crate::backend) type Text = &'static str;

#[derive(Debug, Hash, PartialEq, Eq)]
pub(crate) enum Backends {
    Pacman,
    Rust,
}

impl Backends {
    pub fn iter() -> BackendIter {
        BackendIter(Some(Self::Pacman))
    }
}

#[derive(Debug)]
pub(crate) struct BackendIter(Option<Backends>);

impl Iterator for BackendIter {
    type Item = Box<dyn Backend>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            Some(Backends::Pacman) => {
                self.0 = Some(Backends::Rust);
                Some(Box::new(Pacman::new()))
            }
            Some(Backends::Rust) => {
                self.0 = None;
                Some(Box::new(Rust::new()))
            }
            None => None,
        }
    }
}

pub(crate) trait Backend {
    fn get_binary(&self) -> Text;
    fn get_section(&self) -> Text;
    fn get_switches_install(&self) -> Switches;
    fn get_switches_remove(&self) -> Switches;
    fn get_managed_packages(&self) -> &HashSet<Package>;
    fn load(&mut self, groups: &HashSet<Group>);

    /// Get all packages that are installed in the system.
    fn get_all_installed_packages(&self) -> HashSet<Package>;

    /// Get all packages that were installed in the system explicitly.
    fn get_explicitly_installed_packages(&self) -> HashSet<Package>;

    /// Install the specified packages.
    fn install_packages(&self, packages: &[Package]) {
        if packages.is_empty() {
            return;
        }

        let mut cmd = Command::new(self.get_binary());
        cmd.args(self.get_switches_install());
        for p in packages {
            cmd.arg(format!("{p}"));
        }
        cmd.exec();
    }

    /// Remove the specified packages.
    fn remove_packages(&self, packages: Vec<Package>) {
        let mut cmd = Command::new(self.get_binary());
        cmd.args(self.get_switches_remove());
        for p in packages {
            cmd.arg(format!("{p}"));
        }
        cmd.exec();
    }

    /// extract packages from its own section as read from group files
    fn extract_packages_from_group_file_content(&self, content: &str) -> HashSet<Package> {
        content
            .lines()
            .skip_while(|line| !line.starts_with(&format!("[{}]", self.get_section())))
            .skip(1)
            .filter(|line| !line.starts_with('['))
            .fuse()
            .filter_map(Package::try_from)
            .collect()
    }

    fn get_missing_packages_sorted(&self) -> Vec<Package> {
        let installed = self.get_all_installed_packages();
        let managed = self.get_managed_packages();
        let mut diff: Vec<_> = managed.difference(&installed).cloned().collect();
        diff.sort_unstable();
        diff
    }

    fn add_packages(&mut self, packages: HashSet<Package>);

    fn get_unmanaged_packages_sorted(&self) -> Vec<Package> {
        let installed = self.get_explicitly_installed_packages();
        let required = self.get_managed_packages();
        let mut diff: Vec<_> = installed.difference(required).cloned().collect();
        diff.sort_unstable();
        diff
    }
}
