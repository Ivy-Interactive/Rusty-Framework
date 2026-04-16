use std::any::Any;

/// Trait for type-erased equality comparison on dependency values.
///
/// Mirrors Ivy-Framework's pattern of comparing dependency arrays
/// using `Object.Equals()` — here we use `Any` downcasting + `PartialEq`.
pub trait DynEq: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn eq_any(&self, other: &dyn DynEq) -> bool;
    fn clone_box(&self) -> Box<dyn DynEq>;
}

impl<T: PartialEq + Clone + Any + Send + Sync + 'static> DynEq for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_any(&self, other: &dyn DynEq) -> bool {
        if let Some(other_val) = other.as_any().downcast_ref::<T>() {
            self == other_val
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn DynEq> {
        Box::new(self.clone())
    }
}

/// Compare two dependency slices. Returns `true` if dependencies have changed.
///
/// Rules:
/// - Different lengths → changed
/// - Any element not equal → changed
/// - Empty deps (both sides) → not changed (stable)
pub fn deps_changed(old: &[Box<dyn DynEq>], new: &[&dyn DynEq]) -> bool {
    if old.len() != new.len() {
        return true;
    }
    for (o, n) in old.iter().zip(new.iter()) {
        if !o.eq_any(*n) {
            return true;
        }
    }
    false
}

/// Clone a slice of `&dyn DynEq` into owned boxes for storage.
pub fn clone_deps(deps: &[&dyn DynEq]) -> Vec<Box<dyn DynEq>> {
    deps.iter().map(|d| d.clone_box()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_deps_not_changed() {
        let old: Vec<Box<dyn DynEq>> = vec![Box::new(1i32), Box::new("hello".to_string())];
        let a = 1i32;
        let b = "hello".to_string();
        let new: Vec<&dyn DynEq> = vec![&a, &b];
        assert!(!deps_changed(&old, &new));
    }

    #[test]
    fn test_different_deps_changed() {
        let old: Vec<Box<dyn DynEq>> = vec![Box::new(1i32), Box::new("hello".to_string())];
        let a = 2i32;
        let b = "hello".to_string();
        let new: Vec<&dyn DynEq> = vec![&a, &b];
        assert!(deps_changed(&old, &new));
    }

    #[test]
    fn test_different_length_changed() {
        let old: Vec<Box<dyn DynEq>> = vec![Box::new(1i32)];
        let a = 1i32;
        let b = 2i32;
        let new: Vec<&dyn DynEq> = vec![&a, &b];
        assert!(deps_changed(&old, &new));
    }

    #[test]
    fn test_empty_deps_not_changed() {
        let old: Vec<Box<dyn DynEq>> = vec![];
        let new: Vec<&dyn DynEq> = vec![];
        assert!(!deps_changed(&old, &new));
    }

    #[test]
    fn test_mixed_types_changed() {
        let old: Vec<Box<dyn DynEq>> = vec![Box::new(1i32)];
        let a = 1u32;
        let new: Vec<&dyn DynEq> = vec![&a];
        // Different types → not equal → changed
        assert!(deps_changed(&old, &new));
    }
}
