test:
	export RUST_BACKTRACE=full
	./build.sh
	cargo test --test '*' -- --nocapture

