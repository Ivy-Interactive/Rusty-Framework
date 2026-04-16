use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A JSON patch operation for incremental UI updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Patch {
    #[serde(rename = "replace")]
    Replace { path: String, value: Value },
    #[serde(rename = "add")]
    Add { path: String, value: Value },
    #[serde(rename = "remove")]
    Remove { path: String },
}

/// Compute patches between two JSON values representing widget trees.
pub fn diff(old: &Value, new: &Value) -> Vec<Patch> {
    let mut patches = Vec::new();
    diff_recursive("", old, new, &mut patches);
    patches
}

fn diff_recursive(path: &str, old: &Value, new: &Value, patches: &mut Vec<Patch>) {
    if old == new {
        return;
    }

    match (old, new) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            // Check for removed keys
            for key in old_map.keys() {
                if !new_map.contains_key(key) {
                    patches.push(Patch::Remove {
                        path: format!("{}/{}", path, key),
                    });
                }
            }
            // Check for added or changed keys
            for (key, new_val) in new_map {
                let child_path = format!("{}/{}", path, key);
                match old_map.get(key) {
                    Some(old_val) => diff_recursive(&child_path, old_val, new_val, patches),
                    None => patches.push(Patch::Add {
                        path: child_path,
                        value: new_val.clone(),
                    }),
                }
            }
        }
        (Value::Array(old_arr), Value::Array(new_arr)) => {
            // Simple approach: if arrays differ, replace the whole path
            if old_arr.len() != new_arr.len() {
                patches.push(Patch::Replace {
                    path: path.to_string(),
                    value: new.clone(),
                });
            } else {
                for (i, (o, n)) in old_arr.iter().zip(new_arr.iter()).enumerate() {
                    diff_recursive(&format!("{}/{}", path, i), o, n, patches);
                }
            }
        }
        _ => {
            patches.push(Patch::Replace {
                path: path.to_string(),
                value: new.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_diff_no_changes() {
        let a = json!({"type": "text", "content": "hello"});
        let b = json!({"type": "text", "content": "hello"});
        assert!(diff(&a, &b).is_empty());
    }

    #[test]
    fn test_diff_value_change() {
        let a = json!({"type": "text", "content": "hello"});
        let b = json!({"type": "text", "content": "world"});
        let patches = diff(&a, &b);
        assert_eq!(patches.len(), 1);
    }

    #[test]
    fn test_diff_key_added() {
        let a = json!({"type": "text"});
        let b = json!({"type": "text", "content": "hello"});
        let patches = diff(&a, &b);
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::Add { .. }));
    }
}
