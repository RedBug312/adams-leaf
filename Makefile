SRC = $(shell find src -type f)
OUT = target/release/adams_leaf


.PHONY: all
all: check profile

.PHONY: build
build: $(OUT)

.PHONY: start
start: $(OUT)
	make -C plot OUT=../$(OUT)

.PHONY: check
check:
	cargo test --test integration_test -- --show-output --test-threads=1 > tests/result.log
	diff -I time -I finished --color tests/expect.log tests/result.log

.PHONY: profile
profile: $(OUT)
	time $(OUT) ro exp_graph.json exp_flow_heavy.json exp_flow_reconf.json 2
	cloc src

.PHONY: clean
clean:
	make -C plot clean
	$(RM) $(OUT)


$(OUT): $(SRC)
	cargo build --release
