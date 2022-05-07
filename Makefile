test:
	export RUST_BACKTRACE=1
	./build.sh
	cargo test -- --nocapture

test_with:
	export RUST_BACKTRACE=1
	./build.sh
	cargo test tests::$(w) -- --nocapture

