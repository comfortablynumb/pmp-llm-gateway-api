//! Metadata filtering for knowledge base queries

use serde::{Deserialize, Serialize};

/// Comparison operators for metadata filters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    /// Equal to
    Eq,
    /// Not equal to
    Ne,
    /// Greater than
    Gt,
    /// Greater than or equal to
    Gte,
    /// Less than
    Lt,
    /// Less than or equal to
    Lte,
    /// Contains (for strings or arrays)
    Contains,
    /// Starts with (for strings)
    StartsWith,
    /// Ends with (for strings)
    EndsWith,
    /// In list of values
    In,
    /// Not in list of values
    NotIn,
    /// Exists (field is present)
    Exists,
    /// Not exists (field is not present)
    NotExists,
}

impl std::fmt::Display for FilterOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eq => write!(f, "="),
            Self::Ne => write!(f, "!="),
            Self::Gt => write!(f, ">"),
            Self::Gte => write!(f, ">="),
            Self::Lt => write!(f, "<"),
            Self::Lte => write!(f, "<="),
            Self::Contains => write!(f, "contains"),
            Self::StartsWith => write!(f, "starts_with"),
            Self::EndsWith => write!(f, "ends_with"),
            Self::In => write!(f, "in"),
            Self::NotIn => write!(f, "not_in"),
            Self::Exists => write!(f, "exists"),
            Self::NotExists => write!(f, "not_exists"),
        }
    }
}

/// Logical connectors for combining filters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterConnector {
    /// Logical AND
    And,
    /// Logical OR
    Or,
}

impl Default for FilterConnector {
    fn default() -> Self {
        Self::And
    }
}

/// Filter value that can be various types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// List of values (for In/NotIn operators)
    List(Vec<FilterValue>),
    /// Null value
    Null,
}

impl From<&str> for FilterValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for FilterValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<i64> for FilterValue {
    fn from(n: i64) -> Self {
        Self::Integer(n)
    }
}

impl From<i32> for FilterValue {
    fn from(n: i32) -> Self {
        Self::Integer(n as i64)
    }
}

impl From<f64> for FilterValue {
    fn from(n: f64) -> Self {
        Self::Float(n)
    }
}

impl From<f32> for FilterValue {
    fn from(n: f32) -> Self {
        Self::Float(n as f64)
    }
}

impl From<bool> for FilterValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl<T: Into<FilterValue>> From<Vec<T>> for FilterValue {
    fn from(list: Vec<T>) -> Self {
        Self::List(list.into_iter().map(|v| v.into()).collect())
    }
}

/// A single filter condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterCondition {
    /// Metadata field key
    pub key: String,
    /// Comparison operator
    pub operator: FilterOperator,
    /// Value to compare against (optional for Exists/NotExists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<FilterValue>,
}

impl FilterCondition {
    /// Create a new filter condition
    pub fn new(key: impl Into<String>, operator: FilterOperator, value: FilterValue) -> Self {
        Self {
            key: key.into(),
            operator,
            value: Some(value),
        }
    }

    /// Create an existence check condition
    pub fn exists(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            operator: FilterOperator::Exists,
            value: None,
        }
    }

    /// Create a not-exists check condition
    pub fn not_exists(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            operator: FilterOperator::NotExists,
            value: None,
        }
    }

    /// Create an equality condition
    pub fn eq(key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        Self::new(key, FilterOperator::Eq, value.into())
    }

    /// Create a not-equal condition
    pub fn ne(key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        Self::new(key, FilterOperator::Ne, value.into())
    }

    /// Create a greater-than condition
    pub fn gt(key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        Self::new(key, FilterOperator::Gt, value.into())
    }

    /// Create a greater-than-or-equal condition
    pub fn gte(key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        Self::new(key, FilterOperator::Gte, value.into())
    }

    /// Create a less-than condition
    pub fn lt(key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        Self::new(key, FilterOperator::Lt, value.into())
    }

    /// Create a less-than-or-equal condition
    pub fn lte(key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        Self::new(key, FilterOperator::Lte, value.into())
    }

    /// Create a contains condition
    pub fn contains(key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        Self::new(key, FilterOperator::Contains, value.into())
    }

    /// Create an in-list condition
    pub fn in_list(key: impl Into<String>, values: Vec<FilterValue>) -> Self {
        Self::new(key, FilterOperator::In, FilterValue::List(values))
    }

    /// Create a not-in-list condition
    pub fn not_in_list(key: impl Into<String>, values: Vec<FilterValue>) -> Self {
        Self::new(key, FilterOperator::NotIn, FilterValue::List(values))
    }
}

/// A metadata filter that can be a single condition or a group of conditions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetadataFilter {
    /// A single condition
    Condition(FilterCondition),
    /// A group of conditions with a connector
    Group {
        connector: FilterConnector,
        filters: Vec<MetadataFilter>,
    },
}

impl MetadataFilter {
    /// Create a filter from a single condition
    pub fn condition(condition: FilterCondition) -> Self {
        Self::Condition(condition)
    }

    /// Create an AND group of filters
    pub fn and(filters: Vec<MetadataFilter>) -> Self {
        Self::Group {
            connector: FilterConnector::And,
            filters,
        }
    }

    /// Create an OR group of filters
    pub fn or(filters: Vec<MetadataFilter>) -> Self {
        Self::Group {
            connector: FilterConnector::Or,
            filters,
        }
    }

    /// Check if this filter is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Condition(_) => false,
            Self::Group { filters, .. } => filters.is_empty(),
        }
    }
}

/// Builder for creating complex metadata filters
#[derive(Debug, Default)]
pub struct FilterBuilder {
    filters: Vec<MetadataFilter>,
    connector: FilterConnector,
}

impl FilterBuilder {
    /// Create a new filter builder (defaults to AND connector)
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder that uses OR connector
    pub fn or() -> Self {
        Self {
            filters: Vec::new(),
            connector: FilterConnector::Or,
        }
    }

    /// Add an equality condition
    pub fn eq(mut self, key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::eq(key, value)));
        self
    }

    /// Add a not-equal condition
    pub fn ne(mut self, key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::ne(key, value)));
        self
    }

    /// Add a greater-than condition
    pub fn gt(mut self, key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::gt(key, value)));
        self
    }

    /// Add a greater-than-or-equal condition
    pub fn gte(mut self, key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::gte(key, value)));
        self
    }

    /// Add a less-than condition
    pub fn lt(mut self, key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::lt(key, value)));
        self
    }

    /// Add a less-than-or-equal condition
    pub fn lte(mut self, key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::lte(key, value)));
        self
    }

    /// Add a contains condition
    pub fn contains(mut self, key: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::contains(
                key, value,
            )));
        self
    }

    /// Add an in-list condition
    pub fn in_list(mut self, key: impl Into<String>, values: Vec<FilterValue>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::in_list(
                key, values,
            )));
        self
    }

    /// Add an exists condition
    pub fn exists(mut self, key: impl Into<String>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::exists(key)));
        self
    }

    /// Add a not-exists condition
    pub fn not_exists(mut self, key: impl Into<String>) -> Self {
        self.filters
            .push(MetadataFilter::Condition(FilterCondition::not_exists(key)));
        self
    }

    /// Add a nested filter group
    pub fn group(mut self, filter: MetadataFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Add a raw condition
    pub fn condition(mut self, condition: FilterCondition) -> Self {
        self.filters.push(MetadataFilter::Condition(condition));
        self
    }

    /// Build the final filter
    pub fn build(self) -> Option<MetadataFilter> {
        if self.filters.is_empty() {
            return None;
        }

        if self.filters.len() == 1 {
            return Some(self.filters.into_iter().next().unwrap());
        }

        Some(MetadataFilter::Group {
            connector: self.connector,
            filters: self.filters,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_condition_eq() {
        let condition = FilterCondition::eq("category", "technical");
        assert_eq!(condition.key, "category");
        assert_eq!(condition.operator, FilterOperator::Eq);
        assert_eq!(
            condition.value,
            Some(FilterValue::String("technical".to_string()))
        );
    }

    #[test]
    fn test_filter_condition_exists() {
        let condition = FilterCondition::exists("author");
        assert_eq!(condition.key, "author");
        assert_eq!(condition.operator, FilterOperator::Exists);
        assert!(condition.value.is_none());
    }

    #[test]
    fn test_filter_condition_in_list() {
        let condition = FilterCondition::in_list(
            "status",
            vec![
                FilterValue::String("active".to_string()),
                FilterValue::String("pending".to_string()),
            ],
        );
        assert_eq!(condition.key, "status");
        assert_eq!(condition.operator, FilterOperator::In);
    }

    #[test]
    fn test_filter_builder_simple() {
        let filter = FilterBuilder::new()
            .eq("category", "docs")
            .gt("version", 1i64)
            .build();

        assert!(filter.is_some());

        if let Some(MetadataFilter::Group { connector, filters }) = filter {
            assert_eq!(connector, FilterConnector::And);
            assert_eq!(filters.len(), 2);
        } else {
            panic!("Expected a group filter");
        }
    }

    #[test]
    fn test_filter_builder_single() {
        let filter = FilterBuilder::new().eq("type", "manual").build();

        assert!(filter.is_some());

        if let Some(MetadataFilter::Condition(condition)) = filter {
            assert_eq!(condition.key, "type");
        } else {
            panic!("Expected a condition filter");
        }
    }

    #[test]
    fn test_filter_builder_empty() {
        let filter = FilterBuilder::new().build();
        assert!(filter.is_none());
    }

    #[test]
    fn test_filter_builder_or() {
        let filter = FilterBuilder::or()
            .eq("status", "active")
            .eq("status", "published")
            .build();

        if let Some(MetadataFilter::Group { connector, .. }) = filter {
            assert_eq!(connector, FilterConnector::Or);
        } else {
            panic!("Expected an OR group filter");
        }
    }

    #[test]
    fn test_nested_filters() {
        // (category = "docs" AND version > 1) OR (category = "faq")
        let inner_and = FilterBuilder::new()
            .eq("category", "docs")
            .gt("version", 1i64)
            .build()
            .unwrap();

        let inner_faq = MetadataFilter::Condition(FilterCondition::eq("category", "faq"));

        let filter = FilterBuilder::or()
            .group(inner_and)
            .group(inner_faq)
            .build();

        assert!(filter.is_some());

        if let Some(MetadataFilter::Group { connector, filters }) = filter {
            assert_eq!(connector, FilterConnector::Or);
            assert_eq!(filters.len(), 2);
        }
    }

    #[test]
    fn test_filter_value_conversions() {
        let s: FilterValue = "hello".into();
        assert!(matches!(s, FilterValue::String(_)));

        let i: FilterValue = 42i64.into();
        assert!(matches!(i, FilterValue::Integer(42)));

        let f: FilterValue = 3.14f64.into();
        assert!(matches!(f, FilterValue::Float(_)));

        let b: FilterValue = true.into();
        assert!(matches!(b, FilterValue::Boolean(true)));
    }

    #[test]
    fn test_filter_serialization() {
        let filter = FilterBuilder::new()
            .eq("category", "docs")
            .gte("score", 0.8f64)
            .build()
            .unwrap();

        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("category"));
        assert!(json.contains("docs"));

        let deserialized: MetadataFilter = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.is_empty());
    }

    #[test]
    fn test_operator_display() {
        assert_eq!(FilterOperator::Eq.to_string(), "=");
        assert_eq!(FilterOperator::Ne.to_string(), "!=");
        assert_eq!(FilterOperator::Gt.to_string(), ">");
        assert_eq!(FilterOperator::Contains.to_string(), "contains");
    }
}
