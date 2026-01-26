//! Flint adapter implementation for `SteelMC`.

use flint_steel::{FlintAdapter, FlintWorld, ServerInfo};

use crate::world::SteelTestWorld;

/// Adapter for running Flint tests against `SteelMC`.
///
/// This adapter creates test worlds that use the real steel-core World
/// with RAM-only storage for instant chunk creation.
pub struct SteelAdapter {
    /// Server info for identification
    info: ServerInfo,
}

impl SteelAdapter {
    /// Creates a new Steel adapter.
    ///
    /// Note: You must call `steel_flint::init()` before creating an adapter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            info: ServerInfo {
                minecraft_version: "1.21.11".to_string(),
            },
        }
    }
}

impl Default for SteelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl FlintAdapter for SteelAdapter {
    fn create_test_world(&self) -> Box<dyn FlintWorld> {
        Box::new(SteelTestWorld::new())
    }

    fn server_info(&self) -> ServerInfo {
        self.info.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_registries;
    use crate::{TestLoader, TestRunner};
    use flint_core::test_spec;
    use std::path::PathBuf;

    #[test]
    fn test_run_fence_row_connections() {
        init_test_registries();

        // Load the fence test
        let test_path =
            PathBuf::from("../../../flint/FlintBenchmark/tests/fences/fence_row_connections.json");
        let spec = test_spec::TestSpec::from_file(&test_path).expect("Failed to load fence test");

        // Create adapter and runner
        let adapter = SteelAdapter::new();
        let runner = TestRunner::new(&adapter);

        // Run the test
        let result = runner.run_test(&spec);

        // Check result
        println!(
            "Test '{}': success={}, ticks={}, time={}ms",
            result.test_name, result.success, result.total_ticks, result.execution_time_ms
        );

        for assertion in &result.assertions {
            if !assertion.success {
                println!(
                    "  FAILED at tick {}: {}",
                    assertion.tick,
                    assertion.error_message.as_deref().unwrap_or("")
                );
            }
        }

        assert!(result.success, "Fence row connections test failed");
    }

    #[test]
    fn test_run_all_flint_benchmarks() {
        init_test_registries();

        let test_dir = PathBuf::from("../../../flint/FlintBenchmark/tests");
        if !test_dir.exists() {
            println!("FlintBenchmark tests directory not found, skipping");
            return;
        }

        let loader = TestLoader::new(&test_dir, true).expect("Failed to create test loader");
        let paths = loader
            .collect_all_test_files()
            .expect("Failed to collect test files");

        if paths.is_empty() {
            println!("No test files found in FlintBenchmark");
            return;
        }

        // Load all test specs from paths
        let specs: Vec<test_spec::TestSpec> = paths
            .iter()
            .filter_map(|path| {
                test_spec::TestSpec::from_file(path)
                    .map_err(|e| println!("Failed to load {}: {}", path.display(), e))
                    .ok()
            })
            .collect();

        let adapter = SteelAdapter::new();
        let runner = TestRunner::new(&adapter);
        let summary = runner.run_tests(&specs);

        println!("\n=== Flint Benchmark Results ===");
        println!(
            "Total: {}, Passed: {}, Failed: {}",
            summary.total_tests, summary.passed_tests, summary.failed_tests
        );

        for result in &summary.results {
            let status = if result.success { "PASS" } else { "FAIL" };
            println!(
                "  [{}] {} ({}ms)",
                status, result.test_name, result.execution_time_ms
            );

            if !result.success {
                for assertion in &result.assertions {
                    if !assertion.success {
                        println!(
                            "    -> tick {}: {}",
                            assertion.tick,
                            assertion.error_message.as_deref().unwrap_or("")
                        );
                    }
                }
            }
        }
    }
}
