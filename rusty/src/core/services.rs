use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// A server-level registry of shared services, keyed by concrete type.
///
/// Ported from Ivy-Framework's service provider (`ViewContext.UseService<T>`). Where
/// `create_context`/`use_context` scope a value to a view subtree, services are registered
/// once when the server is built and resolved from any view via `use_service`.
///
/// Values are stored as `Arc<dyn Any + Send + Sync>` keyed by `TypeId`, the same idiom as
/// the per-view context map in [`crate::hooks::hook_store::HookStore::contexts`].
#[derive(Default)]
pub struct ServiceRegistry {
    services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            services: HashMap::new(),
        }
    }

    /// Register a service instance. Registering the same type twice overwrites the
    /// previous instance, matching `create_context`'s insert behaviour.
    pub fn register<T: Send + Sync + 'static>(&mut self, value: T) {
        self.services.insert(TypeId::of::<T>(), Arc::new(value));
    }

    /// Resolve a service by type, or `None` if it was never registered.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|arc| arc.clone().downcast::<T>().ok())
    }

    /// Number of registered services.
    pub fn len(&self) -> usize {
        self.services.len()
    }

    /// Whether no services are registered.
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Database {
        url: String,
    }

    #[derive(Debug, PartialEq)]
    struct Cache {
        capacity: usize,
    }

    #[test]
    fn test_register_then_get() {
        let mut registry = ServiceRegistry::new();
        registry.register(Database {
            url: "postgres://localhost".to_string(),
        });

        let db = registry.get::<Database>().expect("Database should resolve");
        assert_eq!(db.url, "postgres://localhost");
    }

    #[test]
    fn test_get_unregistered_returns_none() {
        let registry = ServiceRegistry::new();
        assert!(registry.get::<Database>().is_none());
    }

    #[test]
    fn test_register_same_type_twice_overwrites() {
        let mut registry = ServiceRegistry::new();
        registry.register(Database {
            url: "first".to_string(),
        });
        registry.register(Database {
            url: "second".to_string(),
        });

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get::<Database>().unwrap().url, "second");
    }

    #[test]
    fn test_two_distinct_types_coexist() {
        let mut registry = ServiceRegistry::new();
        registry.register(Database {
            url: "postgres://localhost".to_string(),
        });
        registry.register(Cache { capacity: 128 });

        assert_eq!(registry.len(), 2);
        assert_eq!(
            registry.get::<Database>().unwrap().url,
            "postgres://localhost"
        );
        assert_eq!(registry.get::<Cache>().unwrap().capacity, 128);
    }

    #[test]
    fn test_empty_registry() {
        let mut registry = ServiceRegistry::new();
        assert!(registry.is_empty());
        registry.register(Cache { capacity: 1 });
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_get_returns_shared_instance() {
        let mut registry = ServiceRegistry::new();
        registry.register(Cache { capacity: 64 });

        let a = registry.get::<Cache>().unwrap();
        let b = registry.get::<Cache>().unwrap();
        // Both handles point at the same allocation, not a clone.
        assert!(Arc::ptr_eq(&a, &b));
    }
}
