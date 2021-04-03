use adams_leaf::utils::json;
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
    /// random seed for heuristic-based routing algorithm
    #[argh(option, short='s', default="0")]
    seed: u64,
}

fn main() {
    let args: Arguments = argh::from_env();

    let network = json::load_network(&args.network);
    let (tsns1, avbs1) = json::load_streams(&args.backgrounds, 1);
    let (tsns2, avbs2) = json::load_streams(&args.inputs, args.fold);
    let config = json::load_config(&args.config);

    let mut cnc = CNC::new(&args.algorithm, network, args.seed, config);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}
