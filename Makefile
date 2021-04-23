SRC = $(shell find src -type f)
OUT = target/release/adams-leaf


.PHONY: all
all: check

.PHONY: build
build: $(OUT)

.PHONY: start
start: $(OUT)
	time make -C plot OUT=../$(OUT)

.PHONY: check
check: $(OUT)
	RUST_BACKTRACE=1 cargo test --lib
	RUST_BACKTRACE=1 cargo test --test integration_test \
		-- --show-output --test-threads=1 > tests/result.log
	diff -I time -I finished --color tests/expect.log tests/result.log
	time $(OUT) data/network/typical.yaml data/streams/scale-motiv-mid.yaml \
		data/streams/scale-motiv-reconf.yaml 6 -c data/config/finetune.yaml
	cloc src

.PHONY: clean
clean:
	make -C plot clean
	$(RM) $(OUT)


$(OUT): $(SRC)
	cargo build --release
