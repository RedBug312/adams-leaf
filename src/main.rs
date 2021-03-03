use adams_leaf::network_wrapper::RoutingCost;
use adams_leaf::routing_algos::{AdamsAnt, RoutingAlgo, RO, SPF};
use adams_leaf::{config::Config, read_flows_from_file, read_topo_from_file};
use regex::Regex;
use std::env;

fn main() -> Result<(), String> {
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
            return Err("用法： adams_leaf [algo type] [topo.json] [base_flow.json] [reconf_flow.json] [倍數] (--config=[設定檔])".to_owned());
        }
    };
    if let Some(config_name) = config_name {
        println!("{}", config_name);
        Config::load_file(&config_name).unwrap();
    }

    let (tsns1, avbs1) = read_flows_from_file(&flow_file_name, 1);
    let (tsns2, avbs2) = read_flows_from_file(&flow_file_name2, times);
    let g = read_topo_from_file(&topo_file_name);
    // FIXME 對這個圖作 Yens algo，0->2這條路有時找得到6條，有時只找得到5條

    let mut cost_list = Vec::<RoutingCost>::new();
    let mut sum_comp_time = 0;
    for _ in 0..1 {
        let mut algo: Box<dyn RoutingAlgo> = {
            if algo_type == "aco" {
                Box::new(AdamsAnt::new(g.clone()))
            } else if algo_type == "ro" {
                Box::new(RO::new(g.clone()))
            } else if algo_type == "spf" {
                Box::new(SPF::new(g.clone()))
            } else {
                panic!("{} 是啥鬼= =", algo_type);
            }
        };
        algo.add_flows(tsns1.clone(), avbs1.clone());
        #[cfg(not(feature = "batch-eval"))]
        {
            println!("=== round 1 ===");
            algo.show_results();
        }
        algo.add_flows(tsns2.clone(), avbs2.clone());
        #[cfg(not(feature = "batch-eval"))]
        {
            println!("=== round 2 ===");
            algo.show_results();
            println!(
                "--- compute time: {} micro sec ---",
                algo.get_last_compute_time()
            );
        }
        cost_list.push(algo.get_cost());
        sum_comp_time += algo.get_last_compute_time();
    }
    RoutingCost::show_brief(cost_list);
    println!(
        "avg computing time: {} microsecond",
        sum_comp_time as f64 / Config::get().exp_times as f64
    );
    Ok(())
}
