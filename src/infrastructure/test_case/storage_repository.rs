//! Storage-backed test case repository implementations

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::storage::Storage;
use crate::domain::test_case::{
    TestCase, TestCaseId, TestCaseInput, TestCaseQuery, TestCaseRepository, TestCaseResult,
    TestCaseResultId, TestCaseResultQuery, TestCaseResultRepository,
};
use crate::domain::DomainError;

/// Storage-backed implementation of TestCaseRepository
#[derive(Debug)]
pub struct StorageTestCaseRepository {
    storage: Arc<dyn Storage<TestCase>>,
}

impl StorageTestCaseRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<TestCase>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl TestCaseRepository for StorageTestCaseRepository {
    async fn get(&self, id: &TestCaseId) -> Result<Option<TestCase>, DomainError> {
        self.storage.get(id).await
    }

    async fn list(&self, query: &TestCaseQuery) -> Result<Vec<TestCase>, DomainError> {
        let all = self.storage.list().await?;

        let mut results: Vec<TestCase> = all
            .into_iter()
            .filter(|tc| {
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
            .collect();

        // Sort by name
        results.sort_by(|a, b| a.name().cmp(b.name()));

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
        if self.storage.exists(test_case.id()).await? {
            self.storage.update(test_case.clone()).await?;
        } else {
            self.storage.create(test_case.clone()).await?;
        }

        Ok(())
    }

    async fn delete(&self, id: &TestCaseId) -> Result<bool, DomainError> {
        self.storage.delete(id).await
    }

    async fn exists(&self, id: &TestCaseId) -> Result<bool, DomainError> {
        self.storage.exists(id).await
    }
}

/// Storage-backed implementation of TestCaseResultRepository
#[derive(Debug)]
pub struct StorageTestCaseResultRepository {
    storage: Arc<dyn Storage<TestCaseResult>>,
}

impl StorageTestCaseResultRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<TestCaseResult>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl TestCaseResultRepository for StorageTestCaseResultRepository {
    async fn get(&self, id: &TestCaseResultId) -> Result<Option<TestCaseResult>, DomainError> {
        self.storage.get(id).await
    }

    async fn list(&self, query: &TestCaseResultQuery) -> Result<Vec<TestCaseResult>, DomainError> {
        let all = self.storage.list().await?;

        let mut results: Vec<TestCaseResult> = all
            .into_iter()
            .filter(|r| {
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
            .collect();

        // Sort by executed_at descending (newest first)
        results.sort_by(|a, b| b.executed_at().cmp(&a.executed_at()));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    async fn save(&self, result: &TestCaseResult) -> Result<(), DomainError> {
        if self.storage.exists(result.id()).await? {
            self.storage.update(result.clone()).await?;
        } else {
            self.storage.create(result.clone()).await?;
        }

        Ok(())
    }

    async fn delete_for_test_case(&self, test_case_id: &TestCaseId) -> Result<usize, DomainError> {
        let all = self.storage.list().await?;
        let mut deleted = 0;

        for result in all {
            if result.test_case_id() == test_case_id {
                if self.storage.delete(result.id()).await? {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }

    async fn get_latest(
        &self,
        test_case_id: &TestCaseId,
    ) -> Result<Option<TestCaseResult>, DomainError> {
        let query = TestCaseResultQuery::for_test_case(test_case_id.clone()).with_limit(1);
        let results = self.list(&query).await?;

        Ok(results.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::test_case::ModelPromptInput;
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_test_case_repo() -> StorageTestCaseRepository {
        let storage = Arc::new(InMemoryStorage::<TestCase>::new());
        StorageTestCaseRepository::new(storage)
    }

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
    async fn test_repository_crud() {
        let repo = create_test_case_repo();
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
    async fn test_repository_list() {
        let repo = create_test_case_repo();

        repo.save(&create_test_case("test-1", "Alpha"))
            .await
            .unwrap();
        repo.save(&create_test_case("test-2", "Beta"))
            .await
            .unwrap();
        repo.save(&create_test_case("test-3", "Gamma"))
            .await
            .unwrap();

        // List all
        let all = repo.list(&TestCaseQuery::new()).await.unwrap();
        assert_eq!(all.len(), 3);

        // List with limit
        let limited = repo
            .list(&TestCaseQuery::new().with_limit(2))
            .await
            .unwrap();
        assert_eq!(limited.len(), 2);

        // Count
        let count = repo.count(&TestCaseQuery::new()).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_update_existing() {
        let repo = create_test_case_repo();
        let mut test_case = create_test_case("test-1", "Test 1");

        // Save
        repo.save(&test_case).await.unwrap();

        // Update
        test_case.set_enabled(false);
        repo.save(&test_case).await.unwrap();

        // Verify
        let retrieved = repo.get(test_case.id()).await.unwrap().unwrap();
        assert!(!retrieved.is_enabled());
    }
}
