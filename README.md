# Ageforge

**A world that forges its own history.** Ageforge runs *Realm of Ages*: a fantasy world simulation where four
peoples start as scattered tribes and, over centuries, become chiefdoms, kingdoms and empires, with dynasties,
wars, plagues, famines, markets, magic, monsters and legends, none of it scripted. You set the world going, move it
forward a year or a thousand at a time, read everything that happened, and reach in with god mode whenever you like.

```
pip install -r requirements.txt
bin/realm-tui
```

The world itself runs in Rust; `bin/realm-tui` builds it the first time (`bin/build-core`, which needs a Rust
toolchain from [rustup](https://rustup.rs)). Python draws the interface, the maps and the books.

---

## What lives in a world

**People and dynasties.** Thousands of named characters who are born, age, marry, love, grieve, scheme and die.
Royal houses with lineages, claims, heirs, regencies, cadet branches and inherited legacies. Genetic traits and
rare power bloodlines that break the usual limits. Personality traits that drive every decision, with stress,
happiness and health tied together.

**Politics.** Titles from county to empire, factions, succession wars, breakaway realms, vassals who expect a seat
on the council, opinion built from traits, memories and hooks, courts with positions and amenities, legitimacy and
tyranny, governments and laws, culture eras and innovations from tribal times to the late middle ages, casus belli,
tributaries, and great decisions that forge empires.

**War.** Armies that march on the map, sieges, field battles, levies, garrisons and regiments, fleets and naval
battles, colonies across the sea, and pirates with their own careers.

**Art, books, heirlooms and monuments.** Only the gifted make them: the artistic trait is about one birth in seventy and
runs in families. Their paintings, carvings, tapestries, songs, chronicles, epics and treatises record what the world
really lived through, change hands, are copied, carried off as plunder, burned with their towns and found again in
ruins. Treatises teach their trade wherever a copy is kept; an epic carries a hero's name into their line's legacy.
The great leave heirlooms that pass from hand to hand in their house. Rich realms raise monuments to their victories
and their dead, a decade's work paid out of revenue, and the conquered sometimes pull them down.

**An economy that works the way economies do.** Every town has jobs in farming, mining, crafts, trade, the clergy,
service and the arcane; skill comes from a people's gifts, their learning and the masters who live there. Lower,
middle and upper classes own land, houses, stock and coin, and poverty is measured against what bread and a roof
cost. Coin is gold: it comes from mines and trade surpluses, and crowns that overspend borrow, raise taxes, seize the
merchants' gold or default. Trade grows from border markets to coastal shipping to a world market. Rulers manage it
themselves (stewards, buildings, court officers, poor relief), and a realm that is badly run pays for it.

**Climate and weather.** Every region has its own warmth and rain from latitude, height, the sea and the mountains'
rain shadow. Warm and cold ages drift over centuries, eruptions take summers away, wet and dry years come in runs,
locusts and blight follow, soils wear out under too many mouths, and grain moves to the hungry only as far as a
realm's carts and clerks can carry it. Famines come out of all of that.

**Magic, with real limits.** Six schools and tiers from cantrips to the tenth. Most casters never pass the third
tier; the fourth to sixth belong to the rare masters of an age, and no ordinary mortal goes past the seventh. Higher
magic, and workings beyond the tenth tier, belong to characters you create. Rare innate gifts, battle arts for those
who fight without magic, elemental bending as a bloodline art, shinobi clans and their techniques, beast taming
that can end in merging a bloodline with a beast, and a champion who returns when the world needs one.

**Monsters as part of the world.** A real ecosystem of mythic species, monsters that breed and age, dungeons with
bosses, dragons that hoard gold and wield wild magic at the cost of their own lives, and calamities that can doom a
continent. Monsters shadow the fields near their lairs, frighten trade away, drive refugees into crowded towns, and
supply the hides, ash, horn and scale that high scrolls are written on. Hunters are licensed by the Wardens' Hall.

**Legends.** Relics forged in legendary ways, curses born of great deeds and great crimes, and cycles of a cursed
villain who returns each age against a marked hero and a seer.

---

## Playing

`bin/realm-tui` opens the game. Start a new world (name, seed, size) or load a saved one.

| Key | Action |
|---|---|
| `1` `2` `3` `4` | +1, +5, +10, +50 years |
| `5` `6` `7` `8` `9` | +100, +200, +300, +500, +1000 years |
| `g` | God mode |
| `Ctrl+P` / `Ctrl+Q` | Command palette / quit |

**Tabs:** Realms, Chronicle (filter by kind, search), Transactions (every crown's books), Economy (output, debt,
credit, poverty, classes and assets, scrolls, monster stores, the bound dead, the richest people), Wars, People
(search, track, edit), Tracked (your characters' journals), Map, and Books.

**God mode:** wake calamities, loose monsters, send plague, famine or plenty, grant gold, start wars or impose
peace, change governments, raise eras, break realms apart, curse houses, smite cities. Create characters to your own
design (every stat, trait, bloodline, class and level), track them, and change anything about anyone.

A world never ends until you end it. Each lives in `worlds/<name>/` with the chronicle, the books, maps, a journal
for every tracked character, and `meta.json`, which records the seed, every advance and every act of the gods. A
world is deterministic, so loading one replays that record and arrives at exactly the same world.

## Running without the interface

```
python3 -m realm_of_ages --years 1000 --seed 7 --dir my-run
```

Options: `--years`, `--seed`, `--width`/`--height` (default 480 x 270), `--dir`, `--fresh`. A run writes the
chronicle, the books, a map every century, `log.csv` every decade and `calibration.md`, and picks up where it left
off when run again on the same `--dir`.

## Measured against history

Every run writes `calibration.md`, which sets the world's numbers beside the medieval record: infant and child
deaths, age at death, children per woman, deaths in childbirth, pandemics, famines, risings, bankrupt crowns,
forced loans, the share of people on the land, poverty, the classes, and how wealth is held.

## Layout

`core/` is the world, in Rust: `state.rs` holds it and founds it, `world.rs` runs the year and is what Python sees,
and each system has its module (`climate`, `economy`, `stewardship`, `populace`, `dynasty`, `titles`, `civics`,
`governance`, `war`, `sea`, `court`, `heart`, `opinion`, `crafts`, `society`, `power`, `monsters`, `ecology`,
`heroes`, `wardens`, `legends`, `beasts`, `bending`, `shinobi`). `export.rs` hands Python a snapshot and `godmode.rs`
takes the gods' acts. `core/gen_tables.py` generates `tables.rs` from the Python tables, so both read one source.

`realm_of_ages/` is the game: `tui` the interface, `session` a world in play, `bridge` the snapshot as the screens
read it, `godmode` the acts, `output` the books and maps, `events` the chronicle's kinds. The Python simulation
modules (`world.py` and the systems it runs) are the reference engine the Rust world was ported from and checked
against; `core/PORT.md` has the method, the numbers and the bugs the port found.

## License

BSD 3-Clause, the standard permissive license with a non-endorsement clause. In plain words: use, copy, change and
sell Ageforge freely; every copy, source or binary, keeps the copyright line, which names **Jay** and links to
https://github.com/otmof-ops/ageforge; and Jay's name may not be used to endorse a derivative without permission.
Full text in [LICENSE](LICENSE).
