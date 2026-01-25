#!/usr/bin/env python3
"""Check Criterion benchmarks against configured thresholds."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tomllib


DEFAULT_CRITERION_DIR = Path("target/criterion")
DEFAULT_THRESHOLDS = Path("benchmarks/thresholds.toml")
ESTIMATES_FILE = Path("new/estimates.json")


def load_thresholds(path: Path) -> dict[str, float]:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    raw = data.get("benchmarks", {})
    thresholds: dict[str, float] = {}
    for name, config in raw.items():
        if not isinstance(config, dict):
            raise ValueError(f"Invalid config for {name}: expected table")
        if "max_mean_ms" in config:
            thresholds[name] = float(config["max_mean_ms"]) * 1_000_000.0
        elif "max_mean_ns" in config:
            thresholds[name] = float(config["max_mean_ns"])
        else:
            raise ValueError(f"Missing max_mean_ms or max_mean_ns for {name}")
    return thresholds


def load_mean_ns(criterion_dir: Path, benchmark: str) -> float:
    estimates_path = criterion_dir / benchmark / "new" / "estimates.json"
    if not estimates_path.exists():
        raise FileNotFoundError(
            f"Missing estimates.json for {benchmark} at {estimates_path}"
        )
    data = json.loads(estimates_path.read_text(encoding="utf-8"))
    mean = data.get("mean", {})
    point = mean.get("point_estimate")
    if point is None:
        raise ValueError(f"Missing mean.point_estimate in {estimates_path}")
    return float(point)


def discover_benchmarks(criterion_dir: Path) -> set[str]:
    benchmarks: set[str] = set()
    if not criterion_dir.exists():
        return benchmarks
    for estimates_path in criterion_dir.rglob(str(ESTIMATES_FILE)):
        try:
            benchmark = str(estimates_path.parent.parent.relative_to(criterion_dir))
        except ValueError:
            continue
        benchmarks.add(benchmark)
    return benchmarks


def format_ns(ns: float) -> str:
    if ns >= 1_000_000.0:
        return f"{ns / 1_000_000.0:.2f} ms"
    if ns >= 1_000.0:
        return f"{ns / 1_000.0:.2f} µs"
    return f"{ns:.2f} ns"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate Criterion benchmarks against thresholds."
    )
    parser.add_argument(
        "--criterion-dir",
        type=Path,
        default=DEFAULT_CRITERION_DIR,
        help="Path to target/criterion directory",
    )
    parser.add_argument(
        "--thresholds",
        type=Path,
        default=DEFAULT_THRESHOLDS,
        help="Path to thresholds TOML",
    )
    parser.add_argument(
        "--allow-unknown",
        action="store_true",
        help="Ignore benchmark results that are missing thresholds",
    )
    args = parser.parse_args()

    if not args.thresholds.exists():
        print(f"Thresholds file not found: {args.thresholds}")
        return 2

    try:
        thresholds = load_thresholds(args.thresholds)
    except Exception as exc:  # noqa: BLE001
        print(f"Failed to load thresholds: {exc}")
        return 2

    if not args.criterion_dir.exists():
        print(f"Criterion directory not found: {args.criterion_dir}")
        return 2

    discovered = discover_benchmarks(args.criterion_dir)
    missing_results = sorted(set(thresholds) - discovered)
    if missing_results:
        print("Missing benchmark results:")
        for name in missing_results:
            print(f"- {name}")
        return 2

    unknown_results = sorted(discovered - set(thresholds))
    if unknown_results and not args.allow_unknown:
        print("Missing thresholds for benchmarks:")
        for name in unknown_results:
            print(f"- {name}")
        print("Rerun with --allow-unknown to ignore.")
        return 2
    if unknown_results and args.allow_unknown:
        print("Ignoring benchmarks without thresholds:")
        for name in unknown_results:
            print(f"- {name}")

    failures: list[str] = []
    for benchmark, limit_ns in thresholds.items():
        try:
            mean_ns = load_mean_ns(args.criterion_dir, benchmark)
        except Exception as exc:  # noqa: BLE001
            failures.append(f"{benchmark}: {exc}")
            continue

        status = "OK"
        if mean_ns > limit_ns:
            status = "FAIL"
            failures.append(
                f"{benchmark}: mean {format_ns(mean_ns)} > limit {format_ns(limit_ns)}"
            )

        print(
            f"{benchmark}: {format_ns(mean_ns)} (limit {format_ns(limit_ns)}) [{status}]"
        )

    if failures:
        print("\nFailures:")
        for failure in failures:
            print(f"- {failure}")
        return 1

    print("\nAll benchmarks within thresholds.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
