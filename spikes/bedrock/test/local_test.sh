#!/bin/bash
# Uji lokal spike: boto3 -> proxy record -> upstream palsu, lalu replay di env bersih.
S="$(cd "$(dirname "$0")" && pwd)"
BIN=/Users/linggih-rp/workspaces/labu/spikes/bedrock/target/debug/bedrock-spike
mkdir -p "$S/out" "$S/emptyhome"; rm -f "$S"/out/*
waitport() { for i in $(seq 1 200); do nc -z 127.0.0.1 "$1" 2>/dev/null && return 0; perl -e 'select(undef,undef,undef,0.05)'; done; echo "port $1 never opened"; return 1; }
MODEL="anthropic.claude-test-v1:0"
export AWS_ACCESS_KEY_ID=AKIDEXAMPLE AWS_SECRET_ACCESS_KEY=dummysecretCANARY AWS_REGION=us-east-1 AWS_DEFAULT_REGION=us-east-1
# Jangan sentuh ~/.aws milik pengguna selama uji lokal.
export AWS_CONFIG_FILE=/dev/null AWS_SHARED_CREDENTIALS_FILE=/dev/null

python3 "$S/fake_upstream.py" 9911 "$S/out" & UP=$!
LABU_UPSTREAM=http://127.0.0.1:9911 LABU_PORT=8787 LABU_CASSETTE="$S/out/c.jsonl" "$BIN" record 2>"$S/out/record.log" & REC=$!
waitport 9911 && waitport 8787
echo "=== RECORD ==="
uv run -q --with boto3 python "$S/client.py" http://127.0.0.1:8787 "$MODEL" "$S/out/record_raw.bin" 2>&1 | tail -5
perl -e 'select(undef,undef,undef,0.3)'
kill $REC $UP 2>/dev/null; wait $REC $UP 2>/dev/null
cat "$S/out/record.log"

echo "=== REPLAY (proxy di env bersih: tanpa AWS_*, HOME kosong, upstream mati) ==="
env -i PATH="$PATH" HOME="$S/emptyhome" LABU_PORT=8788 LABU_CASSETTE="$S/out/c.jsonl" "$BIN" replay 2>"$S/out/replay.log" & REP=$!
waitport 8788
uv run -q --with boto3 python "$S/client.py" http://127.0.0.1:8788 "$MODEL" "$S/out/replay_raw.bin" 2>&1 | tail -5
echo "--- miss test ---"
curl -s -o - -w " [status %{http_code}]\n" -X POST http://127.0.0.1:8788/model/x/converse-stream -d '{"beda":1}'
kill $REP 2>/dev/null; wait $REP 2>/dev/null
cat "$S/out/replay.log"

echo "=== CEK ==="
cmp "$S/out/upstream_sent.bin" "$S/out/record_raw.bin" && echo "upstream == record: byte-equal"
cmp "$S/out/record_raw.bin" "$S/out/replay_raw.bin" && echo "record == replay: byte-equal"
echo "--- yang dilihat upstream ---"
python3 - "$S/out/upstream_seen.json" <<'EOF'
import json, sys
d = json.load(open(sys.argv[1])); h = {k.lower(): v for k, v in d["headers"].items()}
print("path:", d["path"]); print("host:", h.get("host"))
print("auth:", h.get("authorization", "").split("Signature=")[0] + "Signature=<...>")
print("x-amz-date:", h.get("x-amz-date"))
EOF
echo "--- isi cassette ---"
echo "baris: $(wc -l < "$S/out/c.jsonl")"
python3 - "$S/out/c.jsonl" <<'EOF'
import json, sys
for l in open(sys.argv[1]):
    e = json.loads(l); e["body_b64"] = e["body_b64"][:24] + "..."; print(e)
EOF
for needle in CANARY AKIDEXAMPLE Authorization Signature set-cookie req-123; do
  grep -qi "$needle" "$S/out/c.jsonl" && echo "BOCOR: $needle" || echo "bersih: $needle"
done
