SIM := cargo run -q --release -p weft-sim --
PY  := PYTHONPATH=python python3

.PHONY: build check run sweep demo test clean

# Trace the Python example into IR: build/ota.weft.json and build/ota_buggy.weft.json
build:
	$(PY) -m weft build examples/ota.py -o build

check: build
	$(SIM) check build/ota.weft.json

run: build
	$(SIM) run build/ota.weft.json --seed 1

sweep: build
	$(SIM) sweep build/ota.weft.json --seeds 500

# The whole story: correct version passes 500 seeds; the one-line bug is caught.
demo: build
	@echo "\n=== check: validate and print the IR ==="
	$(SIM) check build/ota.weft.json
	@echo "\n=== run one seed ==="
	$(SIM) run build/ota.weft.json --seed 1
	@echo "\n=== sweep 500 seeds: correct version ==="
	$(SIM) sweep build/ota.weft.json --seeds 500
	@echo "\n=== sweep 500 seeds: buggy version (expected to fail) ==="
	$(SIM) sweep build/ota_buggy.weft.json --seeds 500 || true

test:
	cargo test -q
	$(PY) -m unittest discover -s python/tests -q

clean:
	rm -rf build target
