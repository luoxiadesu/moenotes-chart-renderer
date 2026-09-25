#!/usr/bin/env python3
"""Render every score in current masterdata as one complete multi-column PNG.

Linux benchmark: sequential fresh CLI processes. /usr/bin/time reports process
RSS; renderer emits per-stage timings. All manifest rows, including failures and
orphan score metadata, are recorded. Never silently lowers quality or paginates.
"""
import argparse
import csv
import datetime
import hashlib
import json
import platform
from pathlib import Path
import statistics
import subprocess
import time
from sync_masterdata import REGIONS, head_commit, sync


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rows(path, name):
    return json.loads((path / (name + ".json")).read_text(encoding="utf-8"))["_allData"]


def distribution(values):
    if not values:
        return {}
    values = sorted(values)
    return {"min": values[0], "median": statistics.median(values), "mean": statistics.mean(values),
            "p95": values[max(0, int(len(values) * .95 + .9999) - 1)], "max": values[-1], "sum": sum(values)}


def benchmark(args):
    args.output.mkdir(parents=True, exist_ok=True)
    started_at=datetime.datetime.now(datetime.timezone.utc).isoformat()
    started = time.perf_counter()
    commit = head_commit()
    regions = REGIONS if args.region == "all" else [args.region]
    snapshots = {region: sync(region, args.output / "masterdata", commit) for region in regions}
    all_files = {}
    for path in (args.assets / "Live/MusicScore").rglob("*.json"):
        all_files.setdefault(path.stem, []).append(path)
    manifest = []
    for region, snapshot in snapshots.items():
        music = rows(snapshot, "MasterLiveMusic")
        owners = {}
        for item in music:
            for field, difficulty in [("_easyID", "EASY"), ("_normalID", "NORMAL"), ("_hardID", "HARD"), ("_expertID", "EXPERT")]:
                if item.get(field):
                    owners[item[field]] = (item, difficulty)
        provenance = json.loads((snapshot / "provenance.json").read_text(encoding="utf-8"))
        for score in rows(snapshot, "MasterLiveMusicScore"):
            key = score["_musicScoreTextFileName"]
            candidates = all_files.get(key.rsplit("/", 1)[-1], [])
            manifest.append({"region": region, "score_id": score["_id"], "chart_key": key,
                             "has_music_metadata": score["_id"] in owners,
                             "chart_candidates": len(candidates), "input": str(candidates[0].resolve()) if len(candidates) == 1 else None,
                             "input_sha256": sha(candidates[0]) if len(candidates) == 1 else None,
                             "master_level": score.get("_musicScoreDisplayLevel"), "master_full_combo": score.get("_fullComboCount"),
                             "master_commit": commit, "master_version": provenance["version"]})
    (args.output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    settings = {"mode": "complete multi-column single PNG", "skin": args.skin, "theme": args.theme,
                "pixels_per_beat": 64, "pixels_per_lane": 10, "note_height": 8, "arrow_height": 10,
                "supersample": 2, "auto_spacing": True, "mirror": False,
                "processes": 1, "chart_source": "local assets; not claimed to be freshly downloaded CDN charts"}
    results = []
    wall_render_started = time.perf_counter()
    with (args.output / "per-chart.jsonl").open("w", encoding="utf-8") as log:
        for index, item in enumerate(manifest):
            result = dict(item)
            result["status"] = "missing_input" if item["chart_candidates"] == 0 else "ambiguous_input" if item["chart_candidates"] != 1 else "pending"
            if result["status"] == "pending":
                destination = args.output / "images" / item["region"] / (item["chart_key"].replace("/", "_") + ".png")
                destination.parent.mkdir(parents=True, exist_ok=True)
                timing = destination.with_suffix(".time.txt")
                command = [str(args.binary.resolve()), "render", item["input"], "-o", str(destination), "--skin", args.skin,
                           "--timings", "--supersample", "2", "--theme", args.theme]
                if args.packs:
                    command += ["--packs", str(args.packs.resolve())]
                if item["has_music_metadata"]:
                    command += ["--masterdata", str(snapshots[item["region"]]), "--chart-key", item["chart_key"], "--language", "ja", "--assets", str(args.assets.resolve())]
                else:
                    orphan = destination.with_suffix(".metadata.json")
                    orphan.write_text(json.dumps({"title": item["chart_key"], "level": str(item["master_level"]),
                                                   "master_full_combo": item["master_full_combo"], "provenance": {"repository": "https://github.com/StarMoe-org/moenotes-masterdata", "commit": commit, "region": item["region"], "warning": "Score row has no MasterLiveMusic owner; title/author/cover unknown"}}), encoding="utf-8")
                    command += ["--metadata", str(orphan)]
                tick = time.perf_counter()
                try:
                    process = subprocess.run(["/usr/bin/time", "-f", "%e %M", "-o", str(timing), *command], capture_output=True, text=True, timeout=180, encoding="utf-8")
                    result["wall_seconds"] = time.perf_counter() - tick
                    result["exit_code"] = process.returncode
                    final_line = timing.read_text(encoding="utf-8").strip().splitlines()[-1].split()
                    result["process_wall_seconds"] = float(final_line[0]); result["peak_rss_kib"] = int(final_line[1])
                    for line in process.stderr.splitlines():
                        if line.startswith("TIMINGS "):
                            result.update(json.loads(line[8:]))
                    if process.returncode:
                        result.update(status="render_failed", diagnostic=process.stderr[-3000:])
                    else:
                        report = json.loads(destination.with_suffix(".render.json").read_text(encoding="utf-8"))
                        if len(report["images"]) != 1 or report["images"][0]["file"] != destination.name:
                            raise ValueError("Render split the chart into multiple images")
                        image = report["images"][0]
                        from PIL import Image
                        with Image.open(destination) as pixels:
                            pixels.load()
                            if pixels.size != (image["width"], image["height"]): raise ValueError("PNG dimension mismatch")
                        if sha(destination) != image["sha256"]: raise ValueError("PNG SHA mismatch")
                        result.update(status="ok", width=image["width"], height=image["height"], png_bytes=destination.stat().st_size,
                                      png_sha256=image["sha256"], columns=report["columns"], glyphs=report["glyphs"], branches=report["branches"],
                                      effective_pixels_per_beat=report["layout"]["pixels_per_beat"], warnings=report["warnings"],
                                      reconstructed_full_combo=report["statistics"]["reconstructed_full_combo"])
                except (subprocess.TimeoutExpired, OSError, ValueError) as error:
                    result.update(status="test_failed", diagnostic=str(error), wall_seconds=time.perf_counter() - tick)
            results.append(result); log.write(json.dumps(result) + "\n"); log.flush()
            if (index + 1) % 20 == 0:
                print(f"{index + 1}/{len(manifest)}  ok={sum(r['status']=='ok' for r in results)}  failures={sum(r['status']!='ok' for r in results)}", flush=True)
    try:
        end_commit = head_commit()
    except OSError:
        end_commit = None
    ok = [r for r in results if r["status"] == "ok"]
    summary = {"schema_version": 1, "started_at": started_at,"finished_at":datetime.datetime.now(datetime.timezone.utc).isoformat(),
               "repository": "https://github.com/StarMoe-org/moenotes-masterdata", "start_commit": commit, "end_commit": end_commit,
               "master_changed_during_test": end_commit != commit if end_commit else None,
               "settings": settings, "environment": {"platform": platform.platform(), "processor": platform.processor(), "binary_sha256": sha(args.binary)},
               "rows": len(results), "unique_chart_keys": len({r['chart_key'] for r in results}),
               "successful_single_images": len(ok), "failures": [r for r in results if r["status"] != "ok"],
               "orphan_metadata_rows": sum(not r["has_music_metadata"] for r in results),
               "render_loop_seconds": time.perf_counter() - wall_render_started, "total_seconds_including_sync": time.perf_counter() - started,
               "wall_seconds": distribution([r['wall_seconds'] for r in ok]), "render_encode_seconds": distribution([r['render_encode_seconds'] for r in ok]),
               "peak_rss_kib": distribution([r['peak_rss_kib'] for r in ok]),
               "per_region": {region: {"rows": sum(r['region']==region for r in results), "ok": sum(r['region']==region for r in ok),
                                         "wall_seconds": distribution([r['wall_seconds'] for r in ok if r['region']==region])} for region in regions}}
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    with (args.output / "per-chart.csv").open("w", newline="", encoding="utf-8") as output:
        keys = ["region", "score_id", "chart_key", "status", "has_music_metadata", "wall_seconds", "render_encode_seconds", "parse_seconds", "scene_seconds", "publish_seconds", "peak_rss_kib", "width", "height", "columns", "png_bytes"]
        writer = csv.DictWriter(output, fieldnames=keys, extrasaction="ignore"); writer.writeheader(); writer.writerows(results)
    print(json.dumps({k: summary[k] for k in ["rows", "unique_chart_keys", "successful_single_images", "orphan_metadata_rows", "wall_seconds", "render_encode_seconds", "peak_rss_kib"]}), flush=True)
    return int(bool(summary["failures"]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--packs", type=Path)
    parser.add_argument("--skin", default="builtin")
    parser.add_argument("--theme", choices=["white", "black", "print", "dark"], default="white")
    parser.add_argument("--region", choices=REGIONS + ["all"], default="all")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    raise SystemExit(benchmark(args))


if __name__ == "__main__":
    main()
