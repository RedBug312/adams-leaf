OUT ?= ../target/release/adams_leaf
LOG  = $(shell find log -type f)
DAT  = fig-5-1.dat fig-5-2.dat fig-5-3.dat
PNG  = $(DAT:%.dat=%.png)
LOCK = .parallel.lock

FOLD   := $(shell seq 7)
MEMORY := $(shell seq 7)
SEED   := $(shell seq 20)  # only need modify here to get #seeds higher


.PHONY: all
all: $(PNG)

.PHONY: clean
clean:
	$(RM) $(LOG) $(DAT) $(PNG)


%.png: %.gpi %.dat
	gnuplot $< > $@

LOG1 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/spf-mid-$(f)-3-$(s).log))
LOG1 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/ro-mid-$(f)-3-$(s).log))
LOG1 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/aco-mid-$(f)-3-$(s).log))
LOG1 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/aco-mid-$(f)-inf-$(s).log))

fig-5-1.dat: $(LOCK)
	(seq 10 10 70; ./filter-time.sh $(LOG1) \
	| paste $(foreach i,$(SEED),-) \
	| datamash transpose \
	| datamash mean 1-28 \
	| tr -s ' \t' '\n') \
	| paste - - - - - - - \
	| datamash transpose \
	| column -t > $@

LOG2 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/spf-heavy-$(f)-3-$(s).log))
LOG2 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/ro-heavy-$(f)-3-$(s).log))
LOG2 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/aco-heavy-$(f)-3-$(s).log))
LOG2 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/aco-heavy-$(f)-inf-$(s).log))

fig-5-2.dat: $(LOCK)
	(seq 10 10 70; ./filter-time.sh $(LOG2) \
	| paste $(foreach i,$(SEED),-) \
	| datamash transpose \
	| datamash mean 1-28 \
	| tr -s ' \t' '\n') \
	| paste - - - - - - - \
	| datamash transpose \
	| column -t > $@

LOG3 += $(foreach m, $(MEMORY),$(foreach s, $(SEED),log/aco-heavy-4-$(m)-$(s).log))

fig-5-3.dat: $(LOCK)
	(seq 1 1 7; ./filter-time.sh $(LOG3) \
	| paste $(foreach i,$(SEED),-) \
	| datamash transpose \
	| datamash mean 1-7 \
	| tr -s ' \t' '\n') \
	| paste - - - - - - - \
	| datamash transpose \
	| column -t > $@

.INTERMEDIATE: $(LOCK)
$(LOCK): $(OUT)
	mkdir -p log
	$(RM) log/*.log
	# use GNU parallel for progress bar
	parallel --bar --tag --lb --colsep '-' $(OUT) \
		../exp_graph.json ../exp_flow_{2}.json ../exp_flow_reconf.json {3} \
		--config ../data/config/finetune.yaml --algorithm {1} --memory {4} --seed {5} \
		'>' log/{1}-{2}-{3}-{4}-{5}.log \
		::: $(LOG1:log/%.log=%) $(LOG2:log/%.log=%) $(LOG3:log/%.log=%)
	touch $@
