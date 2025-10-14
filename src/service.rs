use std::cmp::Ordering;

use anyhow::Result;

use crate::{
    model::{ConfigPoint, Diff, EnvironmentConfigDiffs, KeyOrd, SystemProperty},
    repository::Repository,
};

impl<T> Diff<T>
where
    T: KeyOrd + PartialEq,
{
    fn can_try_merge(&self, left_or_right_item: &LeftOrRightItem<T>) -> bool {
        match (&self.left_item, &self.right_item, left_or_right_item) {
            (None, Some(right_item), LeftOrRightItem::LeftItem(item)) => {
                right_item.compare_key(item) == Ordering::Equal
            }
            (Some(left_item), None, LeftOrRightItem::RightItem(item)) => {
                left_item.compare_key(item) == Ordering::Equal
            }
            _ => false,
        }
    }

    fn compute_diffs(
        left_items: impl Iterator<Item = T>,
        right_items: impl Iterator<Item = T>,
    ) -> Vec<Self> {
        let left_or_right_items = LeftOrRightItem::combine(left_items, right_items);
        let mut diffs: Vec<Self> = Vec::new();

        for left_or_right_item in left_or_right_items {
            if let Some(last_diff) = diffs.last_mut()
                && last_diff.can_try_merge(&left_or_right_item)
            {
                if last_diff.merge(left_or_right_item) {
                    diffs.remove(diffs.len() - 1);
                }

                continue;
            }

            diffs.push(Self::new(left_or_right_item));
        }

        diffs
    }

    fn merge(&mut self, left_or_right_item: LeftOrRightItem<T>) -> bool {
        match left_or_right_item {
            LeftOrRightItem::LeftItem(item) => self.left_item = Some(item),
            LeftOrRightItem::RightItem(item) => self.right_item = Some(item),
        }

        self.left_item == self.right_item
    }

    fn new(left_or_right_item: LeftOrRightItem<T>) -> Self {
        match left_or_right_item {
            LeftOrRightItem::LeftItem(item) => Self {
                left_item: Some(item),
                right_item: None,
            },
            LeftOrRightItem::RightItem(item) => Self {
                left_item: None,
                right_item: Some(item),
            },
        }
    }
}

enum LeftOrRightItem<T>
where
    T: KeyOrd,
{
    LeftItem(T),
    RightItem(T),
}

impl<T> LeftOrRightItem<T>
where
    T: KeyOrd,
{
    fn combine(
        left_items: impl Iterator<Item = T>,
        right_items: impl Iterator<Item = T>,
    ) -> Vec<Self> {
        let mut items: Vec<_> = left_items
            .map(Self::LeftItem)
            .chain(right_items.map(Self::RightItem))
            .collect();

        items.sort_by(|left, right| {
            let comparison = left.item().compare_key(right.item());

            if comparison == Ordering::Equal {
                match (left, right) {
                    (Self::LeftItem(_), Self::RightItem(_)) => Ordering::Less,
                    (Self::RightItem(_), Self::LeftItem(_)) => Ordering::Greater,
                    _ => Ordering::Equal,
                }
            } else {
                comparison
            }
        });

        items
    }

    const fn item(&self) -> &T {
        match self {
            Self::LeftItem(item) | Self::RightItem(item) => item,
        }
    }
}

pub trait Service: Clone + Send + Sync {
    fn get_environment_config_diffs(
        &self,
        left_environment_name: &str,
        right_environment_name: &str,
    ) -> Result<EnvironmentConfigDiffs>;

    fn get_environment_names(&self) -> Result<Vec<String>>;
}

#[derive(Clone)]
struct ServiceImpl<R>
where
    R: Repository,
{
    repository: R,
}

impl<R> Service for ServiceImpl<R>
where
    R: Repository,
{
    fn get_environment_config_diffs(
        &self,
        left_environment_name: &str,
        right_environment_name: &str,
    ) -> Result<EnvironmentConfigDiffs> {
        let controller_connection = self.repository.get_controller_connection()?;

        Ok(EnvironmentConfigDiffs {
            environment_names: self
                .repository
                .get_environment_names(&controller_connection)?,
            config_point_diffs: self.get_config_point_diffs(
                &controller_connection,
                left_environment_name,
                right_environment_name,
            ),
            system_property_diffs: self.get_system_property_diffs(
                &controller_connection,
                left_environment_name,
                right_environment_name,
            ),
        })
    }

    fn get_environment_names(&self) -> Result<Vec<String>> {
        let controller_connection = self.repository.get_controller_connection()?;

        self.repository
            .get_environment_names(&controller_connection)
    }
}

impl<R> ServiceImpl<R>
where
    R: Repository,
{
    fn get_config_point_diffs(
        &self,
        controller_connection: &R::Connection,
        left_environment_name: &str,
        right_environment_name: &str,
    ) -> Result<Vec<Diff<ConfigPoint>>> {
        let left_environment_connection = self
            .repository
            .get_environment_connection(controller_connection, left_environment_name)?;

        let left_environment_config_points = self
            .repository
            .get_config_points(&left_environment_connection)?;

        let right_environment_connection = self
            .repository
            .get_environment_connection(controller_connection, right_environment_name)?;

        let right_environment_config_points = self
            .repository
            .get_config_points(&right_environment_connection)?;

        Ok(Diff::compute_diffs(
            left_environment_config_points.into_iter(),
            right_environment_config_points.into_iter(),
        ))
    }

    fn get_system_property_diffs(
        &self,
        controller_connection: &R::Connection,
        left_environment_name: &str,
        right_environment_name: &str,
    ) -> Result<Vec<Diff<SystemProperty>>> {
        let left_environment_system_properties = self
            .repository
            .get_system_properties(controller_connection, left_environment_name)?;

        let right_environment_system_properties = self
            .repository
            .get_system_properties(controller_connection, right_environment_name)?;

        Ok(Diff::compute_diffs(
            left_environment_system_properties.into_iter(),
            right_environment_system_properties.into_iter(),
        ))
    }
}

pub fn new_service(repository: impl Repository) -> impl Service {
    ServiceImpl { repository }
}

#[cfg(test)]
mod tests {
    use crate::model::SystemProperty;

    use super::*;

    #[test]
    fn test_compute_diffs() {
        let left_items = vec![
            SystemProperty {
                name: "A".to_string(),
                value: Some("A".to_string()),
            },
            SystemProperty {
                name: "C".to_string(),
                value: Some("C".to_string()),
            },
            SystemProperty {
                name: "D".to_string(),
                value: None,
            },
        ];

        let right_items = vec![
            SystemProperty {
                name: "D".to_string(),
                value: Some("D".to_string()),
            },
            SystemProperty {
                name: "B".to_string(),
                value: Some("B".to_string()),
            },
            SystemProperty {
                name: "C".to_string(),
                value: Some("C".to_string()),
            },
        ];

        let diffs = Diff::compute_diffs(left_items.iter().cloned(), right_items.iter().cloned());

        assert_eq!(diffs.len(), 3);
        assert_eq!(diffs[0].left_item.as_ref(), left_items.get(0));
        assert_eq!(diffs[0].right_item, None);
        assert_eq!(diffs[1].left_item, None);
        assert_eq!(diffs[1].right_item.as_ref(), right_items.get(1));
        assert_eq!(diffs[2].left_item.as_ref(), left_items.get(2));
        assert_eq!(diffs[2].right_item.as_ref(), right_items.get(0));
    }
}
