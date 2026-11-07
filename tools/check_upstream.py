#!/usr/bin/env python3
"""Read-only upstream freshness check. Never merges, repins or replaces code."""
import json
from pathlib import Path
from urllib.request import Request, urlopen

root = Path(__file__).resolve().parents[1]
pinned = json.loads((root / "compatibility.json").read_text())["horizon_revision"]
url = "https://api.github.com/repos/horizon-rust/horizon/git/ref/heads/main"
request = Request(url, headers={"Accept": "application/vnd.github+json", "User-Agent": "horizon-civilizations-check"})
with urlopen(request, timeout=15) as response:
    latest = json.load(response)["object"]["sha"]
print(json.dumps({"pinned": pinned, "upstream_main": latest, "update_candidate": pinned != latest,
                  "action": "manual build and E2E before repinning"}, indent=2))
