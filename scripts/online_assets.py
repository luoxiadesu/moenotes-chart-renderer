#!/usr/bin/env python3
"""Read-only object adapter for a configurable S3 Manager object endpoint."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import urllib.parse
import urllib.request

DEFAULT_BASE = "https://hsjkdajbsadnmsadds.zeabur.app/Default/api/buckets/moenotes/objects/"
LIMITS = {".json": 64 * 1024 * 1024, ".gz": 64 * 1024 * 1024, ".png": 16 * 1024 * 1024}


def object_url(base, key):
    parsed = urllib.parse.urlsplit(base)
    if parsed.scheme not in ("http", "https") or not parsed.netloc or parsed.query or parsed.fragment or parsed.username:
        raise ValueError("Object base must be an HTTP(S) URL without credentials, query or fragment")
    parts = key.split("/")
    if not key or any(p in ("", ".", "..") for p in parts) or any(c in key for c in "\\:%?#") or any(ord(c) < 32 for c in key):
        raise ValueError("Invalid object key")
    if not key.startswith(("Live/MusicScore/", "Image/Jacket/")) or PurePosixPath(key).suffix not in LIMITS:
        raise ValueError("Only chart JSON/gzip and jacket PNG resources are supported")
    return base.rstrip("/") + "/" + "/".join(urllib.parse.quote(p, safe="") for p in parts)


def chart_object(key):
    parts = key.split("/")
    if len(parts) != 2 or any(not p or not all(c.isascii() and (c.isalnum() or c in "_-") for c in p) for p in parts):
        raise ValueError("Chart key must be GROUP/SCORE, for example 0069/0069_03")
    return "Live/MusicScore/" + key + "/" + parts[-1] + ".json"


def fetch_object(key, base=DEFAULT_BASE):
    url = object_url(base, key)
    limit = LIMITS[PurePosixPath(key).suffix]
    request = urllib.request.Request(url, headers={"User-Agent": "moenotes-chart-renderer/0.3"})
    with urllib.request.urlopen(request, timeout=30) as response:
        if response.headers.get_content_type() == "text/html":
            raise ValueError("Received a directory page; use the object API base URL")
        if int(response.headers.get("Content-Length", "0")) > limit:
            raise ValueError("Remote object exceeds size limit")
        data = response.read(limit + 1)
    if len(data) > limit:
        raise ValueError("Remote object exceeds size limit")
    suffix = PurePosixPath(key).suffix
    if suffix == ".json":
        if not isinstance(json.loads(data), dict):
            raise ValueError("Chart JSON must be an object")
    elif suffix == ".png" and not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("Jacket resource is not a PNG")
    elif suffix == ".gz" and not data.startswith(b"\x1f\x8b"):
        raise ValueError("Chart resource is not gzip")
    return data, {"object_key": key, "url": url, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=DEFAULT_BASE)
    parser.add_argument("--chart-key", required=True)
    parser.add_argument("--cover-key", help="Optional jacket key from masterdata")
    parser.add_argument("--output", type=Path, required=True, help="Fresh local by-key directory")
    args = parser.parse_args()
    keys = [chart_object(args.chart_key)]
    if args.cover_key:
        keys.append(f"Image/Jacket/{args.cover_key}/{args.cover_key}__00000.png")
    manifest = []
    # Fetch/validate all requested objects before publishing the new directory.
    resources = [(key, *fetch_object(key, args.base)) for key in keys]
    args.output.mkdir(parents=True, exist_ok=False)
    for key, data, evidence in resources:
        path = args.output / key
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        manifest.append(evidence)
    (args.output / "resource-provenance.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
