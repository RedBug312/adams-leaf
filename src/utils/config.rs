use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Args {
    pub arg_network: String,
    pub arg_backgrounds: String,
    pub arg_inputs: String,
    pub arg_fold: u32,
    pub flag_config: Option<String>,
    pub flag_algorithm: Option<String>,
    pub flag_memory: Option<f64>,
    pub flag_seed: Option<u64>,
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
    pub fn override_from_args(&mut self, args: Args) {
        if let Some(flag) = args.flag_algorithm {
            self.algorithm = flag;
        }
        if let Some(flag) = args.flag_memory {
            self.parameters.tsn_memory = num::clamp(flag, 0.0, 9999999.9);
            self.parameters.avb_memory = num::clamp(flag, 0.0, 9999999.9);
        }
        if let Some(flag) = args.flag_seed {
            self.seed = flag;
        }
    }
}
