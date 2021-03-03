use super::*;
use crate::flow::data::TSNData;

type Info = Vec<(usize, usize)>;
type FT = FlowTable<Info>;

/**
 * 頻寬皆為100
 *  0--1
 *  |\ |\
 *  | \| \
 *  2--3  4
 *     | /
 *     |/
 *     5
 *
 * flow0(0->4): route=[0, 1, 4], links = [0, 4]
 * flow1(0->5): route=[0, 3, 5], links = [2, 6]
 * flow2(0->4): route=[0, 3, 5, 4], links = [2, 6, 7]
 * flow3(0->4): route=[0, 2, 3, 5, 4], links = [1, 5, 6, 7]
 */

fn gen_links(ids: Vec<usize>) -> Info {
    ids.into_iter().map(|id| (id, MTU)).collect()
}
fn gen_flow_table() -> FT {
    let mut ft = FlowTable::new();
    ft.insert(
        vec![
            TSNFlow {
                id: 0.into(),
                src: 0,
                dst: 4,
                size: MTU,
                period: 100,
                max_delay: 100,
                spec_data: TSNData { offset: 0 },
            },
            TSNFlow {
                id: 0.into(),
                src: 0,
                dst: 5,
                size: MTU * 3,
                period: 150,
                max_delay: 150,
                spec_data: TSNData { offset: 0 },
            },
            TSNFlow {
                id: 0.into(),
                src: 0,
                dst: 4,
                size: MTU * 2,
                period: 200,
                max_delay: 200,
                spec_data: TSNData { offset: 0 },
            },
            TSNFlow {
                id: 0.into(),
                src: 0,
                dst: 4,
                size: MTU * 3,
                period: 300,
                max_delay: 300,
                spec_data: TSNData { offset: 0 },
            },
        ],
        vec![],
        vec![],
    );
    ft.update_info(0.into(), gen_links(vec![0, 4]));
    ft.update_info(1.into(), gen_links(vec![2, 6]));
    ft.update_info(2.into(), gen_links(vec![2, 6, 7]));
    ft.update_info(3.into(), gen_links(vec![1, 5, 6, 7]));
    ft
}
fn to_links(vec: &Vec<(usize, usize)>) -> Vec<(usize, f64)> {
    vec.iter().map(|(a, b)| (*a, *b as f64)).collect()
}

#[test]
fn simple_calculate_offset() {
    let gcl = GCL::new(60, 16);
    let ft = gen_flow_table();
    let flow = ft.get_tsn(0.into()).unwrap();
    let links = to_links(ft.get_info(0.into()).unwrap());
    let a = calculate_offsets(&flow, &vec![], &links, &vec![0; 2], &gcl);
    assert_eq!(vec![0, 1], a);

    let flow = ft.get_tsn(2.into()).unwrap();
    let links = to_links(ft.get_info(2.into()).unwrap());
    let a = calculate_offsets(&flow, &vec![], &links, &vec![0; 3], &gcl);
    assert_eq!(vec![0, 1, 2], a);
}
#[test]
fn test_online_schedule() {
    let mut gcl = GCL::new(600, 16);
    let ft = gen_flow_table();

    schedule_fixed_og(&ft, &mut gcl, |_, info| to_links(info)).unwrap();
    //schedule_online(&ft, &ft, &mut gcl, |_, info| info);
    let ans: Vec<(u32, u32)> = vec![(0, 5), (150, 3), (203, 2), (300, 3), (403, 2), (450, 3)];
    assert_eq!(gcl.get_gate_events(2), &ans);
    //panic!("{:?}", gcl.get_gate_events(7));
    //panic!("{:?}", gcl.get_gate_events(6));
}
