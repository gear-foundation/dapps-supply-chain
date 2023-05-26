.PHONY: all build fmt init lint pre-commit test full-test deps

all: init build test

build:
	@echo ⚙️ Building a release...
	@cargo b -r --workspace --exclude deploy
	@ls -l target/wasm32-unknown-unknown/release/*.wasm

fmt:
	@echo ⚙️ Checking a format...
	@cargo fmt --all --check

init:
	@echo ⚙️ Installing a toolchain \& a target...
	@rustup show

lint:
	@echo ⚙️ Running the linter...
	@cargo clippy --workspace --all-targets -- -D warnings

pre-commit: fmt lint full-test

SFT_VERSION = 2.1.2
NFT_VERSION = 0.2.11

deps:
	@mkdir -p target
	@echo ⚙️ Downloading dependencies...
	@path=target/ft-main.wasm;\
	if [ ! -f $$path ]; then\
	    curl -L\
	        https://github.com/gear-dapps/sharded-fungible-token/releases/download/$(SFT_VERSION)/ft_main.opt.wasm\
	        -o $$path;\
	fi
	@path=target/ft-logic.wasm;\
	if [ ! -f $$path ]; then\
	    curl -L\
	        https://github.com/gear-dapps/sharded-fungible-token/releases/download/$(SFT_VERSION)/ft_logic.opt.wasm\
	        -o $$path;\
	fi
	@path=target/ft-storage.wasm;\
	if [ ! -f $$path ]; then\
	    curl -L\
	        https://github.com/gear-dapps/sharded-fungible-token/releases/download/$(SFT_VERSION)/ft_storage.opt.wasm\
	        -o $$path;\
	fi
	@path=target/nft-state.wasm;\
	if [ ! -f $$path ]; then\
	    curl -L\
	        https://github.com/gear-dapps/non-fungible-token/releases/download/$(NFT_VERSION)/nft_state.meta.wasm\
	        -o $$path;\
	fi
	@path=target/nft.wasm;\
	if [ ! -f $$path ]; then\
	    curl -L\
	        https://github.com/gear-dapps/non-fungible-token/releases/download/$(NFT_VERSION)/nft.opt.wasm\
	        -o $$path;\
	fi

test: deps
	@echo ⚙️ Running unit tests...
	@cargo t

full-test: deps
	@echo ⚙️ Running all tests...
	@cargo t -- --include-ignored
