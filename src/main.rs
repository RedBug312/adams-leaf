use adams_leaf::utils::json;
use adams_leaf::utils::config::Config;
use adams_leaf::cnc::CNC;
use regex::Regex;
use std::env;

fn main() {
    let (algo_type, topo_file_name, flow_file_name, flow_file_name2, times, config_name) = {
        let mut args: Vec<String> = env::args().collect();
        let re = Regex::new(r"--config=([^ ]+)").unwrap();
        let mut config_name: Option<String> = None;
        for i in 0..args.len() {
            if let Some(cap) = re.captures(&args[i]) {
                config_name = Some(cap[1].to_owned());
                args.remove(i);
                break;
            }
        }
        if args.len() == 6 {
            (
                args[1].clone(),
                args[2].clone(),
                args[3].clone(),
                args[4].clone(),
                args[5].parse::<usize>().unwrap(),
                config_name,
            )
        } else {
            panic!("用法： adams_leaf [algo type] [topo.json] [base_flow.json] [reconf_flow.json] [倍數] (--config=[設定檔])");
        }
    };
    if let Some(config_name) = config_name {
        println!("{}", config_name);
        Config::load_file(&config_name).unwrap();
    }

    let (tsns1, avbs1) = json::load_streams(&flow_file_name, 1);
    let (tsns2, avbs2) = json::load_streams(&flow_file_name2, times as u32);
    let network = json::load_network(&topo_file_name);
    // FIXME 對這個圖作 Yens algo，0->2這條路有時找得到6條，有時只找得到5條

    let mut cnc = CNC::new(&algo_type, network);

    cnc.add_streams(tsns1, avbs1);
    let time = cnc.configure();
    println!("--- #1 computing time: {} μs ---", time);

    cnc.add_streams(tsns2, avbs2);
    let time = cnc.configure();
    println!("--- #2 computing time: {} μs ---", time);
}
