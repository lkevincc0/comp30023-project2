# ============================================================================
# Submodule Architecture Auto-Detection
# Automatically selects the correct precompiled libraries from project2-bin
# based on the current system architecture (arm64, x86_64, etc.)
# ============================================================================
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

# Default to the Linux x86_64 libraries
LIBENGINE=project2-bin/libengine.a
LIBRUNNER=project2-bin/librunner.a

# Override for macOS ARM64 (Apple Silicon M1/M2/M3)
ifeq ($(UNAME_S),Darwin)
ifeq ($(UNAME_M),arm64)
LIBENGINE=project2-bin/arm64/libengine.a
LIBRUNNER=project2-bin/arm64/librunner.a
endif
endif

CC=cc
CFLAGS=
LDFLAGS=

RUSTC=rustc
RUSTFLAGS=-O -C codegen-units=1 -C panic=abort

all: client.a server.a

# Create executables by linking the static libraries
client: $(LIBRUNNER) client.a
	$(CC) -Wl,--allow-multiple-definition $(CFLAGS) -o $@ $^ $(LDFLAGS)

server: server.a $(LIBENGINE)
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

# Create static libraries from .rs files
client.a: client_rs/src/*.rs
	$(RUSTC) $(RUSTFLAGS) --crate-type=staticlib -o client.a client_rs/src/lib.rs

server.a: server_rs/src/*.rs
	$(RUSTC) $(RUSTFLAGS) --emit=obj -o server_rs/server.o server_rs/src/main.rs
	rm -f $@
	ar rcs $@ server_rs/server.o

clean:
	rm -f client server client.a server.a server_rs/server.o

.PHONY: format
format:
	cd client_rs && cargo fmt
	cd server_rs && cargo fmt

.PHONY: test
test: server
	tests/task1.sh
