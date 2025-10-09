use std::cmp::Ordering;

use anyhow::Result;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigPoint {
    pub name: String,
    pub value: Option<String>,
    pub organization: Option<String>,
}

impl KeyOrd for ConfigPoint {
    fn compare_key(&self, other: &Self) -> Ordering {
        let ordering = self.name.cmp(&other.name);

        if ordering == Ordering::Equal {
            self.organization.cmp(&other.organization)
        } else {
            ordering
        }
    }
}

#[derive(Debug)]
pub struct Diff<T>
where
    T: KeyOrd + PartialEq,
{
    pub left_item: Option<T>,
    pub right_item: Option<T>,
}

#[derive(Debug)]
pub struct EnvironmentConfigDiffs {
    pub config_point_diffs: Result<Vec<Diff<ConfigPoint>>>,
    pub system_property_diffs: Result<Vec<Diff<SystemProperty>>>,
}

pub trait KeyOrd {
    fn compare_key(&self, other: &Self) -> Ordering;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemProperty {
    pub name: String,
    pub value: Option<String>,
}

impl KeyOrd for SystemProperty {
    fn compare_key(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}
