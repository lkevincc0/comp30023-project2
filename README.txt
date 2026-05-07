COMP30023 Project 2 2026

How to use:
1. If using Rust, install the 1.95.0 stable unknown-linux-gnu toolchain
2. Copy the Makefile, and the directories for either C or Rust to your repo
3. Rename directories (optional)
4. Remove irrelevant rules for the other language from the Makefile
   If using Rust, rename the targets to client.a/server.a and client/server
5. Initialise the git submodule for project2-bin
   git submodule add git@github.com:feit-comp30023-2026/project2-bin.git
6. Run make -B
   This should produce server.a/client.a at the root of the repository
7. Run make client server
   C and Rust: This should produce server/client executables at the root of the repository.
   Rust: You can use cargo build and cargo run to run the executables during development.
8. Replace the skeleton code with your implementation
9. Remember to create a .gitignore file, and (optionally) set up a code formatter and linter
10. You are allowed to create a library to share between your client and server implementations

Please report any issues with the provided skeleton code or binaries on Ed.

How to test:
1. run all avaliable test: `make test`