//! `assign_wariness`: assigns a wariness score/threshold to each request.
//!
//! ## Parameters
//!
//!  * `input_parameters`: set of request parameters used to compute the value (default: empty).
//!  * `output_key`: parameter key used to store computed value (default: `throttle_threshold`).
//!  * `discard_existing`: whether to discard and override any existing value (default: `false`).

use crate::plugins::{
    AsyncIO, BoxedPlugin, InternalIO, InternalPlugin, InternalPluginWrapper, PluginSettings,
};
use failure::Fallible;
use std::collections::{BTreeSet, HashMap};

/// Default parameter key name (output and client input).
static DEFAULT_PARAM: &str = "rollout_wariness";

/// Minimum throttling threshold.
const WARINESS_MIN: f64 = 0.0;

/// Maximum throttling threshold.
const WARINESS_MAX: f64 = 1.0;

/// `assign-throttle` policy-plugin.
#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct AssignWariness {
    /// Name of the input client parameters.
    #[default(BTreeSet::new())]
    pub input_parameters: BTreeSet<String>,

    /// Name of the output parameter.
    #[default(DEFAULT_PARAM.to_string())]
    pub output_key: String,

    /// Whether to override any existing parameter.
    #[default(false)]
    pub discard_existing: bool,
}

impl PluginSettings for AssignWariness {
    fn build_plugin(&self, _registry: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        Ok(Box::new(InternalPluginWrapper(self.clone())))
    }
}

impl AssignWariness {
    /// Plugin name, for configuration.
    pub(crate) const PLUGIN_NAME: &'static str = "assign_wariness";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let plugin: Self = cfg.try_into()?;

        ensure!(
            !plugin.input_parameters.is_empty(),
            "empty set of input parameters"
        );
        for input in &plugin.input_parameters {
            ensure!(!input.is_empty(), "empty input parameter name");
        }
        ensure!(!plugin.output_key.is_empty(), "empty output parameter name");

        Ok(Box::new(plugin))
    }

    /// Compute throttle value from input parameters.
    ///
    /// This derives a throttle score in the range `(0, 1.0]`, hashing all configured
    /// input parameters (stable sorted).
    fn compute_throttle(&self, parameters: &HashMap<String, String>) -> Fallible<f64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Left limit not included in range.
        const COMPUTED_MIN: f64 = WARINESS_MIN + 0.000001;
        const COMPUTED_MAX: f64 = WARINESS_MAX;

        // Hash all input parameters.
        let mut hasher = DefaultHasher::default();
        for key in &self.input_parameters {
            let value = parameters.get(key).cloned().unwrap_or_default();
            value.hash(&mut hasher);
        }
        let digest = hasher.finish();

        // Scale down.
        let scaled = (digest as f64) / (std::u64::MAX as f64);
        // Clamp within limits.
        let clamped = scaled.max(COMPUTED_MIN).min(COMPUTED_MAX);

        Ok(clamped)
    }

    /// Assign wariness to a request.
    ///
    /// This can either compute and insert a new wariness value, or
    /// override an existing one, or pass-through a client-provided value.
    fn try_assign_wariness(&self, io: InternalIO) -> Fallible<InternalIO> {
        let (graph, mut parameters) = (io.graph, io.parameters);

        // Optionally clean any client-provided throttling hint.
        if self.discard_existing {
            parameters.remove(&self.output_key);
        }

        let score = match parameters.get(&self.output_key).cloned() {
            None => self.compute_throttle(&parameters)?,
            Some(input) => input.parse()?,
        };

        // Clamp minimum and maximum score, truncate to 6-decimals precision.
        let clamped = score.max(WARINESS_MIN).min(WARINESS_MAX);
        let value = format!("{:.6}", clamped);

        parameters.insert(self.output_key.clone(), value);

        Ok(InternalIO { graph, parameters })
    }
}

impl InternalPlugin for AssignWariness {
    fn run_internal(self: &Self, io: InternalIO) -> AsyncIO<InternalIO> {
        let try_assign = self.try_assign_wariness(io);
        Box::new(futures::future::result(try_assign))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commons::testing::init_runtime;
    use futures::prelude::*;
    use maplit::{btreeset, hashmap};

    #[test]
    fn deserialize_config() {
        let empty: toml::Value = toml::from_str("").unwrap();
        AssignWariness::deserialize_config(empty).unwrap_err();

        let wrong_name: toml::Value = toml::from_str("name = 'foo'").unwrap();
        AssignWariness::deserialize_config(wrong_name).unwrap_err();

        let cfg = r#"
            name = "assign_throttle"
            discard_existing = true
            input_parameters = ["foo"]
            output_key = "bar"
        "#;
        let toml_cfg: toml::Value = toml::from_str(cfg).unwrap();
        let settings = AssignWariness::deserialize_config(toml_cfg).unwrap();
        settings.build_plugin(None).unwrap();
    }

    #[test]
    fn valid_client_throttle_input() {
        let mut runtime = init_runtime().unwrap();

        let throttle_key = "throttle".to_string();
        let throttle_in: f64 = 0.5;
        let graph = crate::tests::generate_graph();
        let parameters = hashmap! {
            throttle_key.clone() => throttle_in.to_string(),
        };
        let plugin = AssignWariness {
            input_parameters: btreeset!(),
            output_key: throttle_key.clone(),
            discard_existing: false,
        };

        let processed = plugin
            .run_internal(InternalIO { graph, parameters })
            .map(|out| out.parameters);
        let mut out_params = runtime.block_on(processed).unwrap();

        let throttle_val = out_params.remove(&throttle_key).unwrap();
        let score: f64 = throttle_val.parse().unwrap();

        assert_eq!(score, throttle_in);
    }

    #[test]
    fn invalid_client_throttle_input() {
        let mut runtime = init_runtime().unwrap();

        let throttle_key = "throttle".to_string();
        let graph = crate::tests::generate_graph();
        let parameters = hashmap! {
            throttle_key.clone() => "invalid".to_string(),
        };
        let plugin = AssignWariness {
            input_parameters: btreeset!(),
            output_key: throttle_key.clone(),
            discard_existing: false,
        };

        let processed = plugin.run_internal(InternalIO { graph, parameters });

        runtime.block_on(processed).unwrap_err();
    }

    #[test]
    fn capped_client_throttle_input() {
        let mut runtime = init_runtime().unwrap();

        let throttle_key = "throttle".to_string();
        let throttle_in = WARINESS_MAX;
        let graph = crate::tests::generate_graph();
        let parameters = hashmap! {
            throttle_key.clone() => throttle_in.to_string(),
        };
        let plugin = AssignWariness {
            input_parameters: btreeset!(),
            output_key: throttle_key.clone(),
            discard_existing: false,
        };

        let processed = plugin
            .run_internal(InternalIO { graph, parameters })
            .map(|out| out.parameters);
        let mut out_params = runtime.block_on(processed).unwrap();

        let throttle_val = out_params.remove(&throttle_key).unwrap();
        let score: f64 = throttle_val.parse().unwrap();

        assert_eq!(score, WARINESS_MAX);
    }

    #[test]
    fn assign_throttle() {
        let mut runtime = init_runtime().unwrap();

        let input = "version".to_string();
        let output_key = "throttle".to_string();
        let graph = crate::tests::generate_graph();
        let parameters = hashmap! {
            input.clone() => "foo".to_string(),
        };
        let plugin = AssignWariness {
            input_parameters: btreeset!(input.clone()),
            output_key: output_key.clone(),
            discard_existing: false,
        };

        let processed = plugin
            .run_internal(InternalIO { graph, parameters })
            .map(|out| out.parameters);
        let mut out_params = runtime.block_on(processed).unwrap();

        let throttle_val = out_params.remove(&output_key).unwrap();
        let score: f64 = throttle_val.parse().unwrap();

        // Pre-computed score from fixed-inputs digest.
        assert_eq!(score, 0.244317);
    }

    #[test]
    fn override_throttle() {
        let mut runtime = init_runtime().unwrap();

        let input = "channel".to_string();
        let output_key = "throttle".to_string();
        let graph = crate::tests::generate_graph();
        let parameters = hashmap! {
            output_key.clone() => std::f64::MAX.to_string(),
            input.clone() => "bar".to_string(),
        };
        let plugin = AssignWariness {
            input_parameters: btreeset!(input.clone()),
            output_key: output_key.clone(),
            discard_existing: true,
        };

        let processed = plugin
            .run_internal(InternalIO { graph, parameters })
            .map(|out| out.parameters);
        let mut out_params = runtime.block_on(processed).unwrap();

        let throttle_val = out_params.remove(&output_key).unwrap();
        let score: f64 = throttle_val.parse().unwrap();

        // Pre-computed score from fixed-inputs digest.
        assert_eq!(score, 0.1993);
    }
}
