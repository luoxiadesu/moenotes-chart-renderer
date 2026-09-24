#!/usr/bin/env python3
"""Follow the authoritative masterdata repository's main branch.

Each update is downloaded consistently from one resolved commit. Consumers use
REGION/current.json; commit directories are cache/provenance, not a frozen feed.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import tempfile
import time
import urllib.request

REPO = "StarMoe-org/moenotes-masterdata"
TABLES = ["MasterLiveMusic", "MasterLiveMusicScore", "MasterText", "MasterBand"]
REGIONS = ["hk-tw-mo", "en", "kr"]


def get(url):
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "moenotes-chart-renderer/0.2"})
            with urllib.request.urlopen(req, timeout=30) as response:
                return response.read()
        except OSError:
            if attempt == 3:
                raise
            time.sleep(0.5 * (attempt + 1))


def head_commit():
    return json.loads(get(f"https://api.github.com/repos/{REPO}/commits/main"))["sha"]


def sync(region, dest, commit=None):
    commit = commit or head_commit()
    if not re.fullmatch("[a-f0-9]{40}", commit):
        raise ValueError("commit must be a full Git SHA")
    base = f"https://raw.githubusercontent.com/{REPO}/{commit}/"
    version = json.loads(get(base + "current_version.json"))["regions"][region]
    data_path = version["data_path"]
    if data_path not in REGIONS:
        raise ValueError("Unexpected masterdata directory")
    final = dest / region / commit
    final.parent.mkdir(parents=True, exist_ok=True)
    if final.exists():
        provenance = json.loads((final / "provenance.json").read_text(encoding="utf-8"))
        if provenance["commit"] != commit or provenance["region"] != region:
            raise ValueError("Cache provenance mismatch")
        if provenance.get("repository") != "https://github.com/" + REPO:
            raise ValueError("Cache repository mismatch")
        for table in TABLES:
            name = table + ".json"
            digest = provenance.get("files", {}).get(name)
            if hashlib.sha256((final / name).read_bytes()).hexdigest() != digest:
                raise ValueError(f"Cache hash mismatch: {name}")
    else:
        stage = Path(tempfile.mkdtemp(prefix=".fetch-", dir=final.parent))
        try:
            hashes = {}
            for name in TABLES:
                data = get(base + data_path + "/" + name + ".json")
                if not isinstance(json.loads(data).get("_allData"), list):
                    raise ValueError(f"Invalid table {name}")
                filename = name + ".json"
                (stage / filename).write_bytes(data)
                hashes[filename] = hashlib.sha256(data).hexdigest()
            provenance = {
                "schema_version": 1, "repository": "https://github.com/" + REPO,
                "commit": commit, "region": region, "version": version["version"],
                "resource_version": version["resource_version"], "verified_at": version["verified_at"],
                "fetched_at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "files": hashes,
            }
            (stage / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8")
            try:
                stage.rename(final)
            except FileExistsError:
                # A simultaneous updater may have installed the same commit.
                return sync(region, dest, commit)
        finally:
            if stage.exists():
                shutil.rmtree(stage)
    pointer = dest / region / "current.json"
    with tempfile.NamedTemporaryFile(mode="w", prefix=".current-", dir=pointer.parent, delete=False) as tmp:
        json.dump({"commit": commit, "directory": commit, "checked_at": datetime.datetime.now(datetime.timezone.utc).isoformat()}, tmp)
        tmp.flush()
        os.fsync(tmp.fileno())
        temp_path = Path(tmp.name)
    os.replace(temp_path, pointer)
    return final


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--region", choices=REGIONS + ["all"], default="hk-tw-mo")
    parser.add_argument("--output", type=Path, default=Path("data/masterdata"))
    parser.add_argument("--watch", action="store_true", help="Poll main and atomically advance current pointers")
    parser.add_argument("--interval", type=float, default=60)
    args = parser.parse_args()
    if args.interval < 30:
        parser.error("--interval must be at least 30 seconds")
    while True:
        try:
            commit = head_commit()
            for region in REGIONS if args.region == "all" else [args.region]:
                sync(region, args.output, commit)
                print(f"{region}: {commit} -> {(args.output / region).resolve()}", flush=True)
        except OSError as error:
            if not args.watch:
                raise
            print(f"Update failed; previous complete snapshot retained: {error}", flush=True)
        if not args.watch:
            break
        time.sleep(args.interval)


if __name__ == "__main__":
    main()
