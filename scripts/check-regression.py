#!/usr/bin/env python3
"""
Benchmark Regression Detection Script

Compares benchmark results against a baseline and alerts on significant regressions.

Usage:
    python scripts/check-regression.py benchmark-results.txt

Exit codes:
    0 - No regression detected
    1 - Regression detected
    2 - Baseline not found (first run)
"""

import sys
import os
import re
import json
from pathlib import Path

# Baseline file location
BASELINE_FILE = Path(".benchmark-baseline.json")

# Regression thresholds (percentage increase that triggers alert)
THRESHOLDS = {
    "p99_latency": 10.0,      # Alert if P99 latency increases >10%
    "throughput": -5.0,       # Alert if throughput decreases >5%
    "memory_per_conn": 20.0,  # Alert if memory per connection increases >20%
}


def parse_benchmark_results(filepath: str) -> dict:
    """Parse benchmark output to extract key metrics."""
    metrics = {}

    with open(filepath, 'r') as f:
        content = f.read()

    # Extract P99 latency (µs)
    p99_match = re.search(r'P99:\s*(\d+)', content)
    if p99_match:
        metrics['p99_latency'] = int(p99_match.group(1))

    # Extract throughput (msg/s)
    throughput_match = re.search(r'Throughput:\s*([\d.]+)\s*msg/s', content)
    if throughput_match:
        metrics['throughput'] = float(throughput_match.group(1))

    # Extract P50 latency
    p50_match = re.search(r'P50:\s*(\d+)', content)
    if p50_match:
        metrics['p50_latency'] = int(p50_match.group(1))

    # Extract P95 latency
    p95_match = re.search(r'P95:\s*(\d+)', content)
    if p95_match:
        metrics['p95_latency'] = int(p95_match.group(1))

    return metrics


def load_baseline() -> dict:
    """Load baseline metrics from file."""
    if not BASELINE_FILE.exists():
        return None
    with open(BASELINE_FILE, 'r') as f:
        return json.load(f)


def save_baseline(metrics: dict):
    """Save metrics as new baseline."""
    with open(BASELINE_FILE, 'w') as f:
        json.dump(metrics, f, indent=2)
    print(f"Saved new baseline to {BASELINE_FILE}")


def check_regression(current: dict, baseline: dict) -> list:
    """Compare current metrics against baseline, return list of regressions."""
    regressions = []

    for metric, threshold in THRESHOLDS.items():
        if metric not in current or metric not in baseline:
            continue

        current_val = current[metric]
        baseline_val = baseline[metric]

        if baseline_val == 0:
            continue

        # Calculate percentage change
        # Positive threshold: alert if current > baseline by threshold %
        # Negative threshold: alert if current < baseline by abs(threshold) %
        if threshold > 0:
            change_pct = ((current_val - baseline_val) / baseline_val) * 100
            if change_pct > threshold:
                regressions.append({
                    'metric': metric,
                    'baseline': baseline_val,
                    'current': current_val,
                    'change_pct': change_pct,
                    'threshold': threshold,
                })
        else:
            change_pct = ((baseline_val - current_val) / baseline_val) * 100
            if change_pct > abs(threshold):
                regressions.append({
                    'metric': metric,
                    'baseline': baseline_val,
                    'current': current_val,
                    'change_pct': -change_pct,
                    'threshold': threshold,
                })

    return regressions


def main():
    if len(sys.argv) < 2:
        print("Usage: check-regression.py <benchmark-results.txt>")
        sys.exit(2)

    results_file = sys.argv[1]
    update_baseline = "--update-baseline" in sys.argv

    # Parse current results
    print(f"Parsing benchmark results from {results_file}...")
    current = parse_benchmark_results(results_file)
    print(f"Current metrics: {json.dumps(current, indent=2)}")

    # Load baseline
    baseline = load_baseline()

    if baseline is None:
        print("No baseline found. This appears to be the first run.")
        save_baseline(current)
        sys.exit(2)

    print(f"Baseline metrics: {json.dumps(baseline, indent=2)}")

    # Check for regressions
    regressions = check_regression(current, baseline)

    if regressions:
        print("\n⚠️  REGRESSION DETECTED!")
        print("=" * 60)
        for r in regressions:
            print(f"  {r['metric']}:")
            print(f"    Baseline: {r['baseline']}")
            print(f"    Current:  {r['current']}")
            print(f"    Change:   {r['change_pct']:+.1f}% (threshold: {r['threshold']:+.1f}%)")
        print("=" * 60)

        if not update_baseline:
            sys.exit(1)
    else:
        print("\n✓ No regression detected")

    # Optionally update baseline
    if update_baseline:
        save_baseline(current)

    sys.exit(0)


if __name__ == "__main__":
    main()
