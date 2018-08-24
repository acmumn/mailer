all: check doc build-debug test
clean:
	cargo clean
watch TARGET="all":
	watchexec -cre rs,toml "just {{TARGET}}"

bench:
	cargo +nightly bench --all
build: build-debug build-release
build-debug:
	cargo build --all
build-release:
	cargo build --all --release
check:
	cargo check --all
clippy:
	cargo +nightly clippy --all
doc:
	cargo doc --all
test:
	cargo test --all

build-docker:
	@echo TODO; exit 1
outdated-deps:
	cargo outdated -R

run +ARGS="":
	cargo run -- {{ARGS}}

reset-database:
	@echo "         ABOUT TO NUKE MYSQL"
	@echo "IF YOU'RE NOT SURE WHERE, ASSUME PROD"
	@echo "HIT ^C IN THE NEXT 5 SECONDS TO CANCEL"
	@sleep 5
	diesel database reset
update-schema: reset-database
	diesel print-schema > src/db/schema.rs
