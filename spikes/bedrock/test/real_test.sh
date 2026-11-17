#!/bin/bash
# Uji Bedrock asli: SATU call ConverseStream direkam, lalu di-replay tanpa kredensial AWS.
# Pakai: real_test.sh <model-id> [region]
S="$(cd "$(dirname "$0")" && pwd)"
BIN="$S/../target/debug/bedrock-spike"
MODEL="${1:?model id}"; REGION="${2:-ap-southeast-3}"
O="$S/out-real"; mkdir -p "$O" "$S/emptyhome"; rm -f "$O"/*
waitport() { for i in $(seq 1 200); do nc -z 127.0.0.1 "$1" 2>/dev/null && return 0; perl -e 'select(undef,undef,undef,0.05)'; done; echo "port $1 never opened"; return 1; }
# Kredensial dummy hanya untuk klien boto3 (tanda tangannya dibuang proxy).
client() { env -i PATH="$PATH" HOME="$S/emptyhome" AWS_ACCESS_KEY_ID=AKIDDUMMY AWS_SECRET_ACCESS_KEY=dummy AWS_DEFAULT_REGION="$REGION" \
  uv run -q --with boto3 python "$S/client_real.py" "$@" 2>&1 | tail -8; }

echo "=== RECORD (proxy pakai kredensial AWS asli, region $REGION) ==="
AWS_REGION="$REGION" LABU_PORT=8787 LABU_CASSETTE="$O/real.jsonl" "$BIN" record 2>"$O/record.log" & REC=$!
waitport 8787 || { cat "$O/record.log"; exit 1; }
client http://127.0.0.1:8787 "$MODEL" "$O" record
perl -e 'select(undef,undef,undef,0.5)'
kill $REC 2>/dev/null; wait $REC 2>/dev/null
cat "$O/record.log"
[ -s "$O/real.jsonl" ] || { echo "GAGAL: cassette kosong"; exit 1; }

echo "=== REPLAY (proxy di env bersih: tanpa AWS_*, HOME kosong) ==="
env -i PATH="$PATH" HOME="$S/emptyhome" LABU_PORT=8788 LABU_CASSETTE="$O/real.jsonl" "$BIN" replay 2>"$O/replay.log" & REP=$!
waitport 8788
client http://127.0.0.1:8788 "$MODEL" "$O" replay
kill $REP 2>/dev/null; wait $REP 2>/dev/null
cat "$O/replay.log"

echo "=== CEK ==="
python3 - "$O" <<'EOF'
import base64, json, sys
o = sys.argv[1]
lines = [json.loads(l) for l in open(f"{o}/real.jsonl")]
e = lines[0]
rec = base64.b64decode(e["body_b64"]); rep = open(f"{o}/replay_raw.bin", "rb").read()
print("baris cassette          :", len(lines), "| status", e["status"], "| header", e["headers"])
print("byte upstream vs replay :", "byte-equal" if rec == rep else "BEDA", f"({len(rec)} vs {len(rep)})")
a, b = json.load(open(f"{o}/record.json")), json.load(open(f"{o}/replay.json"))
print("hasil boto3 sama        :", a == b)
EOF
for needle in AKIA ASIA Authorization Signature x-amz-security-token; do
  grep -qi "$needle" "$O/real.jsonl" && echo "BOCOR: $needle" || echo "bersih: $needle"
done
