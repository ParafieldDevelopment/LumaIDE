//! Dependency validation and topological load ordering.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};

use crate::manifest::ExtensionDescriptor;

/// Prevent a maliciously long dependency chain from exhausting the stack.
const MAX_DEPENDENCY_DEPTH: usize = 256;

/// The order in which extensions should be loaded, plus optional dependencies
/// that were not satisfied and should be ignored.
#[derive(Debug, Default)]
pub struct Resolution {
    /// Indices into the descriptor slice passed to the resolver.
    pub order: Vec<usize>,
    /// Extension name -> optional dependencies that are not installed.
    pub skipped_optional: HashMap<String, Vec<String>>,
    /// Extension name -> reason it cannot be loaded. Only populated by
    /// [`resolve_load_order_with_failures`].
    pub failures: HashMap<String, String>,
}

#[derive(Debug)]
struct SortFailure {
    names: Vec<String>,
    message: String,
}

/// Validate dependencies and return a load order where every extension comes
/// after all of its dependencies. Any invalid extension is an error.
pub fn resolve_load_order(extensions: &[ExtensionDescriptor]) -> Result<Resolution> {
    let resolution = resolve_load_order_with_failures(extensions)?;
    if let Some(error) = resolution.failures.values().next() {
        bail!("{error}");
    }
    Ok(resolution)
}

/// Validate dependencies and return a load order that excludes invalid
/// extensions. Problems are reported per extension instead of aborting the
/// whole load, so unrelated extensions can still start.
pub fn resolve_load_order_with_failures(extensions: &[ExtensionDescriptor]) -> Result<Resolution> {
    let mut by_name: HashMap<&str, usize> = HashMap::new();
    let mut invalid = HashSet::new();
    let mut failures: HashMap<String, String> = HashMap::new();

    for (index, extension) in extensions.iter().enumerate() {
        let name = extension.manifest.name.as_str();
        if let Some(&first) = by_name.get(name) {
            invalid.insert(first);
            invalid.insert(index);
            failures.insert(
                name.to_string(),
                format!("Duplicate extension name '{name}'"),
            );
        } else {
            by_name.insert(name, index);
        }
    }

    let mut dependencies: Vec<Vec<usize>> = vec![Vec::new(); extensions.len()];
    let mut skipped_optional: HashMap<String, Vec<String>> = HashMap::new();

    for (index, extension) in extensions.iter().enumerate() {
        if invalid.contains(&index) {
            continue;
        }
        let name = &extension.manifest.name;
        for dependency in &extension.manifest.dependencies {
            let problem = match by_name.get(dependency.name.as_str()).copied() {
                None => Some(format!("requires missing extension '{}'", dependency.name)),
                Some(dep_index) if invalid.contains(&dep_index) => Some(format!(
                    "requires extension '{}' which failed validation",
                    dependency.name
                )),
                Some(dep_index) => {
                    let dep_version = &extensions[dep_index].manifest.version;
                    if !dependency.version.matches(dep_version) {
                        Some(format!(
                            "requires '{}' {}, but {} is installed",
                            dependency.name, dependency.version, dep_version
                        ))
                    } else {
                        dependencies[index].push(dep_index);
                        None
                    }
                }
            };

            if let Some(problem) = problem {
                if dependency.optional {
                    skipped_optional
                        .entry(name.clone())
                        .or_default()
                        .push(dependency.name.clone());
                } else {
                    invalid.insert(index);
                    failures.insert(name.clone(), format!("Extension '{name}' {problem}"));
                    break;
                }
            }
        }
    }

    let mut valid: Vec<usize> = (0..extensions.len())
        .filter(|index| !invalid.contains(index))
        .collect();

    let order = loop {
        let graph: HashMap<usize, Vec<usize>> = valid
            .iter()
            .map(|&index| {
                (
                    index,
                    dependencies[index]
                        .iter()
                        .copied()
                        .filter(|dep| !invalid.contains(dep))
                        .collect(),
                )
            })
            .collect();

        match topological_sort(&valid, &graph, extensions) {
            Ok(order) => break order,
            Err(failure) => {
                let mut changed = false;
                for name in &failure.names {
                    if let Some(&index) = by_name.get(name.as_str())
                        && invalid.insert(index)
                    {
                        changed = true;
                    }
                    failures
                        .entry(name.clone())
                        .or_insert_with(|| failure.message.clone());
                }
                if !changed {
                    bail!("{}", failure.message);
                }
                valid.retain(|index| !invalid.contains(index));
            }
        }
    };

    // `order` is stable because descriptors are sorted by name.
    debug_assert_eq!(
        order.iter().copied().collect::<HashSet<_>>().len(),
        order.len()
    );

    Ok(Resolution {
        order,
        skipped_optional,
        failures,
    })
}

fn topological_sort(
    valid: &[usize],
    dependencies: &HashMap<usize, Vec<usize>>,
    extensions: &[ExtensionDescriptor],
) -> std::result::Result<Vec<usize>, SortFailure> {
    let mut state: HashMap<usize, u8> = valid.iter().map(|&index| (index, 0)).collect();
    let mut stack: Vec<String> = Vec::new();
    let mut order = Vec::with_capacity(valid.len());

    fn visit(
        index: usize,
        dependencies: &HashMap<usize, Vec<usize>>,
        extensions: &[ExtensionDescriptor],
        state: &mut HashMap<usize, u8>,
        stack: &mut Vec<String>,
        order: &mut Vec<usize>,
        depth: usize,
    ) -> std::result::Result<(), SortFailure> {
        if depth > MAX_DEPENDENCY_DEPTH {
            return Err(SortFailure {
                names: vec![extensions[index].manifest.name.clone()],
                message: format!(
                    "Extension dependency chain exceeds {MAX_DEPENDENCY_DEPTH} levels"
                ),
            });
        }
        match state.get(&index).copied().unwrap_or(0) {
            2 => return Ok(()),
            1 => {
                let start = stack
                    .iter()
                    .position(|name| name == &extensions[index].manifest.name)
                    .unwrap_or(0);
                return Err(SortFailure {
                    names: stack[start..].to_vec(),
                    message: format!(
                        "Cyclic extension dependency: {} -> {}",
                        stack[start..].join(" -> "),
                        extensions[index].manifest.name
                    ),
                });
            }
            _ => {}
        }
        state.insert(index, 1);
        stack.push(extensions[index].manifest.name.clone());
        for &dep in dependencies.get(&index).into_iter().flatten() {
            visit(
                dep,
                dependencies,
                extensions,
                state,
                stack,
                order,
                depth + 1,
            )?;
        }
        stack.pop();
        state.insert(index, 2);
        order.push(index);
        Ok(())
    }

    for &index in valid {
        visit(
            index,
            dependencies,
            extensions,
            &mut state,
            &mut stack,
            &mut order,
            1,
        )?;
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::manifest::ExtensionManifest;

    fn descriptor(json: &str) -> ExtensionDescriptor {
        let manifest: ExtensionManifest = serde_json::from_str(json).unwrap();
        ExtensionDescriptor {
            manifest,
            root: PathBuf::from("."),
            manifest_path: PathBuf::from("extension.json"),
        }
    }

    fn wasm(name: &str, deps: &str) -> ExtensionDescriptor {
        descriptor(&format!(
            r#"{{"name":"{name}","version":"1.0.0","type":"wasm","entry":"{name}.wasm","dependencies":{deps}}}"#
        ))
    }

    #[test]
    fn orders_dependencies_first() {
        let extensions = vec![
            wasm("a", r#"[{"name":"b"}]"#),
            wasm("b", r#"[{"name":"c"}]"#),
            wasm("c", "[]"),
        ];
        let resolution = resolve_load_order(&extensions).unwrap();
        let names: Vec<_> = resolution
            .order
            .iter()
            .map(|&i| extensions[i].manifest.name.as_str())
            .collect();
        assert_eq!(names, vec!["c", "b", "a"]);
    }

    #[test]
    fn rejects_cycles_and_missing_dependencies() {
        let cycle = vec![
            wasm("a", r#"[{"name":"b"}]"#),
            wasm("b", r#"[{"name":"a"}]"#),
        ];
        assert!(resolve_load_order(&cycle).is_err());

        let missing = vec![wasm("a", r#"[{"name":"nope"}]"#)];
        assert!(resolve_load_order(&missing).is_err());
    }

    #[test]
    fn skips_unsatisfied_optional_dependencies() {
        let extensions = vec![wasm(
            "a",
            r#"[{"name":"missing","optional":true},{"name":"wrong","version":"^2.0.0","optional":true}]"#,
        )];
        // `wrong` is absent too; both should be reported as skipped.
        let resolution = resolve_load_order(&extensions).unwrap();
        assert_eq!(resolution.order, vec![0]);
        assert_eq!(resolution.skipped_optional["a"].len(), 2);
    }

    #[test]
    fn rejects_incompatible_required_versions() {
        let extensions = vec![
            wasm("a", r#"[{"name":"b","version":"^2.0.0"}]"#),
            wasm("b", "[]"),
        ];
        assert!(resolve_load_order(&extensions).is_err());
    }

    #[test]
    fn lenient_resolution_skips_only_invalid_extensions() {
        let extensions = vec![wasm("a", r#"[{"name":"nope"}]"#), wasm("b", "[]")];
        let resolution = resolve_load_order_with_failures(&extensions).unwrap();
        let names: Vec<_> = resolution
            .order
            .iter()
            .map(|&i| extensions[i].manifest.name.as_str())
            .collect();
        assert_eq!(names, vec!["b"]);
        assert!(resolution.failures.contains_key("a"));
    }
}
