"""Klien: (1) boto3 converse_stream lewat proxy, (2) POST mentah untuk simpan byte respons."""
import sys, urllib.request
import boto3

endpoint, model, out = sys.argv[1], sys.argv[2], sys.argv[3]
kwargs = dict(
    modelId=model,
    messages=[{"role": "user", "content": [{"text": "Bilang halo dalam 3 kata."}]}],
    inferenceConfig={"maxTokens": 50},
)

client = boto3.client("bedrock-runtime", endpoint_url=endpoint)
resp = client.converse_stream(**kwargs)
text, usage = "", None
for ev in resp["stream"]:
    if "contentBlockDelta" in ev:
        text += ev["contentBlockDelta"]["delta"].get("text", "")
    if "metadata" in ev:
        usage = ev["metadata"]["usage"]
print("boto3 text :", repr(text))
print("boto3 usage:", usage)

if out != "-":
    # POST mentah dengan body tetap, untuk membandingkan byte respons record vs replay.
    from urllib.parse import quote
    body = b'{"messages":[{"role":"user","content":[{"text":"raw"}]}]}'
    req = urllib.request.Request(
        f"{endpoint}/model/{quote(model, safe='')}/converse-stream",
        data=body, headers={"content-type": "application/json"}, method="POST",
    )
    data = urllib.request.urlopen(req).read()
    open(out, "wb").write(data)
    print("raw bytes  :", len(data))
