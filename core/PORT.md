# The Rust engine

Python keeps the terminal game (the TUI), the maps and the books. Rust takes the world: its state and every rule
that changes it, year by year.

## Architecture

- `core/` builds the Python extension `realm_of_ages._core` (`bin/build-core`).
- `_core.World` owns everything: the map and climate grids, towns, realms, cultures, characters, houses, monsters,
  ruins, guild, clans, ledgers, the event log, and the random generator (`rng::Pcg64`, NumPy's PCG64).
- Python talks to it through a narrow interface:
  - `World(width, height, seed)`, `world.step(years)`, `world.year`
  - reading: `events(since)`, `realm_rows()`, `econ_rows()`, `character(id)`, `find(query)`, grids for the map
    (`terrain()`, `owner()`, `towns()`), books data
  - god mode: one method per act (`wake_calamity`, `create_character`, `set_stat`, ...)
  - `save(path)` / `World.load(path)` (binary, versioned)
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
| 3 | People: characters, skills, classes, levels, legacies, lineage, the heart, youth, mortality, births | characters, skills, lineage, heart, youth, court (part); brought forward: the dynasty year, opinion | power model (mods, tiers, power gate, combat rating, skills, perks, prestige classes): founding cr 76 vs 81, caster share and tier rarity match. Dynasty year (deaths, births, marriages, regency, plots, succession by every law, epithets, succession wars) and opinion: over 300 years infant deaths 13.5-14% (target 1 in 6; Python 11.3%), under-15 27% (target 25-33%; Python 23%), adult age at death 58 vs 56; 300 years in 1.1 s (Python 61-70 s). Left: the heart, youth, court (education, perks) |
| 4 | Realms: dynasties, succession, partition, councils, titles, factions, governance, civics, statecraft, war | dynasty, titles, governance, civics, statecraft, warfare, opinion | done. Over 300 years on 2 seeds against the full Python world: living realms 36 vs 30 and 31 vs 31, wars declared 233 vs 245 and 277 vs 229, battles 1187 vs 1551 and 1774 vs 1190, sieges 141 vs 100 and 243 vs 238, new realms 107 vs 120 and 115 vs 87, towns 812 vs 736 and 704 vs 640, people within 10%; 300 years in about 2 s (Python about 60 s) |
| 5 | Society: crafts, guilds, crime, faith, stewardship, the Wardens' Hall, bounties | crafts, society, stewardship, guild, bounties | |
| 6 | The fantastic: magic, arcana, monsters, dragons, gates, ecology, dungeons, beasts, bending, shinobi, heroes, legends, the sea | the rest | |
| 7 | Flip: the TUI and the books read from `_core.World`; the Python engine is retired | session, tui, output, godmode | |

Until slice 7 the Python engine stays the game; each slice is built and checked inside the Rust world.

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
