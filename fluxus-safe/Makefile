# e2e test
ref:
	export RUST_BACKTRACE=full
	./build.sh
	cargo test --test general -- --nocapture

jumbo:
	export RUST_BACKTRACE=full
	./build.sh
	cargo test --test jumbo_general -- --nocapture

unit_test:
	export RUST_BACKTRACE=full
	./build.sh
	cargo test --lib -- --nocapture