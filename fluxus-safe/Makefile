# e2e test
test:
	export RUST_BACKTRACE=full
	./build.sh
	cargo test --test '*' -- --nocapture

unit_test:
	export RUST_BACKTRACE=full
	./build.sh
	cargo test --lib -- --nocapture