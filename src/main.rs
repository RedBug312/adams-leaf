use adams_leaf::utils::config::Args;
use adams_leaf::utils::yaml;
use adams_leaf::cnc::CNC;
use docopt::Docopt;

const USAGE: &'static str = "
Usage: adams_leaf [options] <network> <backgrounds> <inputs> <fold>
       adams_leaf (--help | --version)

Options:
    -h, --help            Display this message
    -c, --config PATH     Configure CNC algorithm and parameters
    -a, --algorithm TYPE  Override algorithm used to calculate routing set
    -m, --memory NUM      Override memory parameters for ACO algorithm
    -s, --seed NUM        Override random seed for ACO or RO algorithm
";

fn main() {
    let argv = std::env::args();
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.argv(argv).deserialize())
        .unwrap_or_else(|e| e.exit());
    println!("{:?}", args);

    let network = yaml::load_network(&args.arg_network);
    let (tsns1, avbs1) = yaml::load_streams(&args.arg_backgrounds, 1);
    let (tsns2, avbs2) = yaml::load_streams(&args.arg_inputs, args.arg_fold);

    let path = args.flag_config.clone()
        .unwrap_or(String::from("data/config/default.yaml"));
    let mut config = yaml::load_config(&path);
    config.override_from_args(args);

    let mut cnc = CNC::new(network, config);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}
