# COMP30023 Computer Systems Project 2: Battleship Game

**Released:** Friday, 1st May, 2026  
**Due:** Sunday, 24th May, 2026 at 11:59pm  
**Weight:** 15% of the final mark

## 1 Project Overview

In this project, you will implement a client-server system for a two-player Battleship game. Clients connect to a server, place ships and take turns making moves until one player wins.

A game engine is provided, which implements the core game logic (e.g., validating ship placements, processing moves, and determining game outcomes).

You are required to implement two components: the server and an implementation of the client API. The server accepts and manages client connections, and invokes the game engine to process game actions and maintain game state. The client API, as defined by `client.h`, provides the interface for communicating with the server.

The server and client API must interoperate using a communication protocol of your design.

## 2 Game Rules

This project implements a two-player variant of the Battleship game. You do not need to implement the game rules, as this is provided by the game engine. This summary is intended to help you design the protocol for client and server to communicate.

**Board**  
Each player has a 10×10 board on which their ships are placed. Columns are labelled A to J, and rows are labelled 1 to 10. Coordinates are specified using a column followed by a row (e.g., A1, J10), with A1 corresponding to the top-left corner of the board.

Each player's board stores the positions of their ships. In addition, each player has to guess where their opponent's ships are. They maintain a record of the moves they have made against the opponent, including the coordinates attacked and the outcomes of those moves.

**Ship Placement**  
Each player must place exactly 4 ships on their board; one ship each of lengths 2, 3, 4, and 5.

Each ship is defined by a starting coordinate, a length, and a direction (either horizontal or vertical). From its starting coordinate, the ship extends either to the right (for horizontal placement) or downwards (for vertical placement). All ships must lie entirely within the bounds of the board and must not overlap with each other.

**Gameplay**  
Players take turns making moves. On a player's turn, they select a coordinate to attack on the opponent's board. Each move results in one of the following outcomes:

- miss: the coordinate does not contain a ship,
- hit: the coordinate contains part of a ship,
- sunk: the move results in all squares forming a particular ship having been hit, or
- win: the move sinks the final remaining ship of the opponent.

The game ends when one player achieves a win by sinking all of the opponent's ships.

## 3 Game Lifecycle

A game proceeds through the following phases:

### 1. Connection establishment

Clients connect to the server and join a game identified by a game identifier. The server waits until two clients have joined the same game before proceeding.

### 2. Ship placement

Each client submits their ship placements. The server validates these placements using the game engine. The game only proceeds once both players have successfully placed their ships.

### 3. Gameplay

The server enters a turn-based loop in which players alternate moves. The active player submits a move, which is processed using the game engine, while the other player waits to receive the opponent's move and its outcome. This continues until a win condition is reached, at which point the game ends.

## 4 Provided Libraries

### 4.1 Game Engine

We provide you with an implementation of a game engine, as defined by `engine.h`. The server must use the provided game engine.

The game engine exposes the following functions:

- `engine_init()`  
  Initialises the game engine and returns a pointer to the active engine instance. Returns `NULL` on failure. Only one engine instance may be active at a time.

- `engine_init_game(engine, game_id)`  
  Initialises a new game with the given game ID. Returns `true` on success and `false` on failure (e.g., if a game with the same ID is already initialised).

- `engine_place_ships(engine, game_id, ships)`  
  Places a player's ships in the specified game. The first call for a given game assigns ships to Player 1, and the second call assigns ships to Player 2. The `ships` parameter is an array of four ships. Returns the player number (1 or 2) on success, or -1 on failure.

- `engine_take_turn(engine, game_id, player, coordinate)`  
  Processes a move made by the specified player at the given coordinate. Returns `TurnResult`, an enum value representing the result of the move (e.g., hit, miss, win, sunk), or `Invalid` if the move is not valid.

- `engine_take_turn_extended(engine, game_id, player, coordinate)`  
  Extended version of `engine_take_turn` that returns the move result in a different format. The returned `ExtendedTurnResult` contains a pointer to a data buffer and its length in bytes. The `TurnResult` can be extracted from this data with `engine_extract_turn_result`. The caller is responsible for freeing this data using `engine_free_extended_result`.

- `engine_end_game(engine, game_id)`  
  Ends the specified game. If the game has not been initialised, this operation has no effect.

- `engine_free(engine)`  
  Free the engine instance and all associated resources.

### 4.2 Client API

For the server, you write the main code and call the engine through the engine interface. This time, it is your turn to implement all of the functions declared in the client interface, `client.h`.

These functions define the communication between the client and server. During testing, we will call your implementation of the client API directly. Your implementation must therefore correctly implement the required behaviour and communicate with the server according to your protocol.

**Note:** You are not required to implement a client application, but you are required to implement the client API, which will be used by our testing code.

Specifically, you must implement the following functions:

- `client_init()`  
  Initialises the client-side implementation and returns a pointer to a client state data structure (defined by the implementation). Returns a non-NULL pointer on success and `NULL` on failure. The pointer will be passed as the `client` parameter in subsequent calls.

- `client_connect(client, addr, port, game_id)`  
  Connects to the game server at the specified hostname/address and port, and attempts to join the specified game ID. Returns `true` on successful connection and `false` otherwise.

- `client_wait_for_opponent(client)`  
  Blocks until two players have joined the game. This function returns only when the game is ready to proceed to the ship placement phase. Returns `true` when both players have joined and `false` on failure.

- `client_send_ships(client, ships)`  
  Sends the player's ship placements to the server. The `ships` parameter is an array of four ships. Returns the player number (1 or 2) assigned by the server on success, or -1 on failure.

- `client_send_move(client, coordinate)`  
  Sends a move to the server for processing. Returns `TurnResult`, the result of the move (e.g., Hit, Miss, Sunk, Win), or `Invalid` on failure.

- `client_send_move_extended(client, coordinate)`  
  Sends a move to the server and requests an extended result. Returns an `ExtendedTurnResult` containing a pointer to data and its length in bytes. The caller is responsible for freeing this result using `client_free_extended_result`.

- `client_receive_move(client)`  
  Receives the opponent's move from the server. Returns a `MoveResult` structure containing the coordinate of the move and its outcome. The caller is responsible for freeing this result using `client_free_move_result`.

- `client_close(client)`  
  Disconnects from the server and frees all resources associated with the client state.

## 5 Implementation

In your submission, include a `Makefile` that produces the following:

For the server, produce a static library `server.a` which contains a `main` function and your server logic. This will be linked with our implementation of `engine.h` to produce a server executable.

The server must accept a single mandatory positional command-line argument `port`:

```
Usage: ./server <port>
```

The server is expected to listen for incoming connection requests on the port passed via command line arguments, on any of the host's IPv4 network addresses.

You will be provided with some sample code showing usage of the engine.

The server component should not shut down by itself. `SIGTERM` will be used to terminate the server between test cases.

You may notice that a port and interface which has been bound to a socket sometimes cannot be reused until after a timeout. To make your testing and our marking easier, please override this behaviour by placing the following lines before the `bind()` call:

```c
int enable = 1;
if (setsockopt(sockfd, SOL_SOCKET, SO_REUSEADDR, &enable, sizeof(int)) < 0) {
    perror("setsockopt");
    exit(3);
}
```

For the client, produce a static library `client.a` which implements the client API (`client.h`). This will be linked with our implementation of a client runner, producing a client executable. The client runner is an interface for players. It will run through the game phases, requesting player input and calling your client implementation as necessary. Run two instances of the client executable against your server to play a game.

The command-line interface for the client runner program is:

```
Usage: ./client [-a <host>] [-p <port>] [-s <game-id>] [-verbose]
```

You will be provided a skeleton implementation of `client.h`.

The client implementation must be single-threaded. Ensure that multiple instances of the client implementation can be initialised and used in a single process.

Ensure that your client implementation does not write to `stdout`. However, the runner can. If you want to output debugging information, use `stderr`.

The engine (present in both client and server programs) supports some environment variables:

- `ENGINE_DEBUG`: When set to `1`, debug logs to `stderr` will be enabled.
- `ENGINE_MODE`: Sets the engine mode. (Used by Task 4 and Task 6)

**Communication Protocol**  
The communication protocol between the client API implementation and the server is not prescribed. One of the main features of this project is that you are responsible for designing and implementing an application layer protocol that allows your client API and server to communicate correctly. The protocol must be implemented over TCP.

### 5.1 Task 1: Connection Establishment

The server initialises the game engine, listens for incoming client connections, and accepts them. Once a client connects, the server initialises the game with game identifier 0. The server must wait until two clients have joined the same game before allowing the game to proceed.

On the client side, the client is initialised with `client_init`. `client_connect` is then called to establish a connection to the server using the specified host, port, and game identifier of 0. After connecting, `client_wait_for_opponent` must block until another client joins the same game. This function should only return once the server indicates that both players are present and the game is ready to begin.

### 5.2 Task 2: Ship Placement

In the ship placement phase, each client submits a ship configuration to the server using `client_send_ships`. This function must transmit a set of ships to the server and return the player number assigned to the client.

Upon receiving ship configurations from both players, the server must pass this information to the game engine for validation and registration. The server must assign player identities based on the order in which client connections were accepted. The player who connected to the server first is Player 1, and the second corresponds to Player 2.

The server must communicate the assigned player number back to each client. The value returned by `client_send_ships` must reflect this assignment. The game must not proceed until both players have successfully submitted valid ship configurations.

### 5.3 Task 3: Gameplay

In the gameplay phase, the two players alternate turns until one of them wins. At any point in time, one player is active and the other is waiting for the result of the opponent's move. Player 1 takes the first turn.

When it is a player's turn, `client_send_move` must send a move to the server and return the result of that move. The server processes the move using the game engine and returns the outcome to the same client. When it is not the player's turn, `client_receive_move` must block until the opponent's move is received. The server must send both the coordinate of the opponent's move and its outcome, which are returned by this function.

After each move, the turn passes to the other player. This process repeats until the engine reports a win, at which point the game is complete. The outcome should be returned to both clients.

`client_close` will be called to clean up the resources associated with the current client instance. This includes disconnecting from the server. After a game completes, a new game with the same game identifier may be initialised through `engine_init_game`.

### 5.4 Task 4: Extended Move Encoding

In previous tasks, it can be assumed that the game identifier (an unsigned 32-bit integer) will always be 0. In this task, the client must now send the game identifier to the server, and initialise the engine with the same value. The game identifier must be sent in plain text and network-byte order. Note that the testing system will check this by snooping on packets.

Additionally, when a move is submitted using `client_send_move_extended`, the server must process the move using `engine_take_turn_extended` instead of `engine_take_turn`. This function returns a variable-length result consisting of a length field and a pointer to a block of data of that exact length. The length can be any unsigned 16-bit integer value. The payload must be treated as opaque binary data and transmitted unchanged.

The server must transmit both the length and the payload data back to the client. The client API will then return this result, which is then interpreted by the caller. Use `engine_extract_turn_result` to extract the `TurnResult`, and send it alongside the coordinate to the opponent.

Note that this task focuses on the correct handling of variable-length binary data in the protocol, rather than on changes to the underlying game logic.

### 5.5 Task 5: Multiple Concurrent Games

In this task, the server must support multiple games running simultaneously. Clients may connect to different games by specifying a game identifier when calling `client_connect`. The server must ensure that clients are matched only with other clients that specify the same game identifier.

Each game must maintain independent state, including connected clients, ship placements, turn order, and progress through the game phases. All communication must be scoped to the corresponding game. In particular, calls such as `client_send_ships`, `client_send_move`, `client_send_move_extended`, and `client_receive_move` must only affect and receive data from the game associated with that client.

The server must correctly route all messages based on the game identifier and ensure that operations for one game do not interfere with any other game. Progress in one game must not block or delay progress in other games. The server should be able to support at least 20 concurrent clients.

To receive full marks, a malicious client that just connects and hangs (i.e., doesn't send a game identifier) should not block subsequent clients.

There are several ways to implement this. One possible approach is to use multiple threads, for example by assigning each game its own thread. You may also need to incorporate an event-driven design (e.g., using `select(2)`, `epoll(7)`). Do not create multiple processes (e.g., with `fork(2)`).

**Note:** The provided game engine is not thread-safe. Concurrent access to the active engine instance is not permitted and must be synchronised appropriately.

If you implement this task, include the string `MULTIPLE_GAMES` in your source code. Otherwise, this functionality will not be tested/marked.

### 5.6 Task 6: Error Handling

Network code should never crash, even if the hosts on the other side behave poorly.

#### 5.6.1 Server

The server implementation returns errors to the client on failure and does not crash after `listen(2)`. For the purpose of assessment, follow these predefined actions:

- Port was not provided - Exit status 1, or optionally continue with default port of 30023
- Failed to initialise engine - Exit status 2
- Socket related failures up to and including `listen(2)` - Exit status 3
- Other errors before call to `listen(2)` - Exit status 5
- `accept(2)` fails - Ignore and proceed to next connection, optionally exit when appropriate with exit status 4
- A client requests to join a game which already has 2 players - Return failure to client
- Player disconnects (while in game) - End game, notify and disconnect both players
- Engine indicates failure for init, ship placement, or take turn - Return failure to both players
- Received malformed data from player in game - End game, notify and disconnect both players

The engine will validate value of arguments which are passed to it. For example, your server does not need to parse or reject invalid coordinate strings, like "Z13". However, you must ensure that the memory is safe to access (e.g., the coordinate must be null-terminated).

When exiting due to error, ensure that `engine_free` is called.

#### 5.6.2 Client

The client implementation does not crash and returns errors through return values. Follow these predefined actions:

- The client pointer is `NULL` - Indicate failure through return value
- Cannot connect to server - Return `false`
- Server indicates error - Indicate failure through return value
- Server disconnects - Indicate failure through return value
- Received malformed data - Indicate failure through return value

You may assume that the values of ships and coordinate parameters are well-formed, validated according to game rules, and safe to access.

Functions may be called out of order. The server must correctly attribute each action to its sender.

For both the engine and client API, failure is indicated by:

- For functions returning a boolean value, return `false`
- For functions returning an `int` value, return `-1`
- For `TurnResult`, return enumeration value of `Invalid`
- For `MoveResult`, return result of `Invalid`, coordinate may be `NULL`
- For `ExtendedTurnResult`, return length of `0` and set data to `NULL`

#### 5.6.3 Other considerations

Both the client and server should abort on memory allocation errors. You do not need to apply a timeout; if the client or server becomes unresponsive, your code is allowed to hang. If you implement timeouts (not a requirement), the timeout duration must be no shorter than 30 seconds. The server may explicitly handle `SIGTERM` and `SIGINT` to shut down gracefully (not a requirement).

## 6 Marking Criteria

The marks are broken down as follows.

| Task # and description | Marks | In CI |
|------------------------|-------|-------|
| 1. Connection establishment | 2 | 1 |
| 2. Ship placement | 2 | 1 |
| 3. Gameplay | 3 | 1.5 |
| 4. Extended move encoding | 2 | 1 |
| 5. Multiple concurrent games | 2 | 1 |
| 6. Error handling | 2 | 1 |
| 7. Build quality | 1 | 1 |
| 8. Quality of software practices | 1 | 0 |
| **Total** | **15** | **7.5** |

You will be given access to a subset of the test cases used to assess your project. The column "In CI" specifies how many of the marks for each task are covered by these visible tests and evaluated through the continuous integration (CI) system. If you consistently pass these tests in the CI, you can be confident of obtaining those marks. The remaining marks will be determined using a separate set of hidden test cases that are not available before submission.

Assessment of Tasks 1-7 will be performed through automated testing.

Code that does not compile and run on the provided VMs or Dev Containers will usually be awarded zero marks for tasks 1-7. Use the GitHub continuous integration (CI) infrastructure to ensure your submission is valid. This is very important. Nearly every year, someone gets code working on their own computer, but it fails in the marking environment. Please push your code to git regularly and check the CI output.

### Build quality

The repository must contain a `Makefile` that produces static libraries `server.a` and `client.a`, along with all source files required for compilation. Place the `Makefile` at the root of your repository, and ensure that running `make clean && make -B server.a client.a` places the `.a` files there too. Make sure that all source code is committed and pushed. Include in your `Makefile` (in 1 line) a `LDFLAGS` variable describing any linker flag(s) that your submission requires. Compiling using `-Wall` should yield no warnings. Running `make clean` should remove all compiler artifacts and executables. Do not commit `.o`, `.a` or other executable files (by having a `.gitignore` file). Scripts (with `.sh` extension) are exempted.

### Quality of software practices

Factors considered include:

- Proper use of version control, based on the regularity of commit and push events, their content and associated commit messages (e.g., repositories with a single commit and/or noninformative commit messages will lose 0.5 marks).
- Quality of code, based on the choice of variable names, comments, formatting (e.g. consistent indentation and spacing), and structure (e.g. abstraction, modularity).
- Proper memory management, based on the absence of memory errors and memory leaks. Code will be tested with `valgrind` to ensure no memory errors, as these are a security risk. When `SIGTERM` is used to terminate process, the server can have unfreed memory for engine and server state relating to active games.

Further deductions may be applied to inappropriate submissions, e.g. catching segmentation faults, hard-coding the output into the code.

## 7 Submission

**Programming language**  
All code must be written in C or Rust (e.g., it should not be a C-wrapper over non-C code). You can use a combination of both languages in your submission, e.g. C for Client, Rust for Server.

Rust submissions must be compiled with stable `rustc`, with no external crates or build scripts. `cargo` will not be present in the marking environment, but you can use it during development.

**Use of libraries**  
You can reuse the code that you wrote for other individual projects if you clearly specify when and for what purpose you have written it (e.g., the code and name of the subject, project description and date, that can be verified if needed).

You may use the code that we have provided for the practicals without attribution.

You can create a library to share code between your client and server implementation.

You may use standard libraries (e.g., to create sockets, send, receive data etc.). Your code must compile and run on the provided VMs. External libraries cannot be used.

**GitHub**  
The use of GitHub is mandatory. Your submission will be assessed using the code in your Project 2 repository (`proj2-<username>`) under the subject's organization.

We strongly encourage you to commit your code at least once per day. Be sure to push after you commit. This is important not only to maintain a backup of your code, but also because the git history and GitHub Actions history may be considered for matters such as special consideration, extensions and potential plagiarism. Proper use of git will have a positive effect on the mark you get for quality of software practices.

**Submission**  
To submit your project, please follow these steps carefully:

1. Push your code to the repository named `proj2-<username>` under the subject's organization, `https://github.com/feit-comp30023-2026`.

Executable files (that is, all files with the executable bit that are in your repository) will be removed before marking. Hence, ensure that none of your source files have the executable bit.

Ensure your code compiles and runs on the provided VMs or Dev Containers. Code that does not compile or produce correct output on VMs or containers will typically receive very low or 0 marks.

2. Submit the full 40-digit SHA1 hash of the commit you want us to mark to the Project 2 Assignment on the LMS.

You are allowed to update your chosen commit by resubmitting the LMS assignment as many times as desired. However, only the last commit hash submitted to the LMS before the deadline (or approved extension) will be marked without a late penalty.

3. Ensure that the commit that you submitted to the LMS is correct and accessible from a fresh clone of your repository. An example of how to do this is as follows:

```bash
git clone git@github.com:feit-comp30023-2026/proj2-<username> proj2
cd proj2
git checkout <commit-hash-submitted-to-lms>
```

Please be aware that we will only mark the commit submitted via the LMS. It is your responsibility to ensure that the submission is correct and corresponds to the commit you want us to mark.

**Late submissions**  
Late submissions will incur a deduction of 2 marks per day (or part thereof). For example, a submission made 1 hour after the deadline is considered to be 1 day late and carries a deduction of 2 marks.

We strongly encourage you to allow sufficient time to follow the submission process outlined above. Leaving it to the last minute usually results in a submission that is a few minutes to a few hours late, or in the submission of the incorrect commit hash. Either case leads to late penalties.

The submission date is determined solely by the date on which the LMS assignment was submitted. Forgetting to submit via the LMS or submitting the wrong commit hash will result in a late penalty that will apply regardless of the commit date.

We will not give partial marks or allow code edits for either known or hidden cases without applying a late penalty (calculated from the deadline).

**Extension policy**  
For extensions between 1-3 calendar days, you must:

1. Have an AAP or fill in FEIT's short extension declaration form before the project's deadline
2. Submit an extension request via the following form: https://forms.cloud.microsoft/r/UewSMfhXeg

For extensions of more than 3 calendar days, you must:

1. Apply for an extension via the special consideration portal before the assessment deadline
2. Receive a successful outcome for your application
3. Submit the outcome of your application via the following form: https://forms.cloud.microsoft/r/aj9TDfGnk5

## 8 Testing

You will be given access to several test cases; however, these are not exhaustive and will not cover all edge cases. Please be aware that the test suite (visible and hidden cases combined) aims to cover all of the business rules in the specification. Hidden test cases will also be more difficult. Hence, you are strongly encouraged to write tests to verify the correctness of your own implementation.

**Testing Locally:** You can clone the sample test cases to test locally, from: `feit-comp30023-2026/project2-cases`.

**Continuous Integration Testing:** To provide you with feedback on your progress before the deadline, we will set up a Continuous Integration (CI) pipeline on GitHub with the visible set of test cases.

Though you are strongly encouraged to use this service, the usage of CI is not assessed, i.e., we do not require CI tasks to be completed for a submission to be considered for marking.

The requisite `ci.yml` file has been provisioned and placed in your repository, but is also available from the `.github/workflows` directory of the skeleton repository `feit-comp30023-2026/project2`.

## 9 Collaboration and Plagiarism

The program you submit as part of this assignment must be your own work.

You cannot copy work from another student or from any other source, such as online repositories. Do not share your code, and do not ask others to give you their programs. Do not post your code on the subject's discussion board Ed. The best way to help your friends in this regard is to say a very firm "no" if they ask to see your program. See https://academicintegrity.unimelb.edu.au for more information.

Note that soliciting solutions by any means (e.g., via posts to online forums), whether or not there is payment involved, is also Academic Misconduct.

You should not post your code to any public location (e.g., in your GitHub account) before receiving the final marks for the subject. This ensures you are not in breach of the Academic Integrity Policy by enabling other students in the same semester to complete their projects. You are not allowed to post the project's specification in any public forum at any stage, as it is the University's copyright.

The use of AI tools is strictly prohibited. This project must be completed without the help of AI, with the final product expected to be your own work.

If you use a small amount of code not written by you, you must attribute that code to the source you got it from (e.g., a book or Stack Exchange).


## EXTRA INFO
```text
CI Status

|        | ETA           | Marks visible
| Task 1 | Released      | 1
| Task 2 | Released      | 1
| Task 3 | Released      | 1.5
| Task 4 | 11/May (----) | 1
| Task 5 | 11/May (----) | 1
| Task 6 | 17/May (----) | 1
| Task 7 | Released      | 1
| Task 8 | None          | 0
| Mem    | ???    (----) | 0


Skeleton/API: https://github.com/feit-comp30023-2026/project2
Cases: https://github.com/feit-comp30023-2026/project2-cases

Please report any unexpected library or CI failures on Ed. 

Recommendations

Do not leave this project to the last minute. 
It involves new APIs (e.g. socket programming). It's not a pure algorithms/data structures assignment, which you may have been used to from COMP10002, COMP20003/COMP20007. 
Read through the full requirements before commencing and design as such of your solution upfront, to avoid unnecessary refactors.
It is better to finish one task than to have two tasks partly done. That is because marks are only allocated to tasks that actually produce the correct results.
A slight warning: Implementing Task 5 correctly and properly (using proper concurrency primitives instead of hacks) is non-trivial. Please be aware that there may be significant effort required to obtain full marks if you're not familiar with pthreads/Rust concurrency primitives. 
Testing

Set ENGINE_DEBUG=1 (server) or --verbose (client) to see debug logs from engine
Use Wireshark and/or tcpdump to debug requests/responses.
Resources

C

Beej's Guide to Network Programming - https://beej.us/guide/bgnet/html/split/
Rust

Please do not commit target/!
Rust Book - https://doc.rust-lang.org/book/
cargo fmt - https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html#automatic-formatting-with-rustfmt (Please use!)
cargo clippy - https://doc.rust-lang.org/clippy/usage.html (Please use!)
Unit Tests - https://doc.rust-lang.org/book/ch11-01-writing-tests.html
Modules - https://doc.rust-lang.org/rust-by-example/mod/split.html
Specification


Implementation

Docker on Apple Silicon Mac: #310 
stderr on server #313
Cases
Last updated: #315
```