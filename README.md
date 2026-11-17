# Ageforge

**A world that forges its own history.** Ageforge runs *Realm of Ages*: a fantasy world simulation where four
peoples start as scattered tribes and, over centuries, become chiefdoms, kingdoms and empires, with dynasties,
wars, plagues, famines, markets, magic, monsters and legends, none of it scripted. You set the world going, move it
forward a year or a thousand at a time, read everything that happened, and reach in with god mode whenever you like.

```
bin/realm-tui
```

Ageforge is a Rust program. `bin/realm-tui` builds it the first time (it needs a Rust toolchain from
[rustup](https://rustup.rs)) and opens the game.

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

`bin/realm-tui` opens the game. Found a new world (name, seed, size) or open a saved one. Press `?` anywhere for the keys.

| Key | Action |
|---|---|
| `1` to `9` | run 1, 5, 10, 50, 100, 200, 300, 500 or 1000 years (`s` stops at the end of the year) |
| `←` `→`, `F1` to `F10` | switch tabs |
| `g` | god mode: every act on the world, each a form whose choices are searchable lists |
| `c` | create a character |
| `e` | change the selected character (stats, traits, class, items, loves, fate) |
| `t` | track or stop tracking the selected character |
| `/` | search the chronicle or the people |
| `x` | end the world |
| `q` | back to the worlds list (the world is saved) |

**Tabs:** Realms, Chronicle (filter by kind with `f`, major only with `m`), Money (every crown's books and
transactions), Economy (output, debt, poverty, classes and what they own, the richest people), Guild, Wars,
People (search, sheets, the dead with `d`), Tracked (the life story of everyone you track), Map, and Books.

**Creating a character** works like starting one in CK3 or Oblivion: pick a people, sex, age and realm; an origin and
an ambition; a class and level; the six stats, prowess and talent; personality traits, inborn traits and power
bloodlines from checklists that describe each one; and every skill. `p` starts from an archetype (Warrior, Knight,
Ranger, Thief, Wizard, Priest, Ruler-in-waiting, Scholar, Artist, Merchant, Runesmith, Hero of legend), `r` rolls a
name, `R` randomizes everything. Opposite traits cancel each other.

**Lives.** Every child is born with an origin drawn from the world around them (a famine year, a plague, a war, a
ruling house, a hero parent, a temple, a workbench, a merchant's house, a harbour, the poorest street) and, coming of
age, forms an ambition from it: glory, wealth, knowledge, faith, power, love, a quiet home, or vengeance on whoever
orphaned them. The ambition steers what they do, and fulfilling it makes them content. Tracked characters keep their
own life story.

A world never ends until you end it. Each lives in `worlds/<name>/` with the chronicle, the books, maps, and
`meta.json`, which records the seed, every run of years and every act of the gods. A world is deterministic, so
opening one replays that record and arrives at exactly the same world.

## Running without the interface

```
target/release/ageforge run --years 1000 --seed 7 --dir my-run
```

Options: `--years`, `--seed`, `--width`/`--height` (default 480 x 270), `--dir`, `--fresh`. A run writes the
chronicle, the books, a map every century and `calibration.md`, and picks up where it left off when run again on the
same `--dir`.

## Measured against history

Every run writes `calibration.md`, which sets the world's numbers beside the medieval record: infant and child
deaths, age at death, children per woman, deaths in childbirth, pandemics, famines, risings, bankrupt crowns,
forced loans, the share of people on the land, poverty, the classes, and how wealth is held.

## Layout

`core/` is the world: `state.rs` holds it and founds it, `engine.rs` seeds it and runs its years, and each system has
its module (`climate`, `economy`, `stewardship`, `populace`, `dynasty`, `titles`, `civics`, `governance`, `war`, `sea`,
`court`, `heart`, `opinion`, `crafts`, `society`, `power`, `monsters`, `ecology`, `heroes`, `wardens`, `legends`,
`beasts`, `bending`, `shinobi`, `works`, `lives`). `godmode.rs` takes the gods' acts.

`game/` is the game: `ui.rs` the terminal interface, `forms.rs` its forms and searchable lists, `god.rs` god mode, the
creator and the editor, `session.rs` a world in play, `sheet.rs` character sheets and the tabs' tables, `books.rs`
the books, `map.rs` the maps, `events.rs` the chronicle's kinds.

`realm_of_ages/` is the original Python engine, kept as the reference the Rust world was ported from and checked
against (`core/PORT.md` has the method, the numbers and the bugs the port found). The game does not use it.

## License

BSD 3-Clause, the standard permissive license with a non-endorsement clause. In plain words: use, copy, change and
sell Ageforge freely; every copy, source or binary, keeps the copyright line, which names **Jay** and links to
https://github.com/otmof-ops/ageforge; and Jay's name may not be used to endorse a derivative without permission.
Full text in [LICENSE](LICENSE).
