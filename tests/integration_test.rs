use adams_leaf::cnc::CNC;
use adams_leaf::utils::yaml;

#[test]
fn it_runs_aco() {
    let (tsns1, avbs1) = yaml::load_streams("data/streams/motiv-heavy.yaml", 1);
    let (tsns2, avbs2) = yaml::load_streams("data/streams/motiv-reconf.yaml", 2);
    let network = yaml::load_network("data/network/typical.yaml");

    let mut config = yaml::load_config("data/config/default.yaml");
    config.algorithm = String::from("aco");
    config.seed = 42;

    let mut cnc = CNC::new(network, config);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}

#[test]
fn it_runs_ro() {
    let (tsns1, avbs1) = yaml::load_streams("data/streams/motiv-heavy.yaml", 1);
    let (tsns2, avbs2) = yaml::load_streams("data/streams/motiv-reconf.yaml", 2);
    let network = yaml::load_network("data/network/typical.yaml");

    let mut config = yaml::load_config("data/config/default.yaml");
    config.algorithm = String::from("ro");
    config.seed = 420;

    let mut cnc = CNC::new(network, config);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}

#[test]
fn it_runs_spf() {
    let (tsns1, avbs1) = yaml::load_streams("data/streams/motiv-heavy.yaml", 1);
    let (tsns2, avbs2) = yaml::load_streams("data/streams/motiv-reconf.yaml", 2);
    let network = yaml::load_network("data/network/typical.yaml");

    let mut config = yaml::load_config("data/config/default.yaml");
    config.algorithm = String::from("spf");

    let mut cnc = CNC::new(network, config);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}
