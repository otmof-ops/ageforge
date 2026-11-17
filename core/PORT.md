# The Rust engine

Python keeps the terminal game (the TUI), the maps and the books. Rust takes the world: its state and every rule
that changes it, year by year.

## Architecture

- `core/` builds the Python extension `realm_of_ages._core` (`bin/build-core`).
- `_core.World` owns everything: the map and climate grids, towns, realms, cultures, characters, houses, monsters,
  ruins, guild, clans, ledgers, the event log, and the random generator (`rng::Pcg64`, NumPy's PCG64).
- Python talks to it through a narrow interface:
  - `World(width, height, state, inc)`: the generator starts where `np.random.default_rng(seed)` starts
  - `step(years)` (the interpreter lock is released while it runs), `year`, `totals()`, `n_characters()`
  - `snapshot(since)` and `grids()`: the whole world as plain dicts and arrays, which `realm_of_ages/bridge.py`
    turns into objects with the Python engine's names, so the TUI, books and maps read it unchanged
  - `events_from(index)`: the chronicle since an index
  - `god(act, args)`: every act of the gods, one dispatcher (`godmode.rs`)
- Saving is a record, not an image: `meta.json` keeps the seed, each advance and each act in order, and loading
  replays it. The Rust world is deterministic, so the replay is exact.
- Data lives in plain structs in vectors indexed by id (`Vec<Town>`, `Vec<Character>`, `Vec<Realm>`), with optional
  per-system data in its own vectors, so a yearly pass is a loop over packed memory.

## Validation

A Python world and a Rust world cannot match event for event: the Python rules iterate sets of strings, whose order
changes between runs. Each slice is instead checked against the Python engine over several seeds on:

- the calibration table (infant and child deaths, age at death, children per woman, deaths in childbirth,
  pandemics, famines, risings, bankruptcies, forced loans, share on the land, poverty, classes, wealth held)
- event counts by kind per century; realm, town and population counts; tier rarity; dragon kills; guild seals
- run time

A slice is done when its numbers sit inside the Python engine's own seed-to-seed spread.

## Slices, in order

| # | Slice | Python modules | Status |
|---|---|---|---|
| 0 | Foundation: PCG64, territory | world (part) | done |
| 1 | Tables and world generation: map, landmasses, resources, first tribes and towns | data, data2, geography, resources, world (init) | done. 8 seeds: map, resources, ruins, 32 tribes with capitals, rulers, kin, houses, bloodlines, territory all inside Python's spread; 5x faster. Clans and bending tribes (about 30 more people at the start) come with slice 6 |
| 2 | The land's year: climate, harvest, grain, soil, births and deaths of towns, labour, classes, money, trade | climate, economy, markets, populace (part); brought forward: stewardship, the treasury (civics.armies_year), the council (dynasty.council) | climate done (64 runs within 1 se). The land, labour, classes, tax, treasury, council, trade, money and stewardship run in `World.step`: 200 years in 0.4 s (Python's full year: 35 s). Against the full Python world over 3 seeds: towns and people within 5-20%, poverty 0.12 vs 0.11 at year 50 and 0.28-0.39 vs 0.24-0.34 at year 200. Populace (moods, risings, plague): pandemics and big outbreaks match (1-2 per 300 years in both); risings fewer (0-1 vs 1-6) until their causes land (wars, faith, lords: slice 4). 300 years in 0.8 s (Python 70-90 s) |
| 3 | People: characters, skills, classes, levels, legacies, lineage, the heart, youth, mortality, births | characters, skills, lineage, heart, youth, court; brought forward: the dynasty year, opinion, crafts | done. Over 300 years: lovers 325 vs 327, love marriages 206 vs 198, scandals 36 vs 29, sworn vengeance 19 vs 18, coping habits 244 vs 375; royal courts, masterworks and vengeance wars fewer until soulstones, heroes and assassins land (slices 5 and 6) |
| 4 | Realms: dynasties, succession, partition, councils, titles, factions, governance, civics, statecraft, war | dynasty, titles, governance, civics, statecraft, warfare, opinion | done. Over 300 years on 2 seeds against the full Python world: living realms 36 vs 30 and 31 vs 31, wars declared 233 vs 245 and 277 vs 229, battles 1187 vs 1551 and 1774 vs 1190, sieges 141 vs 100 and 243 vs 238, new realms 107 vs 120 and 115 vs 87, towns 812 vs 736 and 704 vs 640, people within 10%; 300 years in about 2 s (Python about 60 s) |
| 5 | Society: crafts, guilds, crime, faith, stewardship, the Wardens' Hall, bounties | crafts, society, stewardship, guild, bounties | done (crafts in slice 3, stewardship in slice 2). Over 300 years on 2 seeds: quests done 1035 vs 1293 and 1274 vs 1029, seal raises 78 vs 102 and 97 vs 65, parties 37 vs 56 and 39 vs 46, great bounties 15 vs 17 and 14 vs 20 |
| 6 | The fantastic: magic, arcana, monsters, dragons, gates, ecology, dungeons, beasts, bending, shinobi, heroes, legends, the sea | the rest | done. Over 300 years on 2 seeds: Concord returns 5 vs 5 (both), sightings 93 vs 94, sea monsters 24 vs 20 and 22 vs 21, naval battles 472 vs 402 and 314 vs 340, pirates 75 vs 91 and 94 vs 110, dragon tributes 19 vs 19, wild magic 3 vs 3; 300 years in 6.5 s (Python 34-38 s) before the speed pass |
| 7 | Flip: the TUI and the books read from `_core.World`; the Python engine is retired | session, tui, output, godmode | done. The game, the books, god mode and `python3 -m realm_of_ages` run on the Rust world. Every tab, all 28 acts of the gods, and save and reload (a replay that rebuilds the identical world) checked; the books over 6 seeds at 300 years; 300 years with the books in 10-20 s |

The Python engine stays in the tree as the reference the port is checked against; the game no longer runs it.

| 8 | The game in Rust: the terminal interface, god mode, the character creator and editor, the books, the maps and saving (`game/`), with no Python at all | tui, session, bridge, godmode, output | done. NumPy's seed sequence ported (`engine::seed_state`, checked on 8 seeds), so every seed grows the same world as before; the pure Rust world and the Python extension give the same 300 years event for event. 300 years with every book and three maps in 4.5 s |

## Speed in long worlds

A world grows: seed 4 reaches 1,700 towns by year 400 and 230,000 characters (living and dead) by year 1000.
`World.phase_times()` gives the seconds spent in each phase of the year, so the slow part is measured, not guessed.

- `State.living` holds the ids of the living in id order, rebuilt each year and appended at each birth. Scans that
  walked every character ever born, many once per realm, walk it instead. The order is unchanged, so the world is
  unchanged: the chronicle of 300 years hashes identically before and after. Check any change like this the same way.
- The sea works out each realm's ports once per pass, not once per realm pair or per monster per realm.
- The snapshot sends the forgotten dead as shades; a shade fetches its full record (`World.character_full`) the
  first time a screen asks for more than it holds.

On seed 4: years 450-500 took 31 s and take 10 s; the mirror at year 700 took 13 s and 8.4 GB and takes 5 s and
2.3 GB; the dynasties book took 26 s and takes 0.5 s.

## Built in Rust only

Systems added after the port live only in the Rust world; the Python engine has no counterpart to check them against.

- `works.rs`: art, books, heirlooms and monuments, made only by the `artistic` (a new ordinary gene, 1.5% of births).
- Housing and food strain (`economy.rs`): houses decay and are built only as fast as a town's builders, wealth and
  timber or stone allow; crowding lowers births, raises deaths and drives people out; a harvest loses its seed grain
  (yields of three to four and a half to one, by field rotation) and the share eaten by beasts, rats and mould before
  anyone eats, and fertility follows what is eaten. Over 400 years on 3 seeds the world ends about half as populous,
  with famines in their historical range (8-11 killing over 1,000).

## Python bugs found while porting

Kept in the Rust engine for now so the two can be compared; to be fixed in both once the port is checked.

- **Starting relations are indexed by realm, not by people** (`world.Realm.__init__`): `relation` is seeded
  from `BASE_REL` (people against people) as a 4x4 table, but every table of war and peace is indexed by realm
  id. So realms 0-3 start with the Human/Elf/Dwarf/Orc relations whatever their peoples are, and every later
  tribe starts at 0 with everyone. The intent is plainly per people: seed `relation[a][b]` from
  `BASE_REL[people(a)][people(b)]` for every pair of realms.
- **Fixed:** `dynasty.dynasty_year` crashed (`KeyError: None` in `council`) when a ruler died in a plot and the
  crown passed to someone already ruling elsewhere: the second `succeed` call lacked the extinct check the first
  one has. Found on seed 3 near year 200.
- **Diplomacy pairs every realm only with realms 0-3** (`world.Realm.diplomacy`): the inner loop is
  `for j in range(i + 1, 4)`, so relations drift, border wars break out and wars end by weariness only between the
  first four realm ids and the rest; every other pair reaches war only through `statecraft.ambitions_year`. It
  should be `range(i + 1, self.npol)`.
- **Dead code in factions** (`titles.factions_year`): the faction's strength is scaled by its leader's martial
  skill and by how long its grievances have hardened, but only after `ratio` is computed, and `fp` is never read
  again, so neither has any effect. Either scale before the ratio or remove the lines.
