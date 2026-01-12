//! Test case repository traits and implementations

use async_trait::async_trait;

use crate::domain::error::DomainError;

use super::{TestCase, TestCaseId, TestCaseResult, TestCaseResultId, TestCaseType};

/// Query parameters for listing test cases
#[derive(Debug, Clone, Default)]
pub struct TestCaseQuery {
    /// Filter by test type
    pub test_type: Option<TestCaseType>,
    /// Filter by enabled status
    pub enabled: Option<bool>,
    /// Filter by tag
    pub tag: Option<String>,
    /// Filter by model ID (for ModelPrompt tests)
    pub model_id: Option<String>,
    /// Filter by workflow ID (for Workflow tests)
    pub workflow_id: Option<String>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl TestCaseQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_test_type(mut self, test_type: TestCaseType) -> Self {
        self.test_type = Some(test_type);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    pub fn with_workflow_id(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Query parameters for listing test results
#[derive(Debug, Clone, Default)]
pub struct TestCaseResultQuery {
    /// Filter by test case ID
    pub test_case_id: Option<TestCaseId>,
    /// Filter by passed status
    pub passed: Option<bool>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl TestCaseResultQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_test_case(test_case_id: TestCaseId) -> Self {
        Self {
            test_case_id: Some(test_case_id),
            ..Default::default()
        }
    }

    pub fn with_passed(mut self, passed: bool) -> Self {
        self.passed = Some(passed);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Repository trait for test cases
#[async_trait]
pub trait TestCaseRepository: Send + Sync {
    /// Get a test case by ID
    async fn get(&self, id: &TestCaseId) -> Result<Option<TestCase>, DomainError>;

    /// List test cases matching query
    async fn list(&self, query: &TestCaseQuery) -> Result<Vec<TestCase>, DomainError>;

    /// Count test cases matching query
    async fn count(&self, query: &TestCaseQuery) -> Result<usize, DomainError>;

    /// Save a test case (create or update)
    async fn save(&self, test_case: &TestCase) -> Result<(), DomainError>;

    /// Delete a test case
    async fn delete(&self, id: &TestCaseId) -> Result<bool, DomainError>;

    /// Check if a test case exists
    async fn exists(&self, id: &TestCaseId) -> Result<bool, DomainError>;
}

/// Repository trait for test case results
#[async_trait]
pub trait TestCaseResultRepository: Send + Sync {
    /// Get a result by ID
    async fn get(&self, id: &TestCaseResultId) -> Result<Option<TestCaseResult>, DomainError>;

    /// List results matching query
    async fn list(&self, query: &TestCaseResultQuery) -> Result<Vec<TestCaseResult>, DomainError>;

    /// Save a result
    async fn save(&self, result: &TestCaseResult) -> Result<(), DomainError>;

    /// Delete results for a test case
    async fn delete_for_test_case(&self, test_case_id: &TestCaseId) -> Result<usize, DomainError>;

    /// Get the latest result for a test case
    async fn get_latest(&self, test_case_id: &TestCaseId) -> Result<Option<TestCaseResult>, DomainError>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    use super::super::{ModelPromptInput, TestCaseInput};

    /// In-memory mock repository for testing
    pub struct MockTestCaseRepository {
        test_cases: RwLock<HashMap<String, TestCase>>,
    }

    impl MockTestCaseRepository {
        pub fn new() -> Self {
            Self {
                test_cases: RwLock::new(HashMap::new()),
            }
        }
    }

    impl Default for MockTestCaseRepository {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl TestCaseRepository for MockTestCaseRepository {
        async fn get(&self, id: &TestCaseId) -> Result<Option<TestCase>, DomainError> {
            let guard = self
                .test_cases
                .read()
                .map_err(|_| DomainError::internal("Lock poisoned"))?;

            Ok(guard.get(id.as_str()).cloned())
        }

        async fn list(&self, query: &TestCaseQuery) -> Result<Vec<TestCase>, DomainError> {
            let guard = self
                .test_cases
                .read()
                .map_err(|_| DomainError::internal("Lock poisoned"))?;

            let mut results: Vec<TestCase> = guard
                .values()
                .filter(|tc: &&TestCase| {
                    // Filter by test type
                    if let Some(ref test_type) = query.test_type {
                        if tc.test_type() != test_type {
                            return false;
                        }
                    }

                    // Filter by enabled
                    if let Some(enabled) = query.enabled {
                        if tc.is_enabled() != enabled {
                            return false;
                        }
                    }

                    // Filter by tag
                    if let Some(ref tag) = query.tag {
                        if !tc.tags().contains(tag) {
                            return false;
                        }
                    }

                    // Filter by model_id
                    if let Some(ref model_id) = query.model_id {
                        if let TestCaseInput::ModelPrompt(input) = tc.input() {
                            if &input.model_id != model_id {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }

                    // Filter by workflow_id
                    if let Some(ref workflow_id) = query.workflow_id {
                        if let TestCaseInput::Workflow(input) = tc.input() {
                            if &input.workflow_id != workflow_id {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }

                    true
                })
                .cloned()
                .collect();

            // Sort by name
            results.sort_by(|a: &TestCase, b: &TestCase| a.name().cmp(b.name()));

            // Apply pagination
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(usize::MAX);

            Ok(results.into_iter().skip(offset).take(limit).collect())
        }

        async fn count(&self, query: &TestCaseQuery) -> Result<usize, DomainError> {
            let results = self.list(query).await?;
            Ok(results.len())
        }

        async fn save(&self, test_case: &TestCase) -> Result<(), DomainError> {
            let mut guard = self
                .test_cases
                .write()
                .map_err(|_| DomainError::internal("Lock poisoned"))?;

            guard.insert(test_case.id().as_str().to_string(), test_case.clone());
            Ok(())
        }

        async fn delete(&self, id: &TestCaseId) -> Result<bool, DomainError> {
            let mut guard = self
                .test_cases
                .write()
                .map_err(|_| DomainError::internal("Lock poisoned"))?;

            Ok(guard.remove(id.as_str()).is_some())
        }

        async fn exists(&self, id: &TestCaseId) -> Result<bool, DomainError> {
            let guard = self
                .test_cases
                .read()
                .map_err(|_| DomainError::internal("Lock poisoned"))?;

            Ok(guard.contains_key(id.as_str()))
        }
    }

    /// In-memory mock repository for test results
    pub struct MockTestCaseResultRepository {
        results: RwLock<HashMap<String, TestCaseResult>>,
    }

    impl MockTestCaseResultRepository {
        pub fn new() -> Self {
            Self {
                results: RwLock::new(HashMap::new()),
            }
        }
    }

    impl Default for MockTestCaseResultRepository {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl TestCaseResultRepository for MockTestCaseResultRepository {
        async fn get(&self, id: &TestCaseResultId) -> Result<Option<TestCaseResult>, DomainError> {
            let guard = self
                .results
                .read()
                .map_err(|_| DomainError::internal("Lock poisoned"))?;

            Ok(guard.get(id.as_str()).cloned())
        }

        async fn list(&self, query: &TestCaseResultQuery) -> Result<Vec<TestCaseResult>, DomainError> {
            let guard = self
                .results
                .read()
                .map_err(|_| DomainError::internal("Lock poisoned"))?;

            let mut results: Vec<TestCaseResult> = guard
                .values()
                .filter(|r: &&TestCaseResult| {
                    // Filter by test case ID
                    if let Some(ref tc_id) = query.test_case_id {
                        if r.test_case_id() != tc_id {
                            return false;
                        }
                    }

                    // Filter by passed
                    if let Some(passed) = query.passed {
                        if r.passed() != passed {
                            return false;
                        }
                    }

                    true
                })
                .cloned()
                .collect();

            // Sort by executed_at descending (newest first)
            results.sort_by(|a: &TestCaseResult, b: &TestCaseResult| {
                b.executed_at().cmp(&a.executed_at())
            });

            // Apply pagination
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(usize::MAX);

            Ok(results.into_iter().skip(offset).take(limit).collect())
        }

        async fn save(&self, result: &TestCaseResult) -> Result<(), DomainError> {
            let mut guard = self
                .results
                .write()
                .map_err(|_| DomainError::internal("Lock poisoned"))?;

            guard.insert(result.id().as_str().to_string(), result.clone());
            Ok(())
        }

        async fn delete_for_test_case(&self, test_case_id: &TestCaseId) -> Result<usize, DomainError> {
            let mut guard = self
                .results
                .write()
                .map_err(|_| DomainError::internal("Lock poisoned"))?;

            let ids_to_remove: Vec<String> = guard
                .iter()
                .filter(|(_, r): &(&String, &TestCaseResult)| r.test_case_id() == test_case_id)
                .map(|(id, _): (&String, &TestCaseResult)| id.clone())
                .collect();

            let count = ids_to_remove.len();

            for id in ids_to_remove {
                guard.remove(&id);
            }

            Ok(count)
        }

        async fn get_latest(&self, test_case_id: &TestCaseId) -> Result<Option<TestCaseResult>, DomainError> {
            let query = TestCaseResultQuery::for_test_case(test_case_id.clone()).with_limit(1);
            let results = self.list(&query).await?;

            Ok(results.into_iter().next())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn create_test_case(id: &str, name: &str) -> TestCase {
            TestCase::model_prompt(
                TestCaseId::new(id).unwrap(),
                name,
                ModelPromptInput {
                    model_id: "gpt-4".to_string(),
                    prompt_id: None,
                    variables: std::collections::HashMap::new(),
                    user_message: "Hello".to_string(),
                    temperature: None,
                    max_tokens: None,
                },
            )
        }

        #[tokio::test]
        async fn test_mock_repository_crud() {
            let repo = MockTestCaseRepository::new();

            let test_case = create_test_case("test-1", "Test 1");

            // Save
            repo.save(&test_case).await.unwrap();

            // Get
            let retrieved = repo.get(test_case.id()).await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().name(), "Test 1");

            // Exists
            assert!(repo.exists(test_case.id()).await.unwrap());

            // Delete
            assert!(repo.delete(test_case.id()).await.unwrap());
            assert!(!repo.exists(test_case.id()).await.unwrap());
        }

        #[tokio::test]
        async fn test_mock_repository_list() {
            let repo = MockTestCaseRepository::new();

            repo.save(&create_test_case("test-1", "Alpha")).await.unwrap();
            repo.save(&create_test_case("test-2", "Beta")).await.unwrap();
            repo.save(&create_test_case("test-3", "Gamma")).await.unwrap();

            // List all
            let all = repo.list(&TestCaseQuery::new()).await.unwrap();
            assert_eq!(all.len(), 3);

            // List with limit
            let limited = repo.list(&TestCaseQuery::new().with_limit(2)).await.unwrap();
            assert_eq!(limited.len(), 2);

            // Count
            let count = repo.count(&TestCaseQuery::new()).await.unwrap();
            assert_eq!(count, 3);
        }
    }
}
