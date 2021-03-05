.PHONY: start clean


PARALLEL := parallel -j2 --bar --tag --lb
CARGO := cargo
MAIN := target/release/adams_leaf

LOG  = $(wildcard plot/log/*.log)
DAT  = plot/fig-5-1.dat plot/fig-5-2.dat plot/fig-5-3.dat
PNG  = $(DAT:%.dat=%.png)
LOCK = .plot.lock

FOLD   := 1 2 3 4 5 6 7
MEMORY := 1 2 3 4 5 6 7


all: check

start: $(PNG)

clean:
	$(RM) $(LOG) $(DAT) $(PNG) $(LOCK)

profile: $(MAIN)
	time cargo run --release -- ro exp_graph.json exp_flow_heavy.json exp_flow_reconf.json 2
	cloc src

check: $(MAIN)
	cargo test --test integration_test -- --show-output --test-threads=1 > tests/result.log
	diff -I time -I finished --color tests/expect.log tests/result.log

SOURCES := $(shell find src/ -type f -name '*.rs')

$(MAIN): $(SOURCES)
	cargo build --release

plot/%.png: plot/%.gpi plot/%.dat
	gnuplot $< > $@

plot/fig-5-1.dat: $(LOCK)
	(seq -s ' ' 10 10 70; \
	 utils/stat.sh $(foreach f,$(FOLD),plot/log/spf-mid-$(f)-3.log);   \
	 utils/stat.sh $(foreach f,$(FOLD),plot/log/ro-mid-$(f)-3.log);    \
	 utils/stat.sh $(foreach f,$(FOLD),plot/log/aco-mid-$(f)-3.log);   \
	 utils/stat.sh $(foreach f,$(FOLD),plot/log/aco-mid-$(f)-inf.log); \
	)| datamash -W transpose | column -t > $@

plot/fig-5-2.dat: $(LOCK)
	(seq -s ' ' 10 10 70; \
	 utils/stat.sh $(foreach f,$(FOLD),plot/log/spf-heavy-$(f)-3.log);   \
	 utils/stat.sh $(foreach f,$(FOLD),plot/log/ro-heavy-$(f)-3.log);    \
	 utils/stat.sh $(foreach f,$(FOLD),plot/log/aco-heavy-$(f)-3.log);   \
	 utils/stat.sh $(foreach f,$(FOLD),plot/log/aco-heavy-$(f)-inf.log); \
	)| datamash -W transpose | column -t > $@

plot/fig-5-3.dat: $(LOCK)
	(seq -s ' ' 1 7; \
	 utils/stat.sh $(foreach m,$(MEMORY),plot/log/aco-heavy-4-$(m).log); \
	)| datamash -W transpose | column -t > $@


$(LOCK): $(MAIN)
	mkdir -p plot/log/
	# for figure 5.1 and 5.2
	$(PARALLEL) $(MAIN) {1} \
		exp_graph.json exp_flow_{2}.json exp_flow_reconf.json \
		{3} --config=assets/confs/config.{4}.json \
		'>' plot/log/{1}-{2}-{3}-{4}.log \
		::: spf aco ro ::: mid heavy ::: $(FOLD) ::: 3 inf
	# for figure 5.3
	$(PARALLEL) $(MAIN) {1} \
		exp_graph.json exp_flow_{2}.json exp_flow_reconf.json \
		{3} --config=assets/confs/config.{4}.json \
		'>' plot/log/{1}-{2}-{3}-{4}.log \
		::: aco ::: heavy ::: 4 ::: $(MEMORY)
	touch $@
