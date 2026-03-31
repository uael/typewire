#!/usr/bin/env python3
"""Compare current coverage against the parent commit's git note.

Exits non-zero if any crate's line coverage drops by more than 1.0
percentage point.

Usage:
    coverage-delta.py <coverage.json>

The script reads per-crate coverage from <coverage.json> (written by
``cargo xtask test --coverage``) and the parent commit's coverage from
the ``refs/notes/coverage`` git notes ref.
"""

from __future__ import annotations

import json
import subprocess
import sys
from typing import Dict, Optional


def get_parent_coverage() -> Optional[Dict[str, float]]:
  """Read coverage percentages from the parent commit's git note.

  For merge commits the first parent is used (the branch being merged
  into).  For PRs this is the merge-base with the target branch.

  Returns a dict mapping crate name -> percent, or None when no note
  exists (first run).
  """
  # Determine the comparison base.  On a PR the merge-base with main
  # is most useful; on a push to main HEAD~1 suffices.  We try the
  # merge-base first and fall back to HEAD~1.
  base_sha = None
  try:
    base_sha = (
      subprocess.check_output(
        ["git", "merge-base", "HEAD", "origin/main"],
        stderr=subprocess.DEVNULL,
      )
      .decode()
      .strip()
    )
  except subprocess.CalledProcessError:
    pass

  if base_sha is None:
    try:
      base_sha = (
        subprocess.check_output(
          ["git", "rev-parse", "HEAD~1"],
          stderr=subprocess.DEVNULL,
        )
        .decode()
        .strip()
      )
    except subprocess.CalledProcessError:
      return None

  # Fetch notes ref (may not exist yet).
  subprocess.run(
    ["git", "fetch", "origin", "refs/notes/coverage:refs/notes/coverage"],
    capture_output=True,
  )

  # Read the note attached to the base commit.
  try:
    note = (
      subprocess.check_output(
        ["git", "notes", "--ref=coverage", "show", base_sha],
        stderr=subprocess.DEVNULL,
      )
      .decode()
      .strip()
    )
  except subprocess.CalledProcessError:
    return None

  result = {}  # type: Dict[str, float]
  for line in note.splitlines():
    line = line.strip()
    if not line:
      continue
    # Format: "crate-name: 85.2%"
    name, _, pct = line.partition(":")
    name = name.strip()
    pct = pct.strip().rstrip("%")
    try:
      result[name] = float(pct)
    except ValueError:
      continue
  return result or None


def main() -> None:
  if len(sys.argv) != 2:
    print("Usage: {} <coverage.json>".format(sys.argv[0]), file=sys.stderr)
    sys.exit(2)

  with open(sys.argv[1]) as f:
    current = json.load(f)

  current_map = {c["name"]: c["percent"] for c in current}  # type: Dict[str, float]

  parent = get_parent_coverage()

  if parent is None:
    print("No parent coverage note found -- skipping delta check.")
    return

  # Print comparison table.
  print()
  print("{:<25} {:>8} {:>8} {:>8}  {}".format("Crate", "Old", "New", "Delta", "Status"))
  print("-" * 62)

  failed = False
  for name in sorted(current_map):
    new_pct = current_map[name]
    old_pct = parent.get(name)
    if old_pct is None:
      print("{:<25} {:>8} {:>7.1f}% {:>8}  new".format(name, "N/A", new_pct, ""))
      continue
    delta = new_pct - old_pct
    status = "ok"
    if old_pct - new_pct > 1.0:
      status = "FAIL (>1% regression)"
      failed = True
    elif delta < 0:
      status = "warn"
    print(
      "{:<25} {:>7.1f}% {:>7.1f}% {:>+7.1f}%  {}".format(name, old_pct, new_pct, delta, status)
    )

  print()

  if failed:
    print("FAILED: Coverage regression exceeds 1.0 percentage point threshold.")
    sys.exit(1)
  else:
    print("Coverage delta check passed.")


if __name__ == "__main__":
  main()
