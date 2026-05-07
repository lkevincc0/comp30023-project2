COMP30023 2026 - Project 2 Custom Test Cases

Custom test cases authored locally, in the same format as the official
test-data/ directory. See test-data/README.txt for how the format works.

Layout:
  test-data-custom/<task>/<case_name>/
    seq.txt          - sequence of actions performed by the test runner
    client1-in.txt   - input to client 1 (test mode)
    client1-out.txt  - expected output of client 1
    client2-in.txt   - (optional) input to client 2
    client2-out.txt  - (optional) expected output of client 2
