use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Per-connection metadata available to hooks via `use_service`.
///
/// Ported from Ivy-Framework's `Ivy.AppContext`. Ivy also carries a `MachineId`
/// (used by `QueryScope::Device`) and auth state; Rusty has neither, so the
/// connection id is the only field.
#[derive(Debug, Clone)]
pub struct AppContext {
    pub connection_id: String,
}

impl AppContext {
    pub fn new(connection_id: impl Into<String>) -> Self {
        AppContext {
            connection_id: connection_id.into(),
        }
    }
}

/// A type-keyed container of shared services, resolved by hooks via `use_service`.
///
/// Analogous to the slice of ASP.NET dependency injection that Ivy's
/// `IViewContext.UseService<T>()` reaches. Registration is keyed by `TypeId`, so
/// one instance per concrete type.
///
/// Uses `std::sync::RwLock` deliberately: the lock is only ever held for a map
/// lookup or insert, never across an `.await`.
pub struct ServiceRegistry {
    map: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Register a service instance, replacing any previous instance of the same type.
    pub fn register<T: Send + Sync + 'static>(&self, service: Arc<T>) {
        let mut map = self.map.write().unwrap();
        map.insert(TypeId::of::<T>(), service);
    }

    /// Resolve a service by type, or `None` if it was never registered.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let map = self.map.read().unwrap();
        map.get(&TypeId::of::<T>())
            .cloned()
            .and_then(|any| any.downcast::<T>().ok())
    }

    /// Number of registered services.
    pub fn len(&self) -> usize {
        self.map.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.read().unwrap().is_empty()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ServiceRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceRegistry")
            .field("len", &self.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Greeter {
        greeting: String,
    }

    struct Counter {
        value: u32,
    }

    #[test]
    fn test_register_and_get_round_trip() {
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(Greeter {
            greeting: "hello".to_string(),
        }));

        let resolved = registry.get::<Greeter>().expect("Greeter should resolve");
        assert_eq!(resolved.greeting, "hello");
    }

    #[test]
    fn test_get_missing_type_returns_none() {
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(Greeter {
            greeting: "hi".to_string(),
        }));

        assert!(registry.get::<Counter>().is_none());
    }

    #[test]
    fn test_register_replaces_same_type_and_shares_instance() {
        let registry = ServiceRegistry::new();
        let first = Arc::new(Counter { value: 1 });
        registry.register(first.clone());
        assert_eq!(registry.len(), 1);

        // Resolving twice yields the same underlying allocation.
        let a = registry.get::<Counter>().unwrap();
        let b = registry.get::<Counter>().unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert!(Arc::ptr_eq(&a, &first));

        // Re-registering the same type replaces it rather than adding a second slot.
        registry.register(Arc::new(Counter { value: 2 }));
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get::<Counter>().unwrap().value, 2);
    }

    #[test]
    fn test_two_distinct_types_coexist() {
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(Greeter {
            greeting: "hi".to_string(),
        }));
        registry.register(Arc::new(Counter { value: 128 }));

        assert_eq!(registry.len(), 2);
        assert_eq!(registry.get::<Greeter>().unwrap().greeting, "hi");
        assert_eq!(registry.get::<Counter>().unwrap().value, 128);
    }

    #[test]
    fn test_is_empty_tracks_registration() {
        let registry = ServiceRegistry::new();
        assert!(registry.is_empty());
        registry.register(Arc::new(Counter { value: 1 }));
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_app_context_carries_connection_id() {
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(AppContext::new("conn-42")));
        assert_eq!(
            registry.get::<AppContext>().unwrap().connection_id,
            "conn-42"
        );
    }
}
