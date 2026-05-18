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
echo "running 8 tests"
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

# test alt_game_id: with MULTIPLE_GAMES, any game_id is accepted
start_server
RESULT=$(py_send "$JOIN_WRONG")
stop_server
if [ "$RESULT" = "$OK_HEX" ]; then
    run_test "alt_game_id" "ok"
else
    run_test "alt_game_id" "fail" "expected $OK_HEX got $RESULT"
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

# Python helper: full task2 flow — two clients join, place ships, expect player numbers
py_task2_flow() {
    python3 -c "
import socket, threading, time

def make_join(gid=0):
    return bytes([0x01]) + (4).to_bytes(4,'big') + gid.to_bytes(4,'big')

def make_ships():
    payload = b''
    for c,l,d in [('A1',2,0),('A2',3,0),('A3',4,0),('A4',5,0)]:
        cb = c.encode()
        payload += bytes([len(cb)]) + cb + bytes([l,d])
    return bytes([0x02]) + len(payload).to_bytes(4,'big') + payload

def recv_msg(s):
    h = b''
    while len(h)<5:
        c = s.recv(5-len(h))
        if not c: return None,None
        h += c
    t = h[0]; n = int.from_bytes(h[1:],'big')
    body = b''
    while len(body)<n:
        c = s.recv(n-len(body))
        if not c: break
        body += c
    return t, body

out = {}
def player(name, sock):
    sock.sendall(make_join(0))
    recv_msg(sock)  # OK
    recv_msg(sock)  # READY
    sock.sendall(make_ships())
    t,b = recv_msg(sock)
    out[name] = (t, b.hex() if b else '')

s1 = socket.socket(); s1.settimeout(5); s1.connect(('$HOST', $PORT))
s2 = socket.socket(); s2.settimeout(5); s2.connect(('$HOST', $PORT))
t1=threading.Thread(target=player,args=('p1',s1)); t1.start()
time.sleep(0.1)
t2=threading.Thread(target=player,args=('p2',s2)); t2.start()
t1.join(5); t2.join(5)
print('p1=%s p2=%s' % (out.get('p1'), out.get('p2')))
s1.close(); s2.close()
"
}

# test task2_ships: both players send ships and receive correct player numbers
start_server
RESULT=$(py_task2_flow)
stop_server
if echo "$RESULT" | grep -q "p1=(132, '01')" && echo "$RESULT" | grep -q "p2=(132, '02')"; then
    run_test "task2_ships" "ok"
else
    run_test "task2_ships" "fail" "$RESULT"
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

# test concurrent_games: two games (id=0, id=1) progress independently
start_server
CG=$(python3 -c "
import socket, threading, time
def send(s, t, p=b''): s.sendall(bytes([t])+len(p).to_bytes(4,'big')+p)
def recv(s):
    h=b''
    while len(h)<5:
        c=s.recv(5-len(h))
        if not c: return None,None
        h+=c
    n=int.from_bytes(h[1:],'big'); b=b''
    while len(b)<n:
        c=s.recv(n-len(b))
        if not c: break
        b+=c
    return h[0], b
def ships():
    p=b''
    for c,l,d in [('A1',2,0),('A2',3,0),('A3',4,0),('A4',5,0)]:
        cb=c.encode(); p+=bytes([len(cb)])+cb+bytes([l,d])
    return p
out={}
def play(name, gid):
    s=socket.socket(); s.settimeout(5); s.connect(('127.0.0.1',30023))
    send(s, 0x01, gid.to_bytes(4,'big'))
    recv(s)  # OK
    recv(s)  # READY
    send(s, 0x02, ships())
    out[name]=recv(s)  # PLAYER
    s.close()
def pair(gid, a, b):
    t1=threading.Thread(target=play,args=(a,gid)); t1.start(); time.sleep(0.05)
    t2=threading.Thread(target=play,args=(b,gid)); t2.start()
    t1.join(5); t2.join(5)
ta=threading.Thread(target=pair,args=(0,'g0p1','g0p2'))
tb=threading.Thread(target=pair,args=(1,'g1p1','g1p2'))
ta.start(); tb.start(); ta.join(8); tb.join(8)
print('g0p1=%s g0p2=%s g1p1=%s g1p2=%s' % (out.get('g0p1'),out.get('g0p2'),out.get('g1p1'),out.get('g1p2')))
")
stop_server
if echo "$CG" | grep -q "g0p1=(132, b'\\\\x01') g0p2=(132, b'\\\\x02')" && \
   echo "$CG" | grep -q "g1p1=(132, b'\\\\x01') g1p2=(132, b'\\\\x02')"; then
    run_test "concurrent_games" "ok"
else
    run_test "concurrent_games" "fail" "$CG"
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

