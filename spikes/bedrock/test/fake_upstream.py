"""Upstream palsu: meniru respons ConverseStream (frame event-stream biner)."""
import json, struct, sys, time, zlib
from http.server import BaseHTTPRequestHandler, HTTPServer

OUT = sys.argv[2]


def frame(event_type, payload):
    headers = b""
    for name, value in ((":event-type", event_type), (":content-type", "application/json"), (":message-type", "event")):
        n, v = name.encode(), value.encode()
        headers += bytes([len(n)]) + n + b"\x07" + struct.pack(">H", len(v)) + v
    body = json.dumps(payload).encode()
    total = 12 + len(headers) + len(body) + 4
    prelude = struct.pack(">II", total, len(headers))
    prelude += struct.pack(">I", zlib.crc32(prelude))
    msg = prelude + headers + body
    return msg + struct.pack(">I", zlib.crc32(msg))


FRAMES = [
    frame("messageStart", {"role": "assistant"}),
    frame("contentBlockDelta", {"contentBlockIndex": 0, "delta": {"text": "halo "}}),
    frame("contentBlockDelta", {"contentBlockIndex": 0, "delta": {"text": "dari labu"}}),
    frame("contentBlockStop", {"contentBlockIndex": 0}),
    frame("messageStop", {"stopReason": "end_turn"}),
    frame("metadata", {"usage": {"inputTokens": 3, "outputTokens": 4, "totalTokens": 7}, "metrics": {"latencyMs": 1}}),
]


class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_POST(self):
        n = int(self.headers.get("content-length", 0))
        self.rfile.read(n)
        with open(OUT + "/upstream_seen.json", "w") as f:
            json.dump({"path": self.path, "headers": dict(self.headers)}, f, indent=1)
        with open(OUT + "/upstream_sent.bin", "wb") as f:
            f.write(b"".join(FRAMES))
        self.send_response(200)
        self.send_header("content-type", "application/vnd.amazon.eventstream")
        self.send_header("x-amzn-requestid", "req-123")
        self.send_header("set-cookie", "session=CANARY_COOKIE_SHOULD_NOT_BE_ON_DISK")
        self.send_header("transfer-encoding", "chunked")
        self.end_headers()
        for fr in FRAMES:  # satu frame per chunk, dengan jeda, supaya benar-benar streaming
            self.wfile.write(b"%x\r\n" % len(fr) + fr + b"\r\n")
            self.wfile.flush()
            time.sleep(0.05)
        self.wfile.write(b"0\r\n\r\n")

    def log_message(self, *a):
        pass


HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
