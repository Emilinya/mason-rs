VERBOSE ?=
RELEASE ?=

SILENCE = @
ifeq ($(VERBOSE), 1)
	SILENCE =
endif

TARGET = debug
CARGO_OPTS ?=
CLIPPY_OPTS ?=
ifeq ($(RELEASE), 1)
	override CARGO_OPTS += --release
	TARGET = release
endif

.PHONY: build
build:
	$(SILENCE)cargo build $(CARGO_OPTS)

.PHONY: format
format:
	$(SILENCE)cargo fmt

.PHONY: check
check:
	$(SILENCE)cargo test

.PHONY: lint
lint:
	$(SILENCE)cargo clippy -- -W clippy::use_self $(CLIPPY_OPTS)

.PHONY: coverage
coverage:
	$(SILENCE)cargo +nightly llvm-cov --doctests --open

.PHONY: ci
ci:
	$(SILENCE)make format
	$(SILENCE)make build
	$(SILENCE)make lint CLIPPY_OPTS="-W clippy::print_stderr -D warnings"
	$(SILENCE)make check
