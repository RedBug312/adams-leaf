OUT ?= ../target/release/adams_leaf
LOG  = $(shell find log -type f)
DAT  = fig-5-1.dat fig-5-2.dat fig-5-3.dat
PNG  = $(DAT:%.dat=%.png)
LOCK = .parallel.lock

FOLD   := $(shell seq 7)
MEMORY := $(shell seq 7)
SEED   := $(shell seq 20)  # modify me for more random seeds


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
	(echo INPUTS ACO-3 ACO-INF RO SPF; \
	 ./summarize.sh $(LOG1) \
	 | datamash crosstab 3,1 mean 5 \
	 | tail -n+2) \
	| column -t > $@

LOG2 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/spf-heavy-$(f)-3-$(s).log))
LOG2 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/ro-heavy-$(f)-3-$(s).log))
LOG2 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/aco-heavy-$(f)-3-$(s).log))
LOG2 += $(foreach f, $(FOLD),$(foreach s, $(SEED),log/aco-heavy-$(f)-inf-$(s).log))

fig-5-2.dat: $(LOCK)
	(echo INPUTS ACO-3 ACO-INF RO SPF; \
	 ./summarize.sh $(LOG2) \
	 | datamash crosstab 3,1 mean 5 \
	 | tail -n+2) \
	| column -t > $@

LOG3 += $(foreach m, $(MEMORY),$(foreach s, $(SEED),log/aco-heavy-4-$(m)-$(s).log))

fig-5-3.dat: $(LOCK)
	(echo MEMORY ACO; \
	 ./summarize.sh $(LOG3) \
	 | datamash groupby 1 mean 5 \
	 | sed 's|^aco-||g') \
	| column -t > $@

.INTERMEDIATE: $(LOCK)
$(LOCK): $(OUT)
	mkdir -p log
	$(RM) log/*.log
	# use GNU parallel for progress bar
	parallel -j3 --bar --tag --lb --colsep '-' $(OUT) \
		../data/network/typical.yaml ../data/streams/motiv-{2}.yaml ../data/streams/motiv-reconf.yaml {3} \
		--config ../data/config/finetune.yaml --algorithm {1} --memory {4} --seed {5} \
		'>' log/{1}-{2}-{3}-{4}-{5}.log \
		::: $(LOG1:log/%.log=%) $(LOG2:log/%.log=%) $(LOG3:log/%.log=%)
	touch $@
