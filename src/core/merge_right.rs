use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use crate::core::valid::{Valid, ValidationError, Validator};
use serde_yaml::Value;

pub trait MergeRight {
    fn merge_right(self, other: Self) -> Valid<Self, String>;
}

impl<A: MergeRight> MergeRight for Option<A> {
    fn merge_right(self, other: Self) -> Valid<Self, String> {
        match (self, other) {
            (Some(this), Some(that)) => Some(this.merge_right(that)),
            (None, Some(that)) => Some(that),
            (Some(this), None) => Some(this),
            (None, None) => Valid::succeed(None),
        }
    }
}

impl<A: MergeRight + Default> MergeRight for Arc<A> {
    fn merge_right(self, other: Self) -> Valid<Self, String> {
        let l = Arc::into_inner(self);
        let r = Arc::into_inner(other);
        Arc::new(l.merge_right(r).unwrap_or_default())
    }
}

impl<A> MergeRight for Vec<A> {
    fn merge_right(mut self, other: Self) -> Valid<Self, String> {
        self.extend(other);
        Valid::succeed(self)
    }
}

impl<K, V> MergeRight for BTreeMap<K, V>
where
    K: Ord,
    V: Clone + MergeRight,
{
    fn merge_right(mut self, other: Self) -> Valid<Self, String> {
        let mut errors = ValidationError::empty();

        for (other_key, other_value) in other {
            if let Some(self_value) = self.remove(&other_key) {
                match self_value.merge_right(other_value).to_result() {
                    Ok(merged_value) => {
                        self.insert(other_key, merged_value);
                    }
                    Err(err) => {
                        errors = errors.combine(err);
                    }
                }
            } else {
                self.insert(other_key, other_value);
            }
        }

        if errors.is_empty() {
            Valid::succeed(self)
        } else {
            Valid::from_validation_err(errors)
        }
    }
}

impl<V> MergeRight for BTreeSet<V>
where
    V: Ord,
{
    fn merge_right(mut self, other: Self) -> Valid<Self, String> {
        self.extend(other);
        Valid::succeed(self)
    }
}

impl<V> MergeRight for HashSet<V>
where
    V: Eq + std::hash::Hash,
{
    fn merge_right(mut self, other: Self) -> Valid<Self, String> {
        self.extend(other);
        Valid::succeed(self)
    }
}

impl<K, V> MergeRight for HashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn merge_right(mut self, other: Self) -> Valid<Self, String> {
        self.extend(other);
        Valid::succeed(self)
    }
}

impl MergeRight for Value {
    fn merge_right(self, other: Self) -> Valid<Self, String> {
        match (self, other) {
            (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_), other) => {
                Valid::succeed(other)
            }
            (Value::Sequence(mut lhs), other) => match other {
                Value::Sequence(rhs) => {
                    lhs.extend(rhs);
                    Valid::succeed(Value::Sequence(lhs))
                }
                other => {
                    lhs.push(other);
                    Valid::succeed(Value::Sequence(lhs))
                }
            },
            (Value::Mapping(mut lhs), other) => match other {
                Value::Mapping(rhs) => {
                    let mut errors = ValidationError::empty();
                    for (key, other_value) in rhs {
                        if let Some(lhs_value) = lhs.remove(&key) {
                            match lhs_value.merge_right(other_value).to_result() {
                                Ok(merged_value) => {
                                    lhs.insert(key, merged_value);
                                }
                                Err(err) => {
                                    errors = errors.combine(err);
                                }
                            }
                        } else {
                            lhs.insert(key, other_value);
                        }
                    }

                    if errors.is_empty() {
                        Valid::succeed(Value::Mapping(lhs))
                    } else {
                        Valid::from_validation_err(errors)
                    }
                }
                Value::Sequence(mut rhs) => {
                    rhs.push(Value::Mapping(lhs));
                    Valid::succeed(Value::Sequence(rhs))
                }
                other => Valid::succeed(other),
            },
            (Value::Tagged(mut lhs), other) => match other {
                Value::Tagged(rhs) => {
                    if lhs.tag == rhs.tag {
                        lhs.value = lhs.value.merge_right(rhs.value)?;
                        Valid::succeed(Value::Tagged(lhs))
                    } else {
                        Valid::succeed(Value::Tagged(rhs))
                    }
                }
                other => Valid::succeed(other),
            },
        }
    }
}
