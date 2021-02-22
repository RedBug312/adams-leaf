cargo build --release || { echo "編譯失敗" ; exit 1; }
./target/release/adams_leaf $1 exp_graph.json exp_flow_heavy.json exp_flow_reconf.json 5
