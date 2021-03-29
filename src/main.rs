use adams_leaf::utils::json;
use adams_leaf::utils::config::Config;
use adams_leaf::cnc::CNC;
use argh::FromArgs;

/// A mixed-criticality and online routing model for TSN network
#[derive(FromArgs)]
struct Arguments {
    #[argh(positional)]
    algorithm: String,
    #[argh(positional)]
    network: String,
    #[argh(positional)]
    backgrounds: String,
    #[argh(positional)]
    inputs: String,
    #[argh(positional)]
    fold: u32,
    /// path to configuration file
    #[argh(option, short='c', default="String::from(\"config.example.json\")")]
    config: String,
}

fn main() {
    let args: Arguments = argh::from_env();

    let network = json::load_network(&args.network);
    let (tsns1, avbs1) = json::load_streams(&args.backgrounds, 1);
    let (tsns2, avbs2) = json::load_streams(&args.inputs, args.fold);
    Config::load_file(&args.config)
        .expect("Failed to load config file");

    let mut cnc = CNC::new(&args.algorithm, network);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}
