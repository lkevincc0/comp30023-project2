#!/bin/bash
# Task 1 test suite
set -e

PORT=30023
HOST=127.0.0.1
JOIN_MSG="010000000400000000"      # JOIN game_id=0
JOIN_WRONG="010000000400000001"    # JOIN game_id=1
OK_HEX="8100000000"
ERROR_HEX="8200000000"
READY_HEX="8300000000"

PASS=0
FAIL=0
FAIL_MSGS=""

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

# start & stop server
start_server() {
    ./server $PORT 2>/dev/null &
    SERVER_PID=$!
    sleep 0.3
}

# stop server
stop_server() {
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
}
trap stop_server EXIT

# Python helper: send hex bytes, print hex response
py_send() {
    python3 -c "
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(3)
try:
    s.connect(('$HOST', $PORT))
    s.sendall(bytes.fromhex('$1'))
    print(s.recv(1024).hex())
except Exception as e:
    print('ERROR:' + str(e))
finally:
    s.close()
"
}

# Python helper: send, then read up to 2 responses
py_send_recv2() {
    python3 -c "
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(3)
try:
    s.connect(('$HOST', $PORT))
    s.sendall(bytes.fromhex('$1'))
    h = s.recv(5)
    plen = int.from_bytes(h[1:5], 'big')
    print('MSG1:' + h.hex() + s.recv(plen).hex())
    s.settimeout(1)
    try:
        h2 = s.recv(5)
        plen2 = int.from_bytes(h2[1:5], 'big')
        print('MSG2:' + h2.hex() + s.recv(plen2).hex())
    except socket.timeout:
        pass
except Exception as e:
    print('ERROR:' + str(e))
finally:
    s.close()
"
}

# cargo-style test runner
run_test() {
    local name="$1"
    local result="$2"
    local detail="$3"
    printf "test %-35s" "$name "
    if [ "$result" = "ok" ]; then
        echo -e "${GREEN}ok${NC}"
        PASS=$((PASS + 1))
    else
        echo -e "${RED}FAILED${NC}"
        FAIL=$((FAIL + 1))
        FAIL_MSGS="$FAIL_MSGS\n    $name: $detail"
    fi
}

# build
echo "   Compiling task1"
make server 2>/dev/null
if [ ! -f ./server ]; then
    echo -e "${RED}error${NC}: could not compile server"
    exit 1
fi
echo ""

echo ""
echo "running 6 tests"
echo ""

# test connect1: single client sends JOIN, expects OK
start_server
RESULT=$(py_send "$JOIN_MSG")
stop_server
if [ "$RESULT" = "$OK_HEX" ]; then
    run_test "connect1" "ok"
else
    run_test "connect1" "fail" "expected $OK_HEX got $RESULT"
fi

# test connect2: two clients each get OK
start_server
R1=$(py_send "$JOIN_MSG")
R2=$(py_send "$JOIN_MSG")
stop_server
if [ "$R1" = "$OK_HEX" ] && [ "$R2" = "$OK_HEX" ]; then
    run_test "connect2" "ok"
else
    run_test "connect2" "fail" "c1=$R1 c2=$R2"
fi

# test ready: both clients get OK then READY
start_server
P1_TMP=$(mktemp)
py_send_recv2 "$JOIN_MSG" > "$P1_TMP" &
P1_PID=$!
sleep 0.2
P2_OUT=$(py_send_recv2 "$JOIN_MSG")
wait "$P1_PID" 2>/dev/null || true
P1_OUT=$(cat "$P1_TMP")
rm -f "$P1_TMP"
stop_server
if echo "$P1_OUT" | grep -q "MSG1:${OK_HEX}" && \
   echo "$P1_OUT" | grep -q "MSG2:${READY_HEX}" && \
   echo "$P2_OUT" | grep -q "MSG1:${OK_HEX}" && \
   echo "$P2_OUT" | grep -q "MSG2:${READY_HEX}"; then
    run_test "ready" "ok"
else
    run_test "ready" "fail" "P1=$P1_OUT P2=$P2_OUT"
fi

# test wrong_game_id: server rejects mismatched game_id
start_server
RESULT=$(py_send "$JOIN_WRONG")
stop_server
if [ "$RESULT" = "$ERROR_HEX" ]; then
    run_test "wrong_game_id" "ok"
else
    run_test "wrong_game_id" "fail" "expected $ERROR_HEX got $RESULT"
fi

# ---- custom tests (mirror test-data-custom/task1) ----

# test malformed_join: msg_type != JOIN should yield ERROR
JOIN_BAD_TYPE="990000000400000000"   # type=0x99, len=4, game_id=0
start_server
RESULT=$(py_send "$JOIN_BAD_TYPE")
stop_server
if [ "$RESULT" = "$ERROR_HEX" ]; then
    run_test "malformed_join" "ok"
else
    run_test "malformed_join" "fail" "expected $ERROR_HEX got $RESULT"
fi

# test bad_join_len: JOIN with payload length != 4 should yield ERROR
JOIN_BAD_LEN="01000000080000000000000000"   # type=0x01, len=8, payload=8 bytes of 0
start_server
RESULT=$(py_send "$JOIN_BAD_LEN")
stop_server
if [ "$RESULT" = "$ERROR_HEX" ]; then
    run_test "bad_join_len" "ok"
else
    run_test "bad_join_len" "fail" "expected $ERROR_HEX got $RESULT"
fi

# summary
echo ""
if [ "$FAIL" -gt 0 ]; then
    echo -e "failures:$FAIL_MSGS"
    echo ""
fi
echo -n "test result: "
if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}FAILED${NC}. $PASS passed, $FAIL failed"
else
    echo -e "${GREEN}ok${NC}. $PASS passed"
fi

[ "$FAIL" -eq 0 ] && exit 0 || exit 1

