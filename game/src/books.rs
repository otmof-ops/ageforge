//! The books of the world: the Markdown the realm's chronicle is read from, written straight from the world's state.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use world::people::{Character, NONE};
use world::power::level_of;
use world::state::State;
use world::tables::*;
use world::works::sector_name;

const SOVEREIGN: &str = "Wyrm-Sovereign";
const ACTIONS: [&str; 9] = ["rest", "train", "hunt", "delve", "war", "guard", "study", "serve", "bounty"];
const SHIP_NAMES: [&str; 5] = ["Cog", "Caravel", "Carrack", "Galleon", "Ship of the Line"];
const AXIS_NAMES: [(&str, &str); 5] = [("absolutist", "councilist"), ("hereditary", "elective"), ("theocratic", "secular"),
                                       ("mercantile", "agrarian"), ("traditionalist", "reformist")];
const ROLES: [&str; 6] = ["marshal", "steward", "spymaster", "mage", "loremaster", "admiral"];

// ---------------------------------------------------------------- small helpers

/// An integer with thousands separators.
fn int(x: i64) -> String {
    let s = x.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 { out.push(','); }
        out.push(ch);
    }
    if x < 0 { format!("-{out}") } else { out }
}

/// A float rounded to a whole number, with thousands separators.
fn nf(x: f64) -> String { int(x.round() as i64) }

fn pct(x: f64) -> String { format!("{:.0}%", 100.0 * x) }

fn clip<T: Copy>(v: &[T], i: usize) -> T { v[i.min(v.len() - 1)] }

fn tier_word(t: i64) -> &'static str { TIER_WORDS[t.clamp(0, 10) as usize] }

fn full_name(c: &Character) -> String {
    let mut n = c.name.clone();
    if let Some(r) = &c.regnal { n.push(' '); n.push_str(r); }
    if let Some(e) = &c.epithet { n.push(' '); n.push_str(e); }
    n
}

fn title_of(c: &Character) -> &'static str {
    if c.female { RULER_TITLE_F[c.people.min(3)] } else { RULER_TITLE_M[c.people.min(3)] }
}

/// A character's full name, or a plain word when the id points at no one.
fn nm(st: &State, id: usize) -> String {
    if id != NONE && id < st.chars.len() { full_name(&st.chars[id]) } else { "no one".to_string() }
}

fn nm_or(st: &State, id: usize, other: &str) -> String {
    if id != NONE && id < st.chars.len() { full_name(&st.chars[id]) } else { other.to_string() }
}

/// How the chronicle names a person.
fn who(st: &State, id: usize) -> String {
    if id == NONE || id >= st.chars.len() { return "no one".to_string(); }
    let c = &st.chars[id];
    if let Some((_, _, None)) = c.ruled { return format!("{} {}", title_of(c), full_name(c)); }
    let sing = SINGULAR[c.people.min(3)];
    if c.concord { return format!("{} ({sing}, the Concord)", full_name(c)); }
    if c.clan != NONE { return format!("{} ({sing} shinobi, {})", full_name(c), clip(&SHINOBI_RANKS, c.srank)); }
    if let Some(e) = c.element { return format!("{} ({sing} {e}-bender)", full_name(c)); }
    if let Some(p) = c.profession { return format!("{} ({sing} {p})", full_name(c)); }
    match c.classes.last() {
        Some(&(k, lv)) => {
            let k = if lv > 0 || c.classes.len() == 1 { k } else { c.classes[c.classes.len() - 2].0 };
            format!("{} ({sing} {} {})", full_name(c), CLASSES[k].name, c.level)
        }
        None => format!("{} ({sing})", full_name(c)),
    }
}

fn realm_name(st: &State, k: usize) -> &str { st.realms.get(k).map(|r| r.name.as_str()).unwrap_or("an unknown realm") }
fn realm_short(st: &State, k: usize) -> &str { st.realms.get(k).map(|r| r.short.as_str()).unwrap_or("an unknown realm") }
fn town_name(st: &State, t: usize) -> &str { st.towns.get(t).map(|x| x.name.as_str()).unwrap_or("an unknown town") }
fn land_name(st: &State, l: i64) -> &str {
    if l >= 0 { st.map.lands.get(l as usize).map(|d| d.name.as_str()).unwrap_or("?") } else { "?" }
}
fn land_kind(st: &State, l: i64) -> &str {
    if l >= 0 { st.map.lands.get(l as usize).map(|d| d.kind).unwrap_or("?") } else { "?" }
}

fn alive_towns(st: &State, k: usize) -> Vec<usize> { st.alive_towns_of(k) }

fn pop_of(st: &State, k: usize) -> f64 { alive_towns(st, k).iter().map(|&t| st.towns[t].pop).sum() }

fn classes_of(c: &Character) -> String {
    c.classes.iter().filter(|x| x.1 > 0).map(|x| CLASSES[x.0].name).collect::<Vec<_>>().join(", ")
}

fn is_notable(c: &Character) -> bool { c.hero || c.renown > 0.0 || c.ruled.is_some() }

fn fate(c: &Character) -> String {
    if c.alive { "living".to_string() } else { c.cause.clone().unwrap_or_else(|| "unknown".to_string()) }
}

/// What a trend on the five axes reads as.
fn ideology_label(v: &[f64]) -> String {
    let mut out = vec![];
    for (a, names) in AXIS_NAMES.iter().enumerate() {
        if v[a].abs() >= 0.35 { out.push(if v[a] > 0.0 { names.0 } else { names.1 }); }
    }
    if out.is_empty() { "moderate".to_string() } else { out.join(", ") }
}

fn court_expected(rank: &str) -> i64 {
    match rank { "empire" => 50, "hegemony" => 70, _ => 30 }
}

/// Realms that broke from another: (year, new realm, old realm, the one who led the break).
fn breakaways(st: &State) -> Vec<(i64, usize, usize, usize)> {
    let mut v: Vec<(i64, usize, usize, usize)> = st.breakaways.clone();
    for (k, rm) in st.realms.iter().enumerate() {
        if rm.parent != NONE && !v.iter().any(|b| b.1 == k) { v.push((rm.founded_year, k, rm.parent, rm.reigns.first().map(|r| r.id).unwrap_or(NONE))); }
    }
    v.sort();
    v
}

fn write(dir: &Path, name: &str, lines: &[String]) -> std::io::Result<()> {
    let mut text = lines.join("\n");
    text.push('\n');
    fs::write(dir.join(name), text)
}

fn tier_of_char(st: &State, c: &Character) -> i64 { st.tier(c) }

fn living_everyone(st: &State) -> Vec<&Character> { st.chars.iter().filter(|c| is_notable(c)).collect() }

// ---------------------------------------------------------------- entry point

/// Write every book into `dir` (summary.md, realms.md, dynasties.md, legends.md, myths.md, world.md, magic.md, seas.md,
/// pirates.md, society.md, works.md, dungeons.md, bestiary.md, calibration.md).
pub fn write_all(st: &State, dir: &Path, seed: u64) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    write(dir, "summary.md", &summary(st, seed))?;
    write(dir, "dynasties.md", &dynasties(st))?;
    write(dir, "legends.md", &legends(st, 16))?;
    write(dir, "magic.md", &magic(st))?;
    write(dir, "bestiary.md", &bestiary(st))?;
    write(dir, "world.md", &world_book(st))?;
    write(dir, "myths.md", &myths_book(st))?;
    write(dir, "dungeons.md", &dungeons_book(st))?;
    write(dir, "seas.md", &seas_book(st))?;
    write(dir, "pirates.md", &pirates_book(st))?;
    write(dir, "society.md", &society_book(st))?;
    write(dir, "works.md", &works_book(st))?;
    write(dir, "realms.md", &realms_book(st))?;
    write(dir, "calibration.md", &calibration(st))?;
    Ok(())
}

// ---------------------------------------------------------------- summary

fn summary(st: &State, seed: u64) -> Vec<String> {
    let mut l: Vec<String> = vec![
        format!("# Realm of Ages, year {}", st.year), String::new(),
        format!("Seed {seed}, a {} by {} world. Highest tier of magic ever cast: **{}**.", st.map.w, st.map.h, tier_word(st.world_tier)),
        String::new(), "## The realms".into(), String::new(),
        "| People | Realm | Ruler | House | Law | Legitimacy | Tyranny | Corruption | Towns | Population | Allies |".into(),
        "|---|---|---|---|---|---|---|---|---|---|---|".into()];
    let mut order: Vec<usize> = (0..st.realms.len()).collect();
    order.sort_by_key(|&k| (st.realms[k].extinct, std::cmp::Reverse(alive_towns(st, k).len())));
    for k in order {
        let rm = &st.realms[k];
        if rm.extinct || rm.ruler == NONE || rm.ruler >= st.chars.len() { continue; }
        let towns = alive_towns(st, k);
        let r = &st.chars[rm.ruler];
        let ruler = format!("{} {}, age {}, level {}", title_of(r), full_name(r), r.age(st.year), r.level);
        let broke = if rm.parent != NONE { format!(", broke from {} in {}", realm_name(st, rm.parent), rm.founded_year) } else { String::new() };
        let house = if rm.house != NONE && rm.house < st.houses.len() { st.houses[rm.house].name.clone() } else { "-".into() };
        let corrupt = if rm.corrupt_seats.is_empty() { String::new() } else {
            format!(" ({} corrupt)", rm.corrupt_seats.iter().map(|&i| ROLES[i.min(5)]).collect::<Vec<_>>().join(", "))
        };
        let mut allies = rm.allies.clone();
        allies.sort();
        let allies = allies.iter().map(|&a| realm_short(st, a).to_string()).collect::<Vec<_>>().join(", ");
        l.push(format!("| {} | {} ({}{broke}) | {ruler} | {house} | {} | {:.0} | {:.0} | {:.0}%{corrupt} | {} | {} | {} |",
                       PEOPLES[rm.people.min(3)], rm.name, rm.rank, rm.law, r.legitimacy.unwrap_or(0.0), r.tyranny, 100.0 * rm.corruption,
                       towns.len(), nf(pop_of(st, k)), if allies.is_empty() { "-".to_string() } else { allies }));
    }
    let mut wars = vec![];
    for i in 0..st.realms.len() {
        for j in i + 1..st.realms.len() {
            if st.war.get(i).and_then(|r| r.get(j)).copied().unwrap_or(false) {
                let since = st.war_since.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0);
                wars.push(format!("{} against {} (since {since})", st.realms[i].name, st.realms[j].name));
            }
        }
    }
    l.extend(["".into(), "## Realms born of breakaway".into(), "".into()]);
    let brk = breakaways(st);
    if brk.is_empty() { l.push("- none yet".into()); }
    for &(y, n, o, c) in &brk {
        l.push(format!("- Year {y}: {} broke from {} under {}", realm_name(st, n), realm_name(st, o), nm(st, c)));
    }
    l.extend(["".into(), "## Factions now".into(), "".into()]);
    for k in &st.realms {
        for fa in &k.factions {
            l.push(format!("- In {}: a {} faction of {} dukes, led by {}, at {:.0}% of the crown's strength",
                           k.name, fa.0, fa.2.len(), nm(st, fa.3), 100.0 * fa.4));
        }
    }
    l.extend(["".into(), format!("**Wars now:** {}.", if wars.is_empty() { "none".to_string() } else { wars.join(", ") }), "".into(),
              "## Monsters abroad".into(), "".into()]);
    let mut any = false;
    for m in st.monsters.iter().filter(|m| m.alive) {
        any = true;
        let hoard = if m.hoard > 1.0 { format!(", hoard {}", nf(m.hoard)) } else { String::new() };
        l.push(format!("- {} ({}, strength {}, since year {}{hoard})", m.name, m.kind, nf(m.power), m.born));
    }
    if !any { l.push("- none".into()); }
    l.extend(["".into(), "## Calamities".into(), "".into()]);
    if st.calamity_log.is_empty() { l.push("- none have ended".into()); }
    for (n, a, b, how) in &st.calamity_log { l.push(format!("- {n}: woke in year {a}, ended in year {b}, {how}")); }
    l.extend(["".into(), "## Worldforged relics".into(), "".into()]);
    for (name, _, note) in WORLDFORGED.iter() {
        let mut holder = "never found".to_string();
        if let Some(ix) = st.items.iter().position(|it| it.name == *name) {
            holder = "lost".to_string();
            for c in st.chars.iter().filter(|c| c.alive && c.items.contains(&ix)) { holder = format!("borne by {}", who(st, c.id)); }
            for k in &st.realms { if k.treasury.contains(&ix) { holder = format!("in the treasury of {}", k.name); } }
            for r in &st.ruins { if r.loot.contains(&ix) { holder = format!("lost in {}", r.name); } }
        }
        l.push(format!("- **{name}** ({note}): {holder}"));
    }
    let mut top = living_everyone(st);
    top.sort_by(|a, b| b.renown.partial_cmp(&a.renown).unwrap_or(std::cmp::Ordering::Equal));
    l.extend(["".into(), "## Hall of legends (top 12 by renown)".into(), "".into(),
              "| Name | People | Level | Classes | Tier | Renown | Fate |".into(), "|---|---|---|---|---|---|---|".into()]);
    for h in top.iter().take(12) {
        let t = tier_of_char(st, h);
        l.push(format!("| {} | {} | {} | {} | {} | {} | {} |", full_name(h), PEOPLES[h.people.min(3)], h.level, classes_of(h),
                       if t >= 0 { t.to_string() } else { "-".into() }, nf(h.renown), fate(h)));
    }
    l.extend(["".into(), "## What the guilds learned".into(), "".into(),
              "Each guild's hunts and deeds over the whole history of the world.".into(), "".into(),
              format!("| Guild | Hunt win rate | Heroes | Deaths | Deeds ({}) |", ACTIONS.join("/")), "|---|---|---|---|---|".into()]);
    for (f, g) in st.hero_guilds.iter().enumerate().take(4) {
        let total: u32 = g.acts.iter().sum();
        let mix = g.acts.iter().map(|&a| format!("{:.0}", 100.0 * a as f64 / total.max(1) as f64)).collect::<Vec<_>>().join("/");
        l.push(format!("| {} | {}/{} ({:.0}%) | {} | {} | {mix} |", PEOPLES[f], g.hunt_wins, g.hunts,
                       100.0 * g.hunt_wins as f64 / g.hunts.max(1) as f64, g.heroes_total, g.deaths_total));
    }
    l.extend(["".into(), "**Guild doctrines** (the skill branches each guild has learned to favour, by mean reward per hero-year):".into(), "".into()]);
    for (f, g) in st.hero_guilds.iter().enumerate().take(4) {
        let mut ranked: Vec<((usize, usize), f64)> = g.branch_val.iter().filter(|(_, v)| v.1 >= 20).map(|(k, v)| (*k, v.0)).collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
        let doc = ranked.iter().take(3).map(|((c, b), v)| format!("{} / {} ({v:.2})", CLASSES[*c].name, CLASSES[*c].branches[*b].0)).collect::<Vec<_>>();
        l.push(format!("- {}: {}", PEOPLES[f], if doc.is_empty() { "not settled yet".to_string() } else { doc.join(", ") }));
    }
    l
}

// ---------------------------------------------------------------- dynasties

fn dynasties(st: &State) -> Vec<String> {
    let mut l: Vec<String> = vec!["# The royal houses".into(), "".into(), "## Dynasties and their branches".into(), "".into()];
    let mut hs: Vec<&world::people::House> = st.houses.iter().collect();
    hs.sort_by_key(|h| std::cmp::Reverse(h.reign_years));
    for h in hs.iter().take(12) {
        if h.rulers == 0 { continue; }
        l.push(format!("- **The dynasty of {}** ({} rulers, {} years on thrones, prestige {})", h.name, h.rulers, h.reign_years, nf(h.prestige)));
    }
    l.push(String::new());
    for k in &st.realms {
        let f = k.people.min(3);
        let broke = if k.parent != NONE { format!(" Broke from {} in year {}.", realm_name(st, k.parent), k.founded_year) } else { String::new() };
        l.extend([format!("## {}{}", k.name, if k.extinct { " (no more)" } else { "" }), "".into(),
                  format!("People: {}. Rank: {}.{broke} Succession law: **{}**. Succession wars: {}.", PEOPLES[f], k.rank, k.law, k.civil_wars),
                  "".into(), "| # | Ruler | House | Reign | Years | Legitimacy | Tyranny | How it ended |".into(),
                  "|---|---|---|---|---|---|---|---|".into()]);
        let mut ruled: BTreeSet<usize> = BTreeSet::new();
        for (i, r) in k.reigns.iter().enumerate() {
            if r.id >= st.chars.len() { continue; }
            let c = &st.chars[r.id];
            let end = r.end.map(|e| e.to_string()).unwrap_or_else(|| "...".into());
            let yrs = r.end.unwrap_or(st.year) - r.start;
            let house = if c.house != NONE && c.house < st.houses.len() { ruled.insert(c.house); st.houses[c.house].name.clone() } else { "-".into() };
            l.push(format!("| {} | {} {} | {house} | {}-{end} | {yrs} | {:.0} | {:.0} | {} |", i + 1, title_of(c), full_name(c), r.start,
                           r.legit.or(c.legitimacy).unwrap_or(0.0), r.tyranny, r.how.clone().unwrap_or_else(|| "reigning".into())));
        }
        let mut hs: Vec<&world::people::House> = ruled.iter().map(|&h| &st.houses[h]).collect();
        hs.sort_by_key(|h| std::cmp::Reverse(h.reign_years));
        l.extend(["".into(), "| House | Origin | Legacy | Rulers | Years on the throne | Prestige |".into(), "|---|---|---|---|---|---|".into()]);
        for h in &hs {
            let lv = level_of(h.legacy, &HOUSE_LEGACY_LEVELS) as usize;
            let kind = LEGACY_KINDS.iter().position(|x| *x == h.track).map(|i| LEGACY_NAMES[i]).unwrap_or("-");
            l.push(format!("| {} | {} | {} {} | {} | {} | {} |", h.name, h.origin, kind,
                           if lv > 0 { ROMAN[lv.min(10)] } else { "(unranked)" }, h.rulers, h.reign_years, nf(h.prestige)));
        }
        if let Some(h0) = hs.first() {
            if h0.founder != NONE && h0.founder < st.chars.len() {
                l.extend(["".into(), format!("### Family tree of {} (rulers in bold, crowned years in brackets)", h0.name), "".into()]);
                l.extend(tree(st, h0.founder, k, 8));
            }
        }
        l.push(String::new());
    }
    l
}

fn has_ruler(st: &State, c: usize, depth: usize, max_depth: usize, rulers: &BTreeSet<usize>, memo: &mut HashMap<(usize, usize), bool>) -> bool {
    if let Some(&v) = memo.get(&(c, depth)) { return v; }
    let mut v = rulers.contains(&c);
    if !v && depth < max_depth {
        for &x in &st.chars[c].children {
            if x < st.chars.len() && has_ruler(st, x, depth + 1, max_depth, rulers, memo) { v = true; break; }
        }
    }
    memo.insert((c, depth), v);
    v
}

fn tree(st: &State, root: usize, k: &world::people::Realm, max_depth: usize) -> Vec<String> {
    let rulers: BTreeSet<usize> = k.reigns.iter().map(|r| r.id).collect();
    let reign: HashMap<usize, (i64, Option<i64>)> = k.reigns.iter().map(|r| (r.id, (r.start, r.end))).collect();
    let mut memo = HashMap::new();
    let mut out = vec![];
    walk(st, root, 0, max_depth, &rulers, &reign, &mut memo, &mut out);
    out
}

#[allow(clippy::too_many_arguments)]
fn walk(st: &State, id: usize, depth: usize, max_depth: usize, rulers: &BTreeSet<usize>, reign: &HashMap<usize, (i64, Option<i64>)>,
        memo: &mut HashMap<(usize, usize), bool>, out: &mut Vec<String>) {
    if depth > max_depth { return; }
    let c = &st.chars[id];
    let mut tag = full_name(c);
    if let Some(&(s, e)) = reign.get(&id) {
        tag = format!("**{tag}** [{s}-{}]", e.map(|e| e.to_string()).unwrap_or_default());
    }
    let life = format!("{}-{}", c.born, if c.alive { String::new() } else { c.died.map(|d| d.to_string()).unwrap_or_default() });
    let sp = if c.spouse != NONE && c.spouse < st.chars.len() { Some(full_name(&st.chars[c.spouse])) } else { None };
    let mut extra = vec![];
    if c.hero || c.renown >= 100.0 { extra.push(format!("renown {}", nf(c.renown))); }
    let mut rare: Vec<&str> = c.genetic.iter().map(|&g| GENETIC[g]).filter(|t| ["dragonblood", "starblood", "genius", "spellborn"].contains(t) || t.starts_with("half-")).collect();
    rare.sort();
    if !rare.is_empty() { extra.push(rare.join(", ")); }
    out.push(format!("{}- {tag} ({life}){}{}", "  ".repeat(depth), sp.map(|s| format!(" m. {s}")).unwrap_or_default(),
                     if extra.is_empty() { String::new() } else { format!("; {}", extra.join("; ")) }));
    let mut kids: Vec<usize> = c.children.iter().copied().filter(|&x| x < st.chars.len()).collect();
    kids.sort_by_key(|&x| st.chars[x].born);
    for kid in kids {
        if depth < 1 || has_ruler(st, kid, depth + 1, max_depth, rulers, memo) {
            walk(st, kid, depth + 1, max_depth, rulers, reign, memo, out);
        }
    }
}

// ---------------------------------------------------------------- legends, magic, bestiary

fn legends(st: &State, n: usize) -> Vec<String> {
    let mut top = living_everyone(st);
    top.sort_by(|a, b| (b.renown + b.peak_cr / 50.0).partial_cmp(&(a.renown + a.peak_cr / 50.0)).unwrap_or(std::cmp::Ordering::Equal));
    let mut l = vec!["# Legends of the realm".to_string(), "".into(), format!("The {n} greatest names of {} years.", st.year), "".into()];
    for (i, c) in top.iter().take(n).enumerate() {
        let t = tier_of_char(st, c);
        let house = if c.house != NONE && c.house < st.houses.len() { st.houses[c.house].name.as_str() } else { "no house" };
        l.push(format!("{}. **{}** ({} {}, {house}), level {}, {}; tier {}; renown {}; born {}; {}.", i + 1, full_name(c),
                       SINGULAR[c.people.min(3)], if c.female { "woman" } else { "man" }, c.level,
                       { let k = classes_of(c); if k.is_empty() { "no class".to_string() } else { k } },
                       if t >= 0 { t.to_string() } else { "-".into() }, nf(c.renown), c.born, fate(c)));
    }
    // and each of them in full, as their sheets read
    l.push(String::new());
    for c in top.iter().take(n) {
        l.push(crate::sheet::sheet(st, c.id));
        l.push(String::new());
    }
    l
}

fn magic(st: &State) -> Vec<String> {
    let mut l: Vec<String> = vec!["# Magic".into(), "".into(),
        "Tiers run from cantrips to the tenth. Talent caps how high anyone can climb; most never pass the third. The World Rites lie beyond the tiers.".into(),
        "".into(), "## The tier records".into(), "".into(), "| Year | Caster | Tier | Spell |".into(), "|---|---|---|---|".into()];
    for &(y, cid, t, spell) in &st.tier_events { l.push(format!("| {y} | {} | {} | {spell} |", who(st, cid), tier_word(t))); }
    l.extend(["".into(), "## World Rites spoken".into(), "".into()]);
    if st.rites.is_empty() { l.push("- none".into()); }
    for &(y, cid, rite) in &st.rites { l.push(format!("- Year {y}: {}, {rite}", who(st, cid))); }
    l.extend(["".into(), "## Power genetics now living".into(), "".into()]);
    let mut any = false;
    for &g in BLOODLINES.iter() {
        let n = st.chars.iter().filter(|c| c.alive && c.genetic.contains(&g)).count();
        if n > 0 { any = true; l.push(format!("- {}: {n} living bearers", GENETIC[g])); }
    }
    if !any { l.push("- none".into()); }
    l.extend(["".into(), "## Highest tier reached among each people".into(), "".into(),
              format!("| Year | {} |", PEOPLES.join(" | ")), "|---|---|---|---|---|".into(),
              format!("| {} | {} |", st.year, st.people_tier.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" | ")),
              "".into(), "## The spell ladders".into(), "".into()]);
    for (s, name) in SCHOOLS.iter().enumerate() {
        l.push(format!("- **{name}:** {}", (0..11).map(|t| format!("{}: {}", TIER_WORDS[t], SPELLS[s][t])).collect::<Vec<_>>().join("; ")));
    }
    let mut casters: Vec<(&Character, i64)> = st.chars.iter().filter(|c| c.alive).map(|c| (c, st.tier(c))).filter(|x| x.1 >= 0).collect();
    casters.sort_by_key(|x| std::cmp::Reverse(x.1));
    l.extend(["".into(), "## The strongest casters living".into(), "".into()]);
    for (c, t) in casters.iter().take(10) {
        let school = st.school(c).map(|s| SCHOOLS[s]).unwrap_or("-");
        l.push(format!("- {}: {school}, {} tier, talent {:.1}", who(st, c.id), tier_word(*t), c.talent));
    }
    l
}

fn bestiary(st: &State) -> Vec<String> {
    let mut l: Vec<String> = vec!["# Bestiary".into(), "".into(), "Monsters age, breed and die. Hoards, relics and titles pass to their young.".into(), "".into(),
        format!("## The line of the {SOVEREIGN}"), "".into()];
    if st.sovereign_line.is_empty() { l.push("- never held".into()); }
    for (y, name, how) in &st.sovereign_line { l.push(format!("- Year {y}: {name}, {how}")); }
    l.extend(["".into(), "## Living monsters".into(), "".into(),
              "| Name | Kind | Age | Generation | Strength | Traits | Title | Heroes slain | Hoard | Relics |".into(), "|---|---|---|---|---|---|---|---|---|---|".into()]);
    let mut ms: Vec<&world::people::Monster> = st.monsters.iter().filter(|m| m.alive).collect();
    ms.sort_by(|a, b| b.power.partial_cmp(&a.power).unwrap_or(std::cmp::Ordering::Equal));
    for m in ms {
        let mut traits = m.traits.clone();
        traits.sort();
        let relics = m.items.iter().filter(|&&i| i < st.items.len()).map(|&i| st.items[i].label()).collect::<Vec<_>>().join(", ");
        l.push(format!("| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |", m.name, m.kind, st.year - m.born, m.generation, nf(m.power),
                       if traits.is_empty() { "-".to_string() } else { traits.join(", ") }, m.title.unwrap_or("-"), m.kills, nf(m.hoard),
                       if relics.is_empty() { "-".to_string() } else { relics }));
    }
    l
}

// ---------------------------------------------------------------- the deeper books

fn world_book(st: &State) -> Vec<String> {
    let mut l: Vec<String> = vec!["# The world and its life".into(), "".into(), "## Lands".into(), "".into(),
        "| Land | Kind | Leagues | Towns by people | First found by |".into(), "|---|---|---|---|---|".into()];
    let mut lands: Vec<&world::geography::Land> = st.map.lands.iter().collect();
    lands.sort_by_key(|d| std::cmp::Reverse(d.size));
    for d in lands {
        if d.kind == "isle" && d.size < 150 { continue; }
        let mut held: Vec<(String, usize)> = vec![];
        for t in st.towns.iter().filter(|t| t.alive && t.landmass == d.id as i32) {
            let nm = realm_short(st, t.realm).to_string();
            match held.iter_mut().find(|x| x.0 == nm) { Some(x) => x.1 += 1, None => held.push((nm, 1)) }
        }
        let mut found: Vec<(i64, i64)> = vec![];
        for &(f, y) in &d.found_by { match found.iter_mut().find(|x| x.0 == f) { Some(x) => x.1 = y, None => found.push((f, y)) } }
        let fb = found.iter().map(|&(f, y)| format!("{} ({y})", if f >= 0 { realm_short(st, f as usize) } else { "pirates" })).collect::<Vec<_>>().join(", ");
        l.push(format!("| {} | {} | {} | {} | {} |", d.name, d.kind, int(d.size as i64),
                       if held.is_empty() { "wild".to_string() } else { held.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", ") },
                       if fb.is_empty() { "-".to_string() } else { fb }));
    }
    let eco = &st.eco;
    l.extend(["".into(), "## The living world".into(), "".into(),
              format!("{} species living, {} extinct.", eco.species.iter().filter(|s| s.extinct.is_none()).count(), eco.species.iter().filter(|s| s.extinct.is_some()).count()),
              "".into(), "| Species | Kind | Mythic | Population | Peak | Where it thrives | Origin | Extinct |".into(), "|---|---|---|---|---|---|---|---|".into()]);
    let mut order: Vec<usize> = (0..eco.species.len()).collect();
    order.sort_by(|&a, &b| eco.species[b].peak.partial_cmp(&eco.species[a].peak).unwrap_or(std::cmp::Ordering::Equal));
    for i in order {
        let sp = &eco.species[i];
        let pop = eco.pop.get(i).map(|p| p.as_slice()).unwrap_or(&[]);
        let tot: f64 = pop.iter().sum();
        let mut where_ = "-".to_string();
        if tot > 0.0 && eco.ew > 0 {
            let mut best = 0;
            for (j, &v) in pop.iter().enumerate() { if v > pop[best] { best = j; } }
            let key = (best / eco.ew / 4, best % eco.ew / 4);
            where_ = match eco.region_names.iter().find(|r| r.0 == key) {
                Some(r) => r.1.clone(),
                None => format!("the {} lands", eco.biome.get(best).map(|&b| BIOMES[b.min(8)]).unwrap_or("wild")),
            };
        }
        let kind = match sp.troph { 1 => "grazer", 2 => "predator", _ => "apex" };
        l.push(format!("| {} | {kind} | {} | {} | {} | {where_} | {} | {} |", sp.name, if sp.mythic { "yes" } else { "" }, nf(tot), nf(sp.peak),
                       sp.origin, sp.extinct.map(|y| y.to_string()).unwrap_or_default()));
    }
    l.extend(["".into(), "## What happened to the wild".into(), "".into()]);
    for (y, t) in &eco.events { l.push(format!("- Year {y}: {t}")); }
    l
}

fn myths_book(st: &State) -> Vec<String> {
    let item = |i: usize| if i != NONE && i < st.items.len() { st.items[i].name.clone() } else { "none".to_string() };
    let mut l: Vec<String> = vec!["# Myths, curses and legends".into(), "".into(), "## The curses of return".into(), "".into()];
    for cy in &st.cycles {
        let sealed = cy.state == "sealed";
        l.extend([format!("### {}, {}", cy.villain, cy.title), "".into(),
                  format!("Bound to {} since year {}; it seeks {}. Sworn blade: {}. State: **{}**{}.", land_name(st, cy.landmass as i64), cy.born, item(cy.relic),
                          if cy.blade != NONE { item(cy.blade) } else { "none yet".into() }, cy.state,
                          if sealed { format!(", next return year {}", cy.next_return) } else { String::new() }), "".into()]);
        for (n, a) in cy.ages.iter().enumerate() { l.push(format!("- Age {}: {a}", n + 1)); }
        if !sealed {
            let ruling = if cy.state == "triumphant" && cy.ruler != NONE && cy.ruler < st.chars.len() {
                format!("it rules as {} from the stolen throne", nm(st, cy.ruler))
            } else { "it walks the land".to_string() };
            l.push(format!("- Age {} (still unfolding): {ruling}; marked hero {}; seer {}", cy.ages.len() + 1, nm_or(st, cy.hero, "not yet born"), nm_or(st, cy.seer, "none")));
        }
        l.push(String::new());
    }
    l.extend(["## Curses".into(), "".into(), "| Kind | On | Laid by | Since | Lifted |".into(), "|---|---|---|---|---|".into()]);
    for cu in &st.curses {
        let on = match cu.kind {
            "blood" => st.houses.get(cu.house).map(|h| h.name.clone()).unwrap_or_default(),
            "item" => item(cu.item),
            "land" => land_name(st, cu.land as i64).to_string(),
            _ => st.cycles.get(cu.cycle).map(|c| c.villain.clone()).unwrap_or_default(),
        };
        l.push(format!("| {} | {on} | {} | {} | {} |", cu.kind, cu.source, cu.since, cu.lifted.map(|y| y.to_string()).unwrap_or_else(|| "no".into())));
    }
    l.extend(["".into(), "## Legendary weapons".into(), "".into()]);
    for it in st.items.iter().filter(|it| it.rarity == 5 && it.special == Some("vs_monster") && it.origin.starts_with("forged by")) {
        let b = it.bearers.iter().filter(|&&b| b < st.chars.len()).map(|&b| full_name(&st.chars[b])).collect::<Vec<_>>().join(", ");
        l.push(format!("- **{}** ({}, year {}): borne by {}", it.name, it.origin, it.year, if b.is_empty() { "none yet".to_string() } else { b }));
    }
    l.extend(["".into(), "## Dragonriders and merged blood".into(), "".into()]);
    for (y, c, n, how) in &st.dragon_riders {
        if *c < st.chars.len() { l.push(format!("- Year {y}: {} ({}) {how} the dragon {n}", full_name(&st.chars[*c]), PEOPLES[st.chars[*c].people.min(3)])); }
    }
    for (y, c, k, n) in &st.merges { l.push(format!("- Year {y}: {} merged blood with {n} the {k}", nm(st, *c))); }
    l.extend(["".into(), "## The tales".into(), "".into()]);
    for (y, t, x) in &st.myths { l.push(format!("- **{t}** (year {y}): {x}")); }
    l
}

fn dungeons_book(st: &State) -> Vec<String> {
    let mut l: Vec<String> = vec!["# Dungeons".into(), "".into(), "| Dungeon | Land | Kind | Floors | Deepest cleared | Boss | State | Conquests |".into(),
        "|---|---|---|---|---|---|---|---|".into()];
    let mut rs: Vec<&world::people::Ruin> = st.ruins.iter().filter(|r| r.floors > 0).collect();
    rs.sort_by_key(|r| std::cmp::Reverse(r.floors));
    for r in rs {
        let lm = if r.pos.0 < st.map.h && r.pos.1 < st.map.w { st.map.landmass.at(r.pos.0, r.pos.1) } else { -1 };
        let land = if lm >= 0 { land_name(st, lm as i64) } else { "?" };
        l.push(format!("| {} | {land} | {}{} | {} | {} | {} | {} | {} |", r.name, r.kind, if r.grand { " (grand)" } else { "" }, r.floors, r.floor,
                       r.boss.0, match r.conquered { Some(y) => format!("conquered {y}"), None => "unconquered".into() }, r.conquests.len()));
    }
    l.extend(["".into(), "## Conquests".into(), "".into()]);
    for (y, h, n, b, f) in &st.dungeon_log { l.push(format!("- Year {y}: {} cleared {n} ({f} floors), killing {b}", nm(st, *h))); }
    l
}

fn seas_book(st: &State) -> Vec<String> {
    let mut l: Vec<String> = vec!["# The seas".into(), "".into(), "| Realm | Warships | Merchantmen | Best ship | Lord Admiral | Colonies | Lands known |".into(),
        "|---|---|---|---|---|---|---|".into()];
    for k in &st.realms {
        let adm = if k.council[5] != NONE && k.council[5] < st.chars.len() { full_name(&st.chars[k.council[5]]) } else { "-".into() };
        l.push(format!("| {} | {:.0} | {:.0} | {} | {adm} | {} | {} |", k.name, k.warships, k.merchants, clip(&SHIP_NAMES, k.ship_tier as usize), k.colonies, k.known.len()));
    }
    l.extend(["".into(), "## Discoveries".into(), "".into()]);
    for k in &st.realms {
        for &(y, lid, by) in &k.discoveries {
            l.push(format!("- Year {y}: {} ({}), by {} of {}", land_name(st, lid as i64), land_kind(st, lid as i64),
                           if by != NONE && by < st.chars.len() { full_name(&st.chars[by]) } else { "sailors".into() }, k.name));
        }
    }
    l.extend(["".into(), format!("## Naval battles ({})", st.naval_battles.len()), "".into()]);
    let from = st.naval_battles.len().saturating_sub(40);
    for (y, where_, w, lo, ws, ls) in &st.naval_battles[from..] { l.push(format!("- Year {y}: {w} over {lo} at {where_} ({ws} against {ls})")); }
    l
}

fn pirates_book(st: &State) -> Vec<String> {
    let mut pir: Vec<&Character> = st.pirates.iter().filter(|&&i| i < st.chars.len()).map(|&i| &st.chars[i]).collect();
    pir.sort_by(|a, b| b.infamy.partial_cmp(&a.infamy).unwrap_or(std::cmp::Ordering::Equal));
    let mut l: Vec<String> = vec!["# Pirates".into(), "".into(), "## Pirate Kings".into(), "".into()];
    if st.pirate_kings.is_empty() { l.push("- none yet".into()); }
    for &(y, i) in &st.pirate_kings { l.push(format!("- Year {y}: {}", nm(st, i))); }
    l.extend(["".into(), "## The most infamous, and their records".into(), "".into()]);
    for p in pir.iter().take(15) {
        l.extend([format!("### {} ({}), infamy {}", full_name(p), SINGULAR[p.people.min(3)], nf(p.infamy)), "".into(),
                  format!("Ship: {} ({}, {} hands). Fate: {}.", p.ship.0, clip(&SHIP_NAMES, p.ship.1.min(4)), p.ship.2, fate(p)), "".into()]);
        for line in &p.resume { l.push(format!("- {line}")); }
        l.push(String::new());
    }
    l
}

fn society_book(st: &State) -> Vec<String> {
    let mut l: Vec<String> = vec!["# Society".into(), "".into(), "## Guilds".into(), "".into(), "| Guild | Guildmaster | Members | Gold |".into(), "|---|---|---|---|".into()];
    for g in &st.guilds_soc { l.push(format!("| {} | {} | {} | {} |", g.name, nm_or(st, g.master, "-"), g.members.len(), nf(g.gold))); }
    l.extend(["".into(), "## Faiths".into(), "".into(), "| Faith | Deity | Founded | By | Followers | Tenets |".into(), "|---|---|---|---|---|---|".into()]);
    let mut rs: Vec<&world::people::Religion> = st.religions.iter().collect();
    rs.sort_by(|a, b| b.followers.partial_cmp(&a.followers).unwrap_or(std::cmp::Ordering::Equal));
    for r in rs { l.push(format!("| {} | {} | {} | {} | {} | {} |", r.name, r.deity, r.founded, nm_or(st, r.founder, "of old"), nf(r.followers), r.tenets.join(", "))); }
    l.extend(["".into(), "## The greatest of the age".into(), "".into()]);
    for (w, i) in &st.greatest { l.push(format!("- The greatest {w}: {}", nm(st, *i))); }
    l.extend(["".into(), "## Crime".into(), "".into()]);
    for (k, rm) in st.realms.iter().enumerate() {
        let towns = alive_towns(st, k);
        if !towns.is_empty() {
            l.push(format!("- {}: mean crime {:.0}%", rm.name, 100.0 * towns.iter().map(|&t| st.towns[t].crime).sum::<f64>() / towns.len() as f64));
        }
    }
    l.extend(["".into(), "## The shinobi clans".into(), "".into()]);
    for cl in &st.clans {
        l.push(format!("- **{}**: {} shinobi, led by {}; missions won {}, lost {}; pact with {}", cl.name, cl.members.len(), nm_or(st, cl.head, "no one"), cl.won, cl.lost, cl.pact));
    }
    l.extend(["".into(), "## The bending tribes".into(), "".into()]);
    for t in &st.tribes {
        l.push(format!("- **{}** ({}): {} benders, chief {}", t.name, ELEMENTS[t.element.min(3)], t.members.len(), nm_or(st, t.chief, "-")));
    }
    l.extend(["".into(), format!("## {}", CONCORD[0]), "".into()]);
    for rec in &st.concord_line {
        if rec.id >= st.chars.len() { continue; }
        l.push(format!("- {} ({}), born year {} in a time of need ({}); revealed {}; {}{}", full_name(&st.chars[rec.id]), ELEMENTS[rec.element.min(3)], rec.born, rec.need,
                       rec.revealed.map(|y| y.to_string()).unwrap_or_else(|| "never".into()),
                       if rec.deeds.is_empty() { "no great deeds".to_string() } else { rec.deeds.join("; ") },
                       rec.died.as_ref().map(|d| format!("; died year {}: {}", d.0, d.1)).unwrap_or_default()));
    }
    l
}

fn works_book(st: &State) -> Vec<String> {
    let works = &st.works;
    let wh = |i: usize| if i != NONE && i < st.chars.len() { full_name(&st.chars[i]) } else { "an unknown hand".to_string() };
    let artistic = world::people::gene("artistic");
    let gifted = st.chars.iter().filter(|c| c.alive && c.genetic.contains(&artistic)).count();
    let where_ = |w: &world::works::Work| -> String {
        if let Some(y) = w.lost { return format!("lost (year {y})"); }
        if ["treatise", "chronicle", "epic", "scripture", "theology", "philosophy"].contains(&w.kind) {
            return format!("{} cop{}", w.copies.len(), if w.copies.len() == 1 { "y" } else { "ies" });
        }
        let k = if w.realm != NONE { realm_name(st, w.realm).to_string() } else { "no one".to_string() };
        if w.kind == "heirloom" { return format!("held by {}", wh(w.holder)); }
        format!("in {} ({k})", if w.town != NONE && w.town < st.towns.len() { town_name(st, w.town) } else { "the court" })
    };
    let mut l: Vec<String> = vec!["# Works".into(), "".into(),
        "Only the gifted make art and books: the artistic, about one birth in seventy, and the gift runs in families. Devout makers write holy books, which draw every town that keeps a copy toward their faith; a new faith's founder leaves its scripture. Natural philosophy comes from the learned among the gifted and from learned geniuses: medicine saves lives, agronomy raises yields, astronomy times the sowing, mathematics and engineering raise houses faster, alchemy lifts the arcane trades, and all of it feeds its people's research. Lesser pieces are made, sold and forgotten; these are the ones the world kept.".into(),
        "".into(), format!("{} works remembered ({} still in the world); {gifted} artistic people living.", works.len(), works.iter().filter(|w| w.lost.is_none()).count()), "".into()];
    let sections: [(&str, &[&str]); 6] = [("Monuments", &["monument"]), ("Masterpieces", &["painting", "carving", "tapestry", "song"]),
        ("Books", &["chronicle", "epic", "treatise"]), ("Holy books", &["scripture", "theology"]), ("Natural philosophy", &["philosophy"]), ("Heirlooms", &["heirloom"])];
    for (name, kinds) in sections {
        let mut ws: Vec<&world::works::Work> = works.iter().filter(|w| kinds.contains(&w.kind)).collect();
        ws.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap_or(std::cmp::Ordering::Equal));
        l.extend([format!("## {name} ({})", ws.len()), "".into()]);
        for w in ws.iter().take(40) {
            let maker = if w.kind == "heirloom" { String::new() } else { format!(" by {}, year {}", wh(w.maker), w.year) };
            let faith = if w.faith != NONE && w.faith < st.religions.len() { st.religions[w.faith].name.clone() } else { "an unknown faith".into() };
            let about = match w.kind {
                "treatise" => format!("Teaches {}.", sector_name(w.sector)),
                "philosophy" => format!("On {}.", w.field),
                "scripture" => format!("Scripture of {faith}."),
                _ => format!("Of {}.", w.subject),
            };
            l.push(format!("- **{}** ({}{maker}; {}). {about}", w.title, w.kind, where_(w)));
            l.push(format!("  History: {}", w.history.iter().map(|(y, t)| format!("{y}: {t}")).collect::<Vec<_>>().join("; ")));
        }
        if ws.is_empty() { l.push("- none yet".into()); }
        l.push(String::new());
    }
    let mut makers: BTreeMap<usize, Vec<&world::works::Work>> = BTreeMap::new();
    for w in works.iter().filter(|w| w.kind != "heirloom") { makers.entry(w.maker).or_default().push(w); }
    let mut ms: Vec<(usize, Vec<&world::works::Work>)> = makers.into_iter().collect();
    ms.sort_by(|a, b| {
        let qa: f64 = a.1.iter().map(|w| w.quality).sum();
        let qb: f64 = b.1.iter().map(|w| w.quality).sum();
        qb.partial_cmp(&qa).unwrap_or(std::cmp::Ordering::Equal)
    });
    l.extend(["## The great makers".into(), "".into()]);
    for (m, ws) in ms.iter().take(15) {
        let best = ws.iter().fold(ws[0], |b, w| if w.quality > b.quality { w } else { b });
        l.push(format!("- {}: {} works, best *{}*", wh(*m), ws.len(), best.title));
    }
    l
}

// ---------------------------------------------------------------- realms and statecraft

/// What `a` thinks of `b`, as the named terms the sum is made of; whatever the named terms leave is the realm's rule and the court.
fn opinion_terms(st: &State, a: usize, b: usize) -> (i64, Vec<(String, i64)>) {
    let (ca, cb) = (&st.chars[a], &st.chars[b]);
    let y = st.year;
    let mut t: Vec<(String, i64)> = vec![];
    for &p in &ca.personality {
        if cb.personality.contains(&p) { t.push((format!("shares their {} nature", PERSONALITY[p]), 10)); }
        else if cb.personality.contains(&(p ^ 1)) { t.push((format!("{}, and they are {}", PERSONALITY[p], PERSONALITY[p ^ 1]), -10)); }
    }
    t.push(("their diplomacy".into(), ((cb.attr(0, y) - 6.0) * 1.5).round() as i64));
    if cb.has(world::people::gene("comely")) { t.push(("beautiful".into(), 5)); }
    if ca.people == cb.people { t.push(("same people".into(), 5)); } else { t.push(("a foreigner".into(), -10)); }
    if ca.religion == cb.religion { t.push(("same faith".into(), 10)); }
    else if st.religions.get(ca.religion).is_some_and(|r| r.has("holy war")) { t.push(("a heathen".into(), -30)); }
    else { t.push(("another faith".into(), -15)); }
    if b == ca.father || b == ca.mother || a == cb.father || a == cb.mother { t.push(("parent or child".into(), 20)); }
    else if (ca.father != NONE && ca.father == cb.father) || (ca.mother != NONE && ca.mother == cb.mother) { t.push(("sibling".into(), 20)); }
    if ca.spouse == b { t.push(("spouse".into(), 25)); }
    if ca.house != NONE && ca.house == cb.house { t.push(("same dynasty".into(), 10)); }
    if let Some(&(_, friend)) = ca.relations.iter().find(|r| r.0 == b) { t.push((if friend { "friend" } else { "rival" }.into(), if friend { 60 } else { -60 })); }
    if ca.rival_ruler == b { t.push(("rival ruler".into(), -40)); }
    if ca.realm < st.realms.len() && st.realms[ca.realm].ruler == b && a != b {
        t.push(("legitimacy".into(), ((cb.legitimacy.unwrap_or(50.0) - 50.0) / 2.5).round() as i64));
        if cb.tyranny > 5.0 { t.push(("tyranny".into(), -((cb.tyranny / 2.0).round() as i64))); }
        if let Some((_, start, _)) = cb.ruled { let reign = y - start; if reign < 10 { t.push(("short reign".into(), -(10 - reign) * 2)); } }
        if st.realms[ca.realm].council.contains(&a) { t.push(("on the council".into(), 10)); }
    }
    for m in ca.memories.iter().filter(|m| m.0 == b) { t.push((m.1.to_string(), m.2.round() as i64)); }
    let total = st.opinion(a, b) as i64;
    let named: i64 = t.iter().map(|x| x.1).sum();
    if total != named { t.push(("how the realm is ruled and the court".into(), total - named)); }
    t.retain(|x| x.1 != 0);
    (total, t)
}

fn realms_book(st: &State) -> Vec<String> {
    let mut l: Vec<String> = vec!["# Realms, civilisation and statecraft".into(), "".into(), "## The peoples' learning".into(), "".into(),
        "| People | Era | Innovations | Traditions | Ethos | Trend of the times |".into(), "|---|---|---|---|---|---|".into()];
    for cu in &st.cultures {
        let tr = cu.traditions.iter().map(|&t| TRADITIONS[t].0).collect::<Vec<_>>().join(", ");
        l.push(format!("| {} | {} | {} | {} | {} | {} |", PEOPLES[cu.people.min(3)], clip(&ERAS, cu.era), cu.innovations.len(), if tr.is_empty() { "-".into() } else { tr },
                       ETHOS[cu.people.min(3)], ideology_label(&cu.trend)));
    }
    l.extend(["".into(), "## Living realms".into(), "".into(),
              "| Realm | People | Rank | Government | Crown | Agenda | Towns | Treasury | Levy | Men-at-arms | Strategic resources | Vassals / limit | Overlord | Court (grandeur / expected) |".into(),
              "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|".into()]);
    let mut live: Vec<usize> = (0..st.realms.len()).filter(|&k| !st.realms[k].extinct).collect();
    live.sort_by_key(|&k| std::cmp::Reverse(alive_towns(st, k).len()));
    let vassals_of = |k: usize| -> BTreeMap<usize, &Character> {
        let r = st.realms[k].ruler;
        let mut m = BTreeMap::new();
        for d in st.duchies.values() {
            if d.ctrl == k && d.holder != NONE && d.holder != r && d.holder < st.chars.len() && st.chars[d.holder].alive { m.insert(d.holder, &st.chars[d.holder]); }
        }
        m
    };
    for &k in &live {
        let rm = &st.realms[k];
        let strat: Vec<&str> = rm.held.iter().map(|&x| &RESOURCES[x]).filter(|r| r.kind == "strategic").map(|r| r.name).collect();
        let maa = rm.maa.iter().map(|&(m, n)| format!("{n} {}", MAA[m].0)).collect::<Vec<_>>().join(", ");
        let ov = if rm.overlord != NONE { realm_name(st, rm.overlord).to_string() } else { "-".into() };
        let court = match &rm.court { Some(ct) => format!("{:.0} / {} ({})", ct.grandeur, court_expected(rm.rank), ct.kind), None => "-".into() };
        l.push(format!("| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} / {:.0} | {ov} | {court} |", rm.name, PEOPLES[rm.people.min(3)], rm.rank, rm.gov,
                       rm.crown_law, rm.agenda, alive_towns(st, k).len(), nf(rm.gold), nf(rm.levy), if maa.is_empty() { "-".into() } else { maa },
                       if strat.is_empty() { "-".into() } else { strat.join(", ") }, vassals_of(k).len(), rm.vassal_limit));
    }
    l.extend(["".into(), "## The great vassals, and what they think of their rulers".into(), "".into()]);
    for &k in live.iter().take(12) {
        let rm = &st.realms[k];
        if rm.ruler == NONE || rm.ruler >= st.chars.len() { continue; }
        let dukes = vassals_of(k);
        if dukes.is_empty() { continue; }
        l.push(format!("**{}**, under {}:", rm.name, full_name(&st.chars[rm.ruler])));
        for (&d, c) in dukes.iter().take(6) {
            let (tot, terms) = opinion_terms(st, d, rm.ruler);
            let mut top = terms;
            top.sort_by_key(|t| std::cmp::Reverse(t.1.abs()));
            let mut tags = vec![];
            if st.dread(rm.ruler) > st.boldness(d) { tags.push("intimidated"); }
            if st.chars[rm.ruler].hooks.iter().any(|h| h.0 == d) { tags.push("hooked"); }
            l.push(format!("- {}{}: {tot:+}: {}", full_name(c), if tags.is_empty() { String::new() } else { format!(" ({})", tags.join(", ")) },
                           top.iter().take(5).map(|(n, v)| format!("{n} {v:+}")).collect::<Vec<_>>().join(", ")));
        }
        l.push(String::new());
    }
    l.extend(["".into(), "## Royal courts".into(), "".into()]);
    for &k in &live {
        let rm = &st.realms[k];
        let Some(ct) = &rm.court else { continue };
        let pos = ct.pos.iter().filter(|(_, c)| *c < st.chars.len()).map(|(p, c)| format!("{p}: {}", full_name(&st.chars[*c]))).collect::<Vec<_>>().join(", ");
        let mut arts: Vec<String> = rm.treasury.iter().filter(|&&i| st.items[i].rarity >= 3).map(|&i| st.items[i].name.clone()).collect();
        if rm.ruler != NONE && rm.ruler < st.chars.len() { arts.extend(st.chars[rm.ruler].items.iter().filter(|&&i| st.items[i].rarity >= 3).map(|&i| st.items[i].name.clone())); }
        arts.truncate(5);
        let amen = ["food", "lodging", "fashion", "servants"].iter().zip(ct.amen.iter()).map(|(a, v)| format!("{a} {v}")).collect::<Vec<_>>().join(", ");
        l.push(format!("- **{}** ({} court since year {}): amenities {amen}; {pos}{}{}", rm.name, ct.kind, ct.since,
                       if arts.is_empty() { String::new() } else { format!("; on display: {}", arts.join(", ")) },
                       ct.events.last().map(|e| format!("; lately {}", e.1)).unwrap_or_default()));
    }
    l.extend(["".into(), "## Great decisions and the forming of realms".into(), "".into()]);
    let brk = breakaways(st);
    if brk.is_empty() { l.push("- none".into()); }
    for &(y, n, o, c) in &brk {
        l.push(format!("- Year {y}: {}: broke from {} under {}", realm_name(st, n), realm_name(st, o), nm(st, c)));
    }
    l.extend(["".into(), "## Wars".into(), "".into(), "| Ended | Attacker | Defender | Casus belli | Years | Outcome |".into(), "|---|---|---|---|---|---|".into()]);
    let from = st.war_log.len().saturating_sub(80);
    for w in &st.war_log[from..] { l.push(format!("| {} | {} | {} | {} | {} | {} |", w.0, w.1, w.2, w.6, w.4, w.3)); }
    l.extend(["".into(), "## Wars now".into(), "".into()]);
    if st.wars.is_empty() { l.push("- none".into()); }
    for w in &st.wars {
        l.push(format!("- {} against {} ({}, since {}, warscore {:+.0})", realm_name(st, w.attacker), realm_name(st, w.defender), w.cb, w.start, w.score));
    }
    l.extend(["".into(), "## Hosts in the field".into(), "".into()]);
    if st.armies.is_empty() { l.push("- none".into()); }
    for a in &st.armies {
        l.push(format!("- {}: {} men{}", a.name, int(a.men as i64), if a.siege > 0.0 { format!(", besieging (progress {:.0}%)", a.siege) } else { String::new() }));
    }
    l.extend(["".into(), "## Great battles".into(), "".into()]);
    let mut bs: Vec<&(i64, String, String, String, i64, i64, i64)> = st.battles_fought.iter().collect();
    bs.sort_by_key(|b| std::cmp::Reverse(b.6));
    for (y, w, wn, ln, wm, lm, d) in bs.into_iter().take(20) {
        l.push(format!("- Year {y}: the Battle of {w}: {wn} ({}) against {ln} ({}); {} fell", int(*wm), int(*lm), int(*d)));
    }
    l.extend(["".into(), "## Famines, plagues, risings, bankruptcies".into(), "".into()]);
    let fam: Vec<String> = st.famines.iter().filter(|f| f.2 > 1000.0).map(|&(y, n, d)| format!("- Famine, year {y}: {n} towns, {} dead", nf(d))).collect();
    l.extend(fam[fam.len().saturating_sub(15)..].iter().cloned());
    let epi: Vec<String> = st.epidemics.iter().filter(|e| e.deaths > 2000.0 || e.great).map(|e| {
        format!("- {}{}, {}-{}: {} towns, {} dead", e.name, if e.great { " (a great pandemic)" } else { "" }, e.start, e.end.map(|x| x.to_string()).unwrap_or_else(|| "...".into()), e.towns.len(), nf(e.deaths))
    }).collect();
    l.extend(epi[epi.len().saturating_sub(15)..].iter().cloned());
    let from = st.risings.len().saturating_sub(15);
    for &(y, t, k, w) in &st.risings[from..] { l.push(format!("- Rising, year {y}: {} against {} over {w}", town_name(st, t), realm_name(st, k))); }
    for &(y, k) in &st.bankruptcies { l.push(format!("- Bankruptcy, year {y}: {}", realm_name(st, k))); }
    l.extend(["".into(), "## Great works".into(), "".into()]);
    for k in &st.realms { for n in &k.great_works { l.push(format!("- {n} ({})", k.name)); } }
    l.extend(["".into(), "## Culture history".into(), "".into()]);
    for cu in &st.cultures {
        let from = cu.history.len().saturating_sub(25);
        l.push(format!("- **{}:** {}", PEOPLES[cu.people.min(3)], cu.history[from..].iter().map(|(y, t)| format!("{y} {t}")).collect::<Vec<_>>().join("; ")));
    }
    l
}

// ---------------------------------------------------------------- calibration

fn calibration(st: &State) -> Vec<String> {
    let year = st.year;
    let human: Vec<&Character> = st.chars.iter()
        .filter(|c| c.people == 0 && c.born > 50 && c.born < year - 80 && (c.father != NONE || c.mother != NONE)).collect();
    let born = human.len();
    let span = |c: &Character| c.died.unwrap_or(year) - c.born;
    let dead_inf = human.iter().filter(|c| !c.alive && span(c) < 1).count();
    let dead15 = human.iter().filter(|c| !c.alive && span(c) < 15).count();
    let ages: Vec<i64> = human.iter().filter(|c| !c.alive && span(c) >= 15).map(|c| span(c)).collect();
    let births: Vec<usize> = human.iter().filter(|c| c.female && c.spouse != NONE && !c.alive && span(c) > 44).map(|c| c.children.len()).collect();
    let childbed = human.iter().filter(|c| c.cause.as_deref() == Some("died in childbirth")).count();
    let towns: Vec<&world::people::Town> = st.towns.iter().filter(|t| t.alive).collect();
    let mut l: Vec<String> = vec!["# Calibration".into(), "".into(),
        "The rates below were set from commonly cited figures for medieval Europe (approximate, and varying a".into(),
        "great deal by place and century). Humans are calibrated directly; the other peoples are scaled by their".into(),
        "lifespan. The measured column is what this run actually produced, from the human notables born to parents".into(),
        "in the world after year 50 and at least 80 years ago (so their lives are complete or nearly).".into(), "".into(),
        "| Measure | Reference (medieval Europe, approximate) | This run |".into(), "|---|---|---|".into()];
    if born > 0 {
        l.push(format!("| Infant deaths (first year) | about 15-30% (lower for elites) | {:.0}% of {born} |", 100.0 * dead_inf as f64 / born as f64));
        l.push(format!("| Deaths before 15 | about 30-50% | {:.0}% |", 100.0 * dead15 as f64 / born as f64));
    }
    if !ages.is_empty() { l.push(format!("| Mean age at death, for those reaching 15 | about 50-65 for elites | {:.0} |", ages.iter().sum::<i64>() as f64 / ages.len() as f64)); }
    if !births.is_empty() { l.push(format!("| Children per married woman (completed) | about 5-7 | {:.1} |", births.iter().sum::<usize>() as f64 / births.len() as f64)); }
    if born > 0 { l.push(format!("| Deaths in childbirth | about 1-1.5% per birth | {childbed} of {born} births |", )); }
    let great: Vec<&world::people::Epidemic> = st.epidemics.iter().filter(|e| e.great).collect();
    l.push(format!("| Great pandemics (Black Death scale) | about one every 150-300 years, 30-60% mortality where it reaches | {} in {year} years; {} |", great.len(),
                   if great.is_empty() { "none".to_string() } else { great.iter().take(4).map(|e| format!("{} ({}, {} dead)", e.name, e.start, nf(e.deaths))).collect::<Vec<_>>().join(", ") }));
    l.push(format!("| Small epidemics | every few years | {} |", st.epidemics.len() - great.len()));
    l.push(format!("| Famines (killing over 1,000) | regional bad harvests about one year in seven; great famines a few per century | {} |", st.famines.len()));
    l.push(format!("| Peasant risings | dozens per century across Europe in the Late Middle Ages | {} |", st.risings.len()));
    l.push(format!("| Bankrupt crowns | several (Edward III, Philip II...) | {} |", st.bankruptcies.len()));
    // the realms' economy by class, from the towns
    let ks: Vec<usize> = (0..st.realms.len()).filter(|&k| !st.realms[k].extinct).collect();
    if !ks.is_empty() {
        let mut pop = 0.0;
        let mut poor = 0.0;
        let mut people = [0.0f64; 3];
        let mut top = 0.0;
        for &k in &ks {
            let ts = alive_towns(st, k);
            let kp: f64 = ts.iter().map(|&t| st.towns[t].pop).sum();
            let communal = matches!(st.realms[k].gov, "tribal" | "clan");
            let mut assets = [0.0f64; 3];
            for &t in &ts {
                let tt = &st.towns[t];
                for ci in 0..3 {
                    let (pp, _, pv) = tt.cls[ci];
                    people[ci] += pp;
                    poor += pp * pv;
                    let land = tt.land_value * [if communal { 0.5 } else { 0.0 }, 0.15, if communal { 0.35 } else { 0.85 }][ci];
                    let house = tt.housing.unwrap_or(0.0) * 0.02 * [0.3, 0.35, 0.35][ci];
                    let coin = tt.coin.unwrap_or(0.0) * [0.10, 0.35, 0.55][ci];
                    assets[ci] += land + house + tt.stock[ci] + coin;
                }
            }
            top += assets[2] / assets.iter().sum::<f64>().max(1e-9) * kp;
            pop += kp;
        }
        let pop = pop.max(1.0);
        let farm_towns: Vec<&&world::people::Town> = towns.iter().filter(|t| t.jobs.iter().sum::<f64>() > 0.0).collect();
        let farm = farm_towns.iter().map(|t| t.jobs[0]).sum::<f64>() / farm_towns.iter().map(|t| t.jobs.iter().sum::<f64>()).sum::<f64>().max(1.0);
        let tp = towns.iter().map(|t| t.pop).sum::<f64>().max(1.0);
        let eaten = towns.iter().map(|t| t.food_prod).sum::<f64>() / tp;
        let packed = towns.iter().filter(|t| t.crowded > 0.1).map(|t| t.pop).sum::<f64>() / tp;
        l.push(format!("| Forced loans (seizing merchants' gold) | common in crises (Edward III's wool, Philip IV) | {} |", st.stats_forced.len()));
        l.push(format!("| Share of workers on the land | about 75-90% | {} |", pct(farm)));
        l.push(format!("| Food eaten per head (after seed, beasts and losses) | most households near subsistence | {eaten:.2} |"));
        l.push(format!("| Townsfolk in overcrowded housing | common in growing towns | {} |", pct(packed)));
        l.push(format!("| Poor (below subsistence) | about 30-50% near or below the line in hard times | {} now |", pct(poor / pop)));
        l.push(format!("| Classes (lower / middle / upper) | about 80 / 15 / 5 | {} / {} / {} |", pct(people[0] / pop), pct(people[1] / pop), pct(people[2] / pop)));
        l.push(format!("| Wealth held by the upper class | the richest 5-10% held about 50-70% | {} |", pct(top / pop)));
    }
    l.extend(["".into(), "## The parameters".into(), "".into(),
        "- Crude birth and death rates (humans): 40 and 35 per thousand a year in a fed town, higher births on open land.".into(),
        "- Child mortality: 16% in the first year, 3% a year to five, 0.8% a year to fifteen.".into(),
        "- Adult mortality: 0.4% a year plus 0.25% growing about 8.5% a year past thirty (doubling every eight years).".into(),
        "- Marital fertility: 32% a year for a married woman, halving in her last fertile years and falling with each child.".into(),
        "- Childbirth: 1.2% of births kill the mother.".into(),
        "- Harvests: a smooth regional field with 13% spread; a year without a summer about one year in thirty somewhere.".into(),
        "- Famine: half of any food shortfall left after the granary dies; the hungry migrate.".into(),
        "- Plague: small outbreaks about one year in forty, great pandemics at most one in 120 years (35-60% fatality).".into(),
        "- Technology: four eras (Tribal to Late Medieval), each taking two to four centuries, like 500-1500 in Europe.".into(), "".into(),
        format!("Population now: {} in {} towns; {} have migrated from hunger and war since the beginning.",
                nf(towns.iter().map(|t| t.pop).sum::<f64>()), towns.len(), nf(st.migrants))]);
    l
}
