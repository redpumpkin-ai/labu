"""Klien uji Bedrock asli. Klien memakai kredensial dummy: tanda tangannya dibuang proxy.

  client_real.py <endpoint> <model> <out_dir> record   satu call boto3, simpan teks
  client_real.py <endpoint> <model> <out_dir> replay   call boto3 yang sama + POST mentah
                                                       dengan body identik, simpan byte
"""
import json, sys, urllib.request
import boto3

endpoint, model, out, mode = sys.argv[1:5]
captured = {}


def capture(request, **_):
    captured["url"], captured["body"] = request.url, request.body


client = boto3.client("bedrock-runtime", endpoint_url=endpoint)
client.meta.events.register("before-send.bedrock-runtime.ConverseStream", capture)
resp = client.converse_stream(
    modelId=model,
    messages=[{"role": "user", "content": [{"text": "Bilang halo dalam 3 kata."}]}],
    inferenceConfig={"maxTokens": 50, "temperature": 0},
)
events, text, usage = [], "", None
for ev in resp["stream"]:
    events.append(next(iter(ev)))
    if "contentBlockDelta" in ev:
        text += ev["contentBlockDelta"]["delta"].get("text", "")
    if "metadata" in ev:
        usage = ev["metadata"]["usage"]
print("status :", resp["ResponseMetadata"]["HTTPStatusCode"])
print("events :", len(events), sorted(set(events)))
print("text   :", repr(text))
print("usage  :", usage)
json.dump({"text": text, "usage": usage, "events": events}, open(f"{out}/{mode}.json", "w"))

if mode == "replay":
    body = captured["body"]
    body = body if isinstance(body, bytes) else body.encode()
    req = urllib.request.Request(captured["url"], data=body, headers={"content-type": "application/json"}, method="POST")
    data = urllib.request.urlopen(req).read()
    open(f"{out}/replay_raw.bin", "wb").write(data)
    print("raw    :", len(data), "bytes")
