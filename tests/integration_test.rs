use adams_leaf::cnc::CNC;
use adams_leaf::utils::json;

#[test]
fn it_runs_aco() {
    let (tsns1, avbs1) = json::load_streams("exp_flow_heavy.json", 1);
    let (tsns2, avbs2) = json::load_streams("exp_flow_reconf.json", 2);
    let network = json::load_network("exp_graph.json");

    let mut cnc = CNC::new("aco", network);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}

#[test]
fn it_runs_ro() {
    let (tsns1, avbs1) = json::load_streams("exp_flow_heavy.json", 1);
    let (tsns2, avbs2) = json::load_streams("exp_flow_reconf.json", 2);
    let network = json::load_network("exp_graph.json");

    let mut cnc = CNC::new("ro", network);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}

#[test]
fn it_runs_spf() {
    let (tsns1, avbs1) = json::load_streams("exp_flow_heavy.json", 1);
    let (tsns2, avbs2) = json::load_streams("exp_flow_reconf.json", 2);
    let network = json::load_network("exp_graph.json");

    let mut cnc = CNC::new("spf", network);

    cnc.add_streams(tsns1, avbs1);
    let elapsed = cnc.configure();
    println!("--- #1 elapsed time: {} μs ---", elapsed);

    cnc.add_streams(tsns2, avbs2);
    let elapsed = cnc.configure();
    println!("--- #2 elapsed time: {} μs ---", elapsed);
}
