cargo build --release || { echo "編譯失敗" ; exit 1; }
./target/release/adams_leaf $1 exp_graph.json exp_flow_mid.json exp_flow_light.json $2
