# Civilizations — синхронізація v0.1.15 (2026-11-08)

## Що зтягнуто/інтегровано
- `origin/main` уже містив наші коміти (`6a1c28f`, `228bf77` — закомічено й запушено).
- Новий стек: `feat/source-pickup-witness-v0115` (86 комітів над main, містить 5 попередніх гілок):
  forestry journal, verified oak stem, 3D worker guards, assigned courier, source-pickup witness,
  first-play diagnostics.
- Локальний `main`: merge-коміт `78b414f` (конфлікт у `core.rs` вирішено), потім `style`-коміт
  відкочено (`git revert e2c72ee`), бо Python source-contract тести звіряють точний текст коду.
  Тому `cargo fmt --check` на цьому стеку НЕ проходить (як і в upstream).

## Наші виправлення цього стеку
- `core.rs`: `deliver_farm_wheat` знову перенесено в `impl World` (borrow-safe), `finish_bakery`
  без подвійного mutable-позичання, `warehouse_debit` без зайвого `{}`.
- `glue.rs`: лишились `stack.item.as_ref() == …` (Arc<str>).
- `tests/test_native_barrel_inventory.py`: контракт оновлено на `as_ref()` (коміт `7c65059`).

## Гейти
- `cargo build` OK; `cargo clippy --all-targets` чисто; `cargo test` **145/145**.
- Python: 5 failures + 2 errors — **стільки ж на чистій гілці** (2 errors — symlink без Developer Mode;
  5 failures — застарілі контракти самого upstream, не наші).
- `tools/package.py` → `civilizations-0.1.14.hnmod` sha256 `7d5db3c8…`; `verify_static.py` PASS.

## Збірка
- Бандл розпаковано в `%TEMP%\opencode\civ-bundle-0115`, `native-build --instance horizon --trust-native-code`.
- У `horizon-native-mods` також лежать `minimap`, `mousetricks`, `treefall` — вони пінять ІНШІ коміти
  Horizon (`6f8c7a0e…`, `142388e1…`) і в цю збірку не включені.

## Таблички (2026-11-08)
- Було: `data merge ... {front_text:{messages:['{"text":"TOWN HALL"}',...]}}` → гра показує компонентний JSON як текст, тому на табличці виднілось `{"text":"…"}`.
- Стало: прості NBT-рядки з NBT-екрануванням, `color:"black"`, `has_glowing_text` для дошки потреб.
- Перевірено в грі на тестовій табличці: рендериться `VILLAGE NEEDS / WOOD -4 / STONE -10 / HOMELESS 10` без JSON.
- Текст дощок: мер — назва села / TOWN HALL / MAYOR <імʼя> / POP <n>; потреби — VILLAGE NEEDS / WOOD -n / STONE -n / HOMELESS n (або WAREHOUSE / PENDING TX / CHECK STOCK).
- Білди: `native-18840db-82fcfe3b0f0c` (без фіксу), далі фінальний з фіксом.

## Таблички на стінах (2026-11-08, v2)
- Було: стоячі таблички на підлозі всередині ратуші (не читались).
- Стало: `minecraft:oak_wall_sign[facing=north]` на зовнішньому боці північної стіни,
  по обидва боки від дверей, на висоті очей (`origin.y + 1`, `origin.z - 3`).
- План ратуші (`town_hall_plan`) зберігає НАЗВУ блока (`minecraft:oak_wall_sign`), бо саме її
  повертає світове читання; `facing=north` додається лише в `setblock` у `build_town_hall_one_step`.
- Міграція вже збудованих ратуш: `refresh_hall_boards` прибирає старі стоячі таблички
  (`legacy_hall_board_positions`) і ставить настінні, якщо місце вільне.
- Вміст: права — назва села / TOWN HALL / MAYOR <імʼя> / POP <n>;
  ліва — VILLAGE NEEDS / WOOD -n / STONE -n / HOMELESS n (підсвічена),
  або STOCK OK / READY TO BUILD, або WAREHOUSE / PENDING TRADE / CHECK BARRELS.
- У грі перевірено вручну: обидві таблички читаються ззовні біля дверей.
- Rust-тест блюпринта оновлено; `cargo test` 145/145, clippy чисто.

## Parallel sessions — shared working tree (3 agents, 2026-11-08)

- The repo, launcher and instance are shared by three concurrent sessions.
  Never revert or reformat another session's hunks; `git status` before edits.
- Launcher builds are exclusive: `error: A native build or cache cleanup is
  running.` means another session is building. Retry after ~40 s, never kill
  the other launcher/rustc PID.
- This session owns: `Job::Laborer/Digger`, `Settlement::workplace_demand` /
  `staff_workplaces`, `staff_village` + `rename_actor` (name tags), `WorkPost`
  kinds (lumber/quarry/sand/dirt), `Good::Sand/Dirt`, `/civ post|posts|roles|
  status`, camp signs, PIT_TARGET pit service in `autonomous_village_step`.
- Another session is modularizing native hooks (`sign_click.rs`, `sign_edit.rs`,
  `horizon-mixin.toml`, test/verify_static updates). Leave those files alone.
- A packaged `.hnmod` snapshots the WHOLE tree, so a build may contain another
  session's unfinished work; check `build.log` errors before blaming your change.

# Module tree (2026-11-08, after main was modularized)

- `main` now holds `mixins/civilizations/core/` + `glue/`; the old single
  `core.rs` / `glue.rs` are gone. `feat/hall-information-boards-v0115` is the
  superseded single-file branch: do not merge it, port from it instead.
- Ported into modules: `glue/build.rs` (dig_natural, two-wide/two-high
  quarry_one_step with step repair and explicit stall logs),
  `core/world/gather.rs` (plan_quarry_gather, gather_claim_ok),
  `glue/gather.rs` (executor uses gather_claim_ok), `glue/economy.rs`
  (a new pit must not overlap an existing plot).
- `main` is at b50c330 and pushed. Gates on main: cargo test 197/197, clippy
  clean, package + verify_static PASS, Python exactly the declared 22-failure
  baseline (249 passing).

# Latest worktree — quarry pit + wider plot search (2026-11-08)

- `Settlement.quarry_step: u8` (serde default 0), `MAX_QUARRY_STEPS = 24`,
  `World::advance_quarry`. A quarry post now opens a real staircase pit:
  `quarry_one_step` removes natural soil/stone growth one step at a time with
  the bound UUID miner near it, and a stone block goes through the normal
  `plan_gather` receipt journal (1 cobble only after that worker returns).
- The miner is bound on the first dig (`bound.or_else(first living miner)`),
  and `guide_actors` walks that miner to the last opened step before anything
  else, so a fresh pit is actually worked.
- VERIFIED in Ashford (blocks read back through the control socket): the pit
  opened at (-75,70,-327), steps 0..4 air, step 5 (-70,65,-327) = stone.
- Real limit: vanilla villager pathing does not descend a 1-wide shaft, so
  the miner works from the rim. The dig reach was raised to
  `native_worker_near(..., 4.5, 5.0)` so a shallow pit works; deeper pits need
  a 2-wide ramp design.
- Plot search also tries a wider ring (48/56/64/72 blocks) and dy +-3, because
  a wooded hillside village has no level 5x5 closer in.
- STILL BLOCKED: huts/bakery need a level 5x5; Ashford's terrain has 1-block
  dips (air at y-1) that clearing cannot fix. Next: fill dips with real dirt
  or cobble taken from the hall barrels, or an even wider survey.
- Gates: cargo test 196/196, clippy clean. Native builds: c76028487d3b,
  86847900b040.

# Latest worktree — village area stays loaded (2026-11-08)

- Chunks are the smallest horizontal load unit; a sub-chunk square cannot be
  loaded. The engine already gives a held chunk's neighbours a lighter level,
  so the optimisation is to hold FEWER chunks.
- `village_chunk_footprint(center, sites)` (core): civic 3x3
  (`HALL_CHUNK_RADIUS = 1`) + the chunk of every real workplace + the corridor
  chunks sampled every 4 blocks from the hall to it. Sorted, deterministic,
  capped at `MAX_HELD_VILLAGE_CHUNKS = 48` (over budget → corridors dropped,
  then the civic 3x3 only). No fixed radius square any more.
- `settlement_work_sites` (glue) = posts + farm + bakery + hall job + active
  hut job + up to 8 completed hut sites. `hold_village_chunks` uses the native
  ticket API `native_force_chunk_in(owner, dimension, chunk, hold)` with owner
  `civilizations:village/<id>` and `native_forced_chunks(owner)` to diff.
- Called in `level_tick` at tick 1 and every 100 ticks BEFORE the planner;
  tickets are not saved by the engine, so they are re-held every 5 s; a
  village with autonomy off releases all of them. `/civ chunks NAME` reports.
- Known limit: mod tickets are always FORCED/entity-ticking (no level
  parameter), and only the level in place is managed, so a village keeps
  working while the player is elsewhere in the SAME dimension only.
- Gates: `cargo test` 175/175, clippy clean, Python 194 passed (7 known).

# Latest worktree — forest plots are opened by the builder (2026-11-08)

- `is_ground` now accepts sand/red sand/gravel/coarse dirt/podzol/rooted dirt/
  snow/andesite/deepslate floors, so desert and river villages are buildable.
- `is_clearable_growth` + `clear_growth_only` + `open_build_plot`: a builder
  may clear natural vegetation (grass, ferns, leaf litter, flowers, snow,
  saplings, leaves, logs) on its own plot. Every cell is validated BEFORE any
  edit, so a refused plot is never half cleared; at most 4 plots are opened
  per planner turn and 120 blocks per plot. Containers, player blocks and
  existing structures are never touched.
- Applied to the civic hall, field, bakery and hut reservations; the first
  refusal is logged (`started no build and no field: ...`) when nothing starts.
- Why: Ashford is a forest village — every 5x5 plot failed on
  `minecraft:leaf_litter` / logs, so no hut, field or bakery could ever begin.

# Latest worktree — sand/dirt pits and readable status (2026-11-08)

- `Good::Sand`/`Good::Dirt` (`minecraft:sand`/`minecraft:dirt`) flow through the
  same physical pipeline as wood/stone: a real block is removed, the bound
  worker walks home, and one real item is inserted into a hall barrel. No virtual
  stock was added. `Good::job()` names the only role allowed to extract a good.
- `PostKind::Sand`/`PostKind::Dirt` with `Job::Digger` (one shared digger seat).
  `/civ post NAME sand|dirt` places a pit; the orchestrator keeps a small
  `PIT_TARGET = 32` buffer per pit and gathers within `POST_RADIUS` of it.
- `/civ status NAME` prints one line: population, laborers, hall state, homeless,
  next builds, missing logs/cobble, camps with their worker, current harvest.
- Role changes now broadcast one gold line to chat (`Ashford: Finn is now the
  village lumberjack`) in addition to the renamed tag and `/civ roles`.
- Gates: `cargo test` 166/166, clippy clean, Python 184 passed with only the 7
  known pre-existing failures (5 source drift + 2 Windows symlink).
- Not done: population growth, dirt-from-grass, sand/dirt used by any recipe.

## Latest worktree — roles from real workplaces (2026-11-08)

- `Job::Laborer` (serde `laborer`) is a real role: no farmer without a field,
  no baker without a bakery, no lumberjack/miner without a camp or a saved
  material shortage. `Settlement::workplace_demand()` derives seats from real
  sites; `staff_workplaces()` reassigns stably (camp worker + issued harvest
  worker are pinned) and returns only actual changes.
- `glue::staff_village` runs at the start of `autonomous_village_step`, before
  `auto_spawn_villagers`, saves once per real change and renames the affected
  NPC via `native_merge_entity_nbt` (plain `CustomName` string +
  `CustomNameVisible`). No world edit in staffing.
- Removed two deadlocks: `start_farm` no longer requires an existing farmer and
  `start_bakery` only requires a builder; `need_field`/`need_bakery` in the
  orchestrator no longer wait for workers that could only exist after the site.
- `gather_one_step` now restricts a bound camp to its own worker so the sign
  name and the delivering resident match. `/civ roles NAME` shows
  assigned/workplaces per role plus laborers.
- Gates: `cargo test` 159/159, clippy clean, Python suite back to its known
  baseline (5 pre-existing + 2 Windows symlink errors); new
  `tests/test_workplace_staffing.py` passes.
- Not done: sand/dirt posts need new `Good` variants; job changes have no
  visible confirmation in chat beyond the name tag and `/civ roles`.

## Робочі пости / оркестратор (2026-11-08)
- Додано `WorkPost { kind: Lumber|Quarry, origin, worker_id }` у Settlement (`posts`, serde-default).
- Валідація: ≤4 пости, унікальні місця, 8..80 блоків від центру, worker_id лише живий житель з відповідною професією.
- `World::add_post`, `post_origin`, `set_post_worker` (ідемпотентно; інший worker → помилка).
- Команди: `/civ post NAME lumber|quarry` (OP, стати на природну землю, клітинка має бути порожня), `/civ posts NAME`.
- Оркестратор (`autonomous_village_step`): якщо посту немає, перший знайдений ресурс резервує пост на місці стоянки робітника; далі пошук дерева/каменю обмежений `POST_RADIUS=24` навколо посту.
- `gather_one_step` привʼязує вибраного робітника до посту (той самий UUID-bound житель).
- Знак посту (кожні 400 тіків разом з ратушею): `LUMBER CAMP|QUARRY / <імʼя> / RADIUS 24 / WOOD|STONE`.
- Гейти: `cargo test` **149/149**, clippy чисто. Білд: `native-18840db-5f92db6574f7` (таблички), далі білд з постами.
- НЕ зроблено: перепризначення професій на льоту — імʼя NPC містить професію (`Gwen (miner) [civ:1:1006]`), тому зміна роботи вимагає перейменування фізичного NPC; це наступний крок.

## Далі
- Дочекатися білда → перевірити `native-mods.json` → перезапустити гру (зараз запущена попередня збірка).
- Smoke-тест нових команд: `/civ diagnose`, `/civ gathercheck`, `/civ discardgather`, courier/witness логіка.
