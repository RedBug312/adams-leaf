LOG = $(shell find log -type f)
DAT = fig-5-1.dat fig-5-2.dat fig-5-3.dat
PNG = $(DAT:%.dat=%.png)
LOCK = .parallel.lock

FOLD   := $(shell seq 7)
MEMORY := $(shell seq 7)


.PHONY: all
all: $(PNG)

.PHONY: clean
clean:
	$(RM) $(LOG) $(DAT) $(PNG)


%.png: %.gpi %.dat
	gnuplot $< > $@

LOG1 += $(foreach f, $(FOLD),log/spf-mid-$(f)-3.log)
LOG1 += $(foreach f, $(FOLD),log/ro-mid-$(f)-3.log)
LOG1 += $(foreach f, $(FOLD),log/aco-mid-$(f)-3.log)
LOG1 += $(foreach f, $(FOLD),log/aco-mid-$(f)-inf.log)

fig-5-1.dat: $(LOCK)
	(seq 10 10 70; ./filter-time.sh $(LOG1)) \
	| paste - - - - - - - \
	| datamash transpose \
	| column -t > $@

LOG2 += $(foreach f, $(FOLD),log/spf-heavy-$(f)-3.log)
LOG2 += $(foreach f, $(FOLD),log/ro-heavy-$(f)-3.log)
LOG2 += $(foreach f, $(FOLD),log/aco-heavy-$(f)-3.log)
LOG2 += $(foreach f, $(FOLD),log/aco-heavy-$(f)-inf.log)

fig-5-2.dat: $(LOCK)
	(seq 10 10 70; ./filter-time.sh $(LOG2)) \
	| paste - - - - - - - \
	| datamash transpose \
	| column -t > $@

LOG3 += $(foreach m, $(MEMORY),log/aco-heavy-4-$(m).log)

fig-5-3.dat: $(LOCK)
	(seq 1 1 7; ./filter-time.sh $(LOG3)) \
	| paste - - - - - - - \
	| datamash transpose \
	| column -t > $@

.PHONY:test
test:
	seq 56 | paste $(foreach i,$(shell seq 14),-)


.INTERMEDIATE: $(LOCK)
$(LOCK): $(OUT)
	mkdir -p log
	# use GNU parallel for progress bar
	parallel --bar --tag --lb --colsep '-' $(OUT) {1} \
		../exp_graph.json ../exp_flow_{2}.json ../exp_flow_reconf.json \
		{3} --config=../assets/confs/config.{4}.json \
		'>' log/{1}-{2}-{3}-{4}.log ::: \
		$(LOG1:log/%.log=%) $(LOG2:log/%.log=%) $(LOG3:log/%.log=%)
	touch $@
