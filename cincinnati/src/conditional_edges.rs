use smart_default::SmartDefault;

/// ConditionalEdge stores the conditional edges
#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone)]
#[serde(default)]
pub struct ConditionalEdge {
    #[serde(skip_serializing)]
    pub edge_regex: ConditionalUpdateEdge,
    pub edges: Vec<ConditionalUpdateEdge>,
    pub risks: Vec<ConditionalUpdateRisk>,
}

/// Stores an instance of the Edge
#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone, Eq, PartialEq, Hash)]
#[serde(default)]
pub struct ConditionalUpdateEdge {
    pub from: String,
    pub to: String,
}

/// Stores the Risk and its matching rules
#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone, Eq, PartialEq, Hash)]
#[serde(default)]
pub struct ConditionalUpdateRisk {
    pub url: String,
    pub name: String,
    pub message: String,
    #[serde(rename = "matchingRules")]
    pub matching_rules: Vec<ClusterCondition>,
}

/// ClusterCondition has the Type and PromQL query used to identify the blocked clusters
#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone, Eq, PartialEq, Hash)]
#[serde(default)]
pub struct ClusterCondition {
    #[serde(rename = "type")]
    pub condition_type: String,
    #[serde(skip_serializing_if = "PromQLClusterCondition::is_empty")]
    pub promql: PromQLClusterCondition,
}

/// Contains the PromQL string
#[derive(Debug, Serialize, Deserialize, SmartDefault, Clone, Eq, PartialEq, Hash)]
#[serde(default)]
pub struct PromQLClusterCondition {
    pub promql: String,
}

impl ConditionalEdge {
    /// gets the mutable vector of edges
    pub fn mut_edges(&mut self) -> &mut Vec<ConditionalUpdateEdge> {
        &mut self.edges
    }
}

impl PromQLClusterCondition {
    /// returns true if there is no PromQL condition to serialize.
    pub fn is_empty(&self) -> bool {
        self.promql.is_empty()
    }
}
