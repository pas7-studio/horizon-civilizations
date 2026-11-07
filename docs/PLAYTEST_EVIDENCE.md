# Read-only evidence from the first Windows playtest

This is a **save-metadata checker**, not an automated in-game test. Use only a
fresh, disposable `Civilizations-Test` world after launching the modded client.
Complete the commands in `docs/TEST_PLAN.md` in-game; especially `/civ found
Ashford`, `/civ actors Ashford` and `/civ roster Ashford`. **Save and exit the
world** before collecting evidence so the JSON snapshot is stable.

From a PowerShell terminal in the repository checkout:

```powershell
python tools/playtest_evidence.py `
  --world-dir 'C:\Games\Horizon\saves\Civilizations-Test' `
  --settlement Ashford --min-linked 10 `
  --output '.\playtest-evidence.json'
```

Use the actual path to your test world (it must contain the folder
`horizon-civilizations\world.json`). The optional `--log` flag accepts the
client's `latest.log`; only aggregate mod-related line counts are included in
the report. The script does not upload or modify game files, and it refuses to
write its output anywhere under the world save directory.

Expected after successful NPC creation: `uuid_linked: 10`. The report also
shows saved construction cursor, completed physical hut record count, farm
stage, harvest sequence, receipt count and world day. A `PASS` result only
means **these saved metadata checks passed**. It does NOT prove that NPCs were
visible, crops/blocks were changed, trading was crash-safe, or that a reload
reconciled actual entities. Verify those visually and record screenshots/logs
separately. If `.swap` exists or the file is corrupted, stop: recover manually
from a backup in an isolated test copy, never auto-overwrite the world.

To compare restart behavior, save one report after the first session and a
second report after reopening and executing `/civ roster Ashford`; verify the
saved UUID-linked count, distinct identities and physical progress match.
The world file SHA-256 lets you identify exactly which saved snapshot was
inspected. Do not upload raw `latest.log` or a complete `world.json` containing
player identifiers to a public issue.
