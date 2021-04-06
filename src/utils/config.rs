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
        args.flag_algorithm
            .map(|a| self.algorithm = a);
        args.flag_memory
            .map(|m| num::clamp(m, 0.0, 9999999.9))
            .map(|m| {
                self.parameters.tsn_memory = m;
                self.parameters.avb_memory = m;
            });
        args.flag_seed
            .map(|s| self.seed = s);
    }
}
