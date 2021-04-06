use adams_leaf::utils::json;
use adams_leaf::utils::yaml;
use adams_leaf::cnc::CNC;
use adams_leaf::utils::config::Arguments;

fn main() {
    let args: Arguments = argh::from_env();

    let network = json::load_network(&args.network);
    let (tsns1, avbs1) = json::load_streams(&args.backgrounds, 1);
    let (tsns2, avbs2) = json::load_streams(&args.inputs, args.fold);

    let mut config = yaml::load_config(&args.config);
    config.override_from_args(args);

    let mut cnc = CNC::new(network, config);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}
