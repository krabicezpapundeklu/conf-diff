use std::cmp::Ordering;

use anyhow::Result;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct Diff<T>
where
    T: KeyOrd + PartialEq,
{
    pub left_item: Option<T>,
    pub right_item: Option<T>,
}

impl<T> Diff<T>
where
    T: KeyOrd + PartialEq,
{
    pub fn item(&self) -> &T {
        if let Some(left) = &self.left_item {
            left
        } else {
            self.right_item.as_ref().unwrap()
        }
    }
}

#[derive(Debug)]
pub struct EnvironmentConfigDiffs {
    pub environment_names: Vec<String>,
    pub config_point_diffs: Result<Vec<Diff<ConfigPoint>>>,
    pub system_property_diffs: Result<Vec<Diff<SystemProperty>>>,
}

pub trait KeyOrd {
    fn compare_key(&self, other: &Self) -> Ordering;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SystemProperty {
    pub name: String,
    pub value: Option<String>,
}

impl KeyOrd for SystemProperty {
    fn compare_key(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}
