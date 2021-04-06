use serde::Deserialize;
use argh::FromArgs;

/// A mixed-criticality and online routing model for TSN network
#[derive(FromArgs)]
pub struct Arguments {
    #[argh(positional)]
    pub network: String,
    #[argh(positional)]
    pub backgrounds: String,
    #[argh(positional)]
    pub inputs: String,
    #[argh(positional)]
    pub fold: u32,
    /// path to configuration file
    #[argh(option, short='c', default="String::from(\"data/config/default.yaml\")")]
    pub config: String,
    /// override algorithm used to calculate routing set
    #[argh(option, short='a')]
    pub algorithm: Option<String>,
    /// override memory parameter for ACO algorithm
    #[argh(option, short='m')]
    pub memory: Option<f64>,
    /// override random seed for ACO or RO algorithms
    #[argh(option, short='s')]
    pub seed: Option<u64>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub name: String,
    pub algorithm: String,
    pub weights: [f64; 4],
    pub early_stop: bool,
    pub timeout: u64,
    pub seed: u64,
    pub parameters: Parameters,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Parameters {
    pub tsn_memory: f64,
    pub avb_memory: f64,
}

impl Config {
    pub fn override_from_args(&mut self, args: Arguments) {
        if let Some(algorithm) = args.algorithm {
            self.algorithm = algorithm;
        }
        if let Some(memory) = args.memory {
            let memory = num::clamp(memory, 0.0, 9999999.9);
            self.parameters.tsn_memory = memory;
            self.parameters.tsn_memory = memory;
        }
        if let Some(seed) = args.seed {
            self.seed = seed;
        }
    }
}
