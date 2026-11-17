//! The character sheet and the rows the game's tabs show, read straight from the world state: the sheet as
//! Markdown, wealth and the economy, the Wardens' Hall, the realms and the money ledger, and the wars.

use std::collections::BTreeSet;

use world::lives::AMBITIONS;
use world::people::{gene, Character, NONE};
use world::power::level_of;
use world::shinobi::technique;
use world::state::State;
use world::stewardship::stage;
use world::tables::*;
use world::wardens::SEALS;

const WORKING: f64 = 0.55;
const STAGES: [&str; 3] = ["barter", "coin", "credit"];
const CLASS_NAMES: [&str; 3] = ["lower", "middle", "upper"];
const ATTR_LABELS: [&str; 6] = ["Diplomacy", "Martial", "Stewardship", "Intrigue", "Learning", "Storytelling"];
const ATTRIBUTE_LABELS: [&str; 8] = ["Strength", "Intelligence", "Willpower", "Agility", "Speed", "Endurance", "Personality", "Luck"];
const FOCUS_NAMES: [&str; 5] = ["Diplomacy", "Martial", "Stewardship", "Intrigue", "Learning"];
const AXIS_NAMES: [(&str, &str); 5] = [("absolutist", "councilist"), ("hereditary", "elective"), ("theocratic", "secular"),
                                       ("mercantile", "agrarian"), ("traditionalist", "reformist")];
const BATTLE_ARTS: [&str; 7] = ["Steel Breath", "Iron Fortress Stance", "Flowing Step", "Twin Fang Strike", "Mind's-Eye Parry", "Thousand Cuts", "Heaven-Cleaver"];
const GIFT_TEXT: [&str; 12] = [
    "casts one tier above their training, and past their people's limit", "can reach beyond the tenth tier from the ninth",
    "the fields of their town yield more", "sees the worth of anything; earns double at trade, lifts their town's markets",
    "teaches a trade twice as well", "the sick of their town recover", "a body like armour", "learns everything faster",
    "persuades anyone", "men follow them", "sees through lies and plots", "speaks with beasts",
];

// ------------------------------------------------------------------ formatting

fn group(int_part: &str) -> String {
    let mut out = String::new();
    for (i, ch) in int_part.chars().enumerate() {
        if i > 0 && (int_part.len() - i) % 3 == 0 { out.push(','); }
        out.push(ch);
    }
    out
}

/// A number with thousands separators and a fixed count of decimals.
fn commas(x: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, x);
    let (sign, body) = match s.strip_prefix('-') { Some(b) => ("-", b.to_string()), None => ("", s.clone()) };
    let (int_part, frac) = match body.split_once('.') { Some((i, f)) => (i.to_string(), format!(".{f}")), None => (body.clone(), String::new()) };
    format!("{sign}{}{frac}", group(&int_part))
}

fn commas_i(x: i64) -> String { commas(x as f64, 0) }

fn pct(x: f64) -> String { format!("{:.0}%", x * 100.0) }

fn cut(s: &str, n: usize) -> String { s.chars().take(n).collect() }

fn plural_join(v: &[String], sep: &str) -> String { v.join(sep) }

// ------------------------------------------------------------------ names

fn full_of(c: &Character) -> String {
    let mut n = c.name.clone();
    if let Some(r) = &c.regnal { if !r.is_empty() { n.push(' '); n.push_str(r); } }
    if let Some(e) = &c.epithet { if !e.is_empty() { n.push(' '); n.push_str(e); } }
    n
}

/// A person's name with regnal number and epithet.
pub fn full_name(st: &State, c: usize) -> String {
    st.chars.get(c).map(full_of).unwrap_or_else(|| "-".to_string())
}

fn name_at(st: &State, c: usize) -> String { if c == NONE { "-".to_string() } else { full_name(st, c) } }

fn house_name(st: &State, h: usize) -> Option<&str> { if h == NONE { None } else { st.houses.get(h).map(|x| x.name.as_str()) } }

fn realm_name(st: &State, k: usize) -> String { st.realms.get(k).map(|r| r.name.clone()).unwrap_or_else(|| "-".to_string()) }

fn realm_short(st: &State, k: usize) -> String { st.realms.get(k).map(|r| r.short.clone()).unwrap_or_else(|| "-".to_string()) }

// ------------------------------------------------------------------ wealth

fn current_ruler(c: &Character) -> bool { c.ruled.is_some_and(|r| r.2.is_none()) }

/// Everything a person owns: coin, the worth of their gear and relics, and their share of the land and houses of the
/// lordships they hold.
pub fn net_worth(st: &State, c: usize) -> f64 {
    let Some(ch) = st.chars.get(c) else { return 0.0 };
    let mut w = ch.gold.max(0.0) + ch.items.iter().map(|&i| st.items[i].value).sum::<f64>() * 0.01;
    for &t in &ch.lordships {
        let Some(s) = st.towns.get(t) else { continue };
        if !s.alive { continue; }
        let communal = st.realms.get(s.realm).is_some_and(|r| matches!(r.gov, "tribal" | "clan"));
        w += 0.25 * (s.land_value * if communal { 0.35 } else { 0.85 } + s.housing.unwrap_or(0.0) * 0.02 * 0.35);
    }
    if current_ruler(ch) && ch.realm < st.realms.len() { w += 0.05 * st.realms[ch.realm].gold.max(0.0); }
    w
}

fn alive_towns(st: &State, k: usize) -> Vec<usize> { st.alive_towns_of(k) }

struct ClassEcon { people: f64, income: f64, poverty: f64, land: f64, housing: f64, stock: f64, coin: f64, per_head: f64, share: f64 }

/// A realm's economy by class: people, income a head, poverty, and what each class owns.
fn realm_economy(st: &State, k: usize) -> ([ClassEcon; 3], f64) {
    let towns = alive_towns(st, k);
    let pop: f64 = towns.iter().map(|&t| st.towns[t].pop).sum::<f64>().max(1.0);
    let communal = matches!(st.realms[k].gov, "tribal" | "clan");
    let mut raw = vec![];
    let mut total = 0.0;
    for ci in 0..3 {
        let (mut n, mut inc, mut pov, mut land, mut house, mut stock, mut coin) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for &t in &towns {
            let tt = &st.towns[t];
            let (pp, per, pv) = tt.cls[ci];
            n += pp; inc += pp * per; pov += pp * pv;
            land += tt.land_value * [if communal { 0.5 } else { 0.0 }, 0.15, if communal { 0.35 } else { 0.85 }][ci];
            house += tt.housing.unwrap_or(0.0) * 0.02 * [0.3, 0.35, 0.35][ci];
            stock += tt.stock[ci];
            coin += tt.coin.unwrap_or(0.0) * [0.10, 0.35, 0.55][ci];
        }
        total += land + house + stock + coin;
        raw.push((n, inc, pov, land, house, stock, coin));
    }
    let mut poverty = 0.0;
    let mk = |i: usize| {
        let (n, inc, pov, land, housing, stock, coin) = raw[i];
        let sum = land + housing + stock + coin;
        ClassEcon { people: n, income: inc / n.max(1.0), poverty: pov / n.max(1.0), land, housing, stock, coin,
                    per_head: sum / n.max(1.0), share: sum / total.max(1e-9) }
    };
    let classes = [mk(0), mk(1), mk(2)];
    for r in &raw { poverty += r.2; }
    (classes, poverty / pop)
}

fn live_realms_by_gdp(st: &State) -> Vec<usize> {
    let mut ks: Vec<usize> = (0..st.realms.len()).filter(|&k| !st.realms[k].extinct).collect();
    ks.sort_by(|&a, &b| st.realms[b].real_gdp.partial_cmp(&st.realms[a].real_gdp).unwrap_or(std::cmp::Ordering::Equal));
    ks
}

/// The Economy table: one row per living realm, richest output first.
pub fn econ_rows(st: &State) -> Vec<Vec<String>> {
    let mut rows = vec![];
    for k in live_realms_by_gdp(st) {
        let r = &st.realms[k];
        let towns = alive_towns(st, k);
        let mut work: f64 = towns.iter().map(|&t| WORKING * st.towns[t].pop).sum();
        if work == 0.0 { work = 1.0; }
        let jobless = towns.iter().map(|&t| st.towns[t].jobless * WORKING * st.towns[t].pop).sum::<f64>() / work;
        let (classes, poverty) = realm_economy(st, k);
        let total_pop: f64 = towns.iter().map(|&t| st.towns[t].pop).sum::<f64>();
        let has_money = r.credit.is_some() && r.money != 0.0;
        let stage_ix = st.cultures.get(r.people).map(|cu| stage(&cu.innovations) as usize).unwrap_or(0);
        let parts = ["h", "a", "h", "s"].iter().zip(r.parts.iter()).filter(|(_, &v)| v >= 1.0)
            .map(|(n, v)| format!("{n}{v:.0}")).collect::<Vec<_>>().join(" ");
        rows.push(vec![
            cut(&r.name, 38), STAGES[stage_ix.min(2)].to_string(), commas(r.real_gdp, 0),
            if has_money { commas(r.money, 0) } else { "-".to_string() }, commas((-r.gold).max(0.0), 0), pct(r.rate),
            if has_money { format!("{:.0}", r.credit.unwrap_or(0.0)) } else { "-".to_string() }, pct(jobless), pct(poverty),
            pct(classes[1].people / total_pop.max(1.0)), r.partners.to_string(), r.top_scroll.to_string(),
            if parts.is_empty() { "-".to_string() } else { parts }, pct(r.bound_share),
        ]);
    }
    rows
}

/// The classes and assets table: three rows for each of the twelve largest economies.
pub fn class_rows(st: &State) -> Vec<Vec<String>> {
    let mut rows = vec![];
    for k in live_realms_by_gdp(st).into_iter().take(12) {
        let (classes, _) = realm_economy(st, k);
        let pop = classes.iter().map(|c| c.people).sum::<f64>();
        let pop = if pop == 0.0 { 1.0 } else { pop };
        for (i, v) in classes.iter().enumerate() {
            rows.push(vec![
                cut(&st.realms[k].name, 34), CLASS_NAMES[i].to_string(), commas(v.people, 0), pct(v.people / pop),
                format!("{:.2}", v.income * 1000.0), pct(v.poverty), commas(v.land, 0), commas(v.housing, 0), commas(v.stock, 0),
                commas(v.coin, 0), format!("{:.1}", v.per_head * 1000.0), pct(v.share),
            ]);
        }
    }
    rows
}

/// The richest living people: name, realm, net worth, gold in hand, work, id.
pub fn rich_rows(st: &State, n: usize) -> Vec<Vec<String>> {
    let mut people: Vec<(usize, f64)> = st.living.iter().copied().filter(|&i| st.chars.get(i).is_some_and(|c| c.alive))
        .map(|i| (i, net_worth(st, i))).collect();
    people.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    people.truncate(n);
    people.into_iter().map(|(i, nw)| {
        let ch = &st.chars[i];
        let work = match ch.profession { Some(p) => p.to_string(), None if current_ruler(ch) => "ruler".to_string(),
                                         None if !ch.lordships.is_empty() => "lord".to_string(), None => "-".to_string() };
        vec![cut(&full_of(ch), 40), realm_short(st, ch.realm), commas(nw, 1), commas(ch.gold, 0), work, i.to_string()]
    }).collect()
}

// ------------------------------------------------------------------ realms and money

/// The Realms tab: id, name, people, rank, rule, ruler, towns, population, gold, army, wars; most towns first.
pub fn realm_rows(st: &State) -> Vec<Vec<String>> {
    let mut ks: Vec<(usize, usize)> = (0..st.realms.len()).filter(|&k| !st.realms[k].extinct).map(|k| (k, alive_towns(st, k).len())).collect();
    ks.sort_by(|a, b| b.1.cmp(&a.1));
    ks.into_iter().map(|(k, nt)| {
        let r = &st.realms[k];
        let pop: f64 = alive_towns(st, k).iter().map(|&t| st.towns[t].pop).sum();
        let at_war: Vec<String> = st.war.get(k).map(|row| row.iter().enumerate().filter(|(o, &w)| w && *o < st.realms.len())
            .map(|(o, _)| st.realms[o].short.clone()).take(3).collect()).unwrap_or_default();
        vec![k.to_string(), r.name.clone(), PEOPLES[r.people].to_string(), r.rank.to_string(), r.gov.to_string(), name_at(st, r.ruler),
             nt.to_string(), (pop as i64).to_string(), (r.gold as i64).to_string(), (r.army_power as i64).to_string(),
             if at_war.is_empty() { "-".to_string() } else { at_war.join(", ") }]
    }).collect()
}

/// The money ledger: per living realm, the sums of the years after `since_year`.
pub fn ledger_rows(st: &State, since_year: i64) -> Vec<Vec<String>> {
    let mut ks: Vec<(usize, usize)> = (0..st.realms.len()).filter(|&k| !st.realms[k].extinct).map(|k| (k, alive_towns(st, k).len())).collect();
    ks.sort_by(|a, b| b.1.cmp(&a.1));
    let mut out = vec![];
    for (k, _) in ks {
        let mut sums: Vec<(&str, f64)> = vec![];
        for (&(y, kid), row) in &st.ledger {
            if kid != k || y <= since_year { continue; }
            for &(cat, v) in row {
                match sums.iter_mut().find(|s| s.0 == cat) { Some(s) => s.1 += v, None => sums.push((cat, v)) }
            }
        }
        let get = |name: &str| sums.iter().find(|s| s.0 == name).map(|s| s.1).unwrap_or(0.0);
        let income: f64 = sums.iter().filter(|s| s.1 > 0.0).map(|s| s.1).sum();
        let spending: f64 = -sums.iter().filter(|s| s.1 < 0.0).map(|s| s.1).sum::<f64>();
        let armies = get("army upkeep") + get("regiments") + get("field armies") + get("muster");
        out.push(vec![cut(&st.realms[k].name, 40), commas(income, 0), commas(spending, 0), commas(get("taxes"), 0), commas(get("loot"), 0),
                      commas(get("tribute in") + get("tribute out"), 0), commas(armies, 0), commas(get("court"), 0),
                      commas(get("interest"), 0), commas(st.realms[k].gold, 0)]);
    }
    out
}

/// The latest `last` bookings: year, amount and "[category] Realm: note".
pub fn tx_lines(st: &State, last: usize) -> Vec<(i64, f64, String)> {
    let from = st.tx.len().saturating_sub(last);
    st.tx[from..].iter().map(|(y, kid, cat, amt, note, _)| {
        let name = if *kid != NONE && *kid < st.realms.len() { st.realms[*kid].short.as_str() } else { "-" };
        (*y, *amt, format!("[{cat}] {name}: {note}"))
    }).collect()
}

// ------------------------------------------------------------------ the Wardens' Hall

/// The parties, highest seal first, then most quests done.
pub fn party_rows(st: &State) -> Vec<Vec<String>> {
    let mut ps: Vec<&world::wardens::Party> = st.hall.parties.iter().collect();
    ps.sort_by(|a, b| b.rank.cmp(&a.rank).then(b.done.cmp(&a.done)));
    ps.into_iter().map(|p| {
        let names: Vec<String> = p.members.iter().filter(|&&i| i < st.chars.len()).map(|&i| full_name(st, i)).collect();
        vec![p.name.clone(), SEALS[p.rank.min(6)].to_string(), names.len().to_string(), p.done.to_string(), p.lost.to_string(),
             p.formed.to_string(), cut(&names.join(", "), 90)]
    }).collect()
}

/// The quest board, highest seal first, then richest reward.
pub fn quest_rows(st: &State) -> Vec<Vec<String>> {
    let mut qs: Vec<&world::wardens::Quest> = st.hall.quests.iter().collect();
    qs.sort_by(|a, b| b.rank.cmp(&a.rank).then(b.reward.partial_cmp(&a.reward).unwrap_or(std::cmp::Ordering::Equal)));
    qs.into_iter().map(|q| vec![SEALS[q.rank.min(6)].to_string(), q.kind.to_string(), cut(&q.title, 70), commas(q.reward, 0),
                                q.posted.to_string(), q.expires.to_string()]).collect()
}

/// The hall's log, the latest `last` entries oldest first.
pub fn hall_log(st: &State, last: usize) -> Vec<String> {
    let from = st.hall.log.len().saturating_sub(last);
    st.hall.log[from..].iter().map(|(y, who, _kind, title, rank, outcome, lost)| {
        format!("{y:>5}  [{}] {who}: {title} -> {outcome}{}", SEALS[(*rank).min(6)], if *lost > 0 { format!(" ({lost} lost)") } else { String::new() })
    }).collect()
}

// ------------------------------------------------------------------ the wars

/// The Wars tab: wars now, hosts in the field, recent wars, the greatest battles.
pub fn war_lines(st: &State) -> Vec<String> {
    let mut out = vec!["WARS NOW".to_string()];
    for w in &st.wars {
        out.push(format!("  {} against {}: {}, since {}, warscore {:+.0}", realm_name(st, w.attacker), realm_name(st, w.defender), w.cb, w.start, w.score));
    }
    out.push(String::new());
    out.push("HOSTS IN THE FIELD".to_string());
    for a in &st.armies {
        let siege = if a.siege != 0.0 { format!(", siege {:.0}%", a.siege) } else { String::new() };
        out.push(format!("  {}: {} men at ({}, {}){siege}", a.name, commas_i(a.men as i64), a.pos.0, a.pos.1));
    }
    out.push(String::new());
    out.push("RECENT WARS".to_string());
    let from = st.war_log.len().saturating_sub(60);
    for w in st.war_log[from..].iter().rev() {
        out.push(format!("  {}: {} vs {} ({}, {}y): {}", w.0, w.1, w.2, w.6, w.4, w.3));
    }
    out.push(String::new());
    out.push("GREATEST BATTLES".to_string());
    let mut bs: Vec<&(i64, String, String, String, i64, i64, i64)> = st.battles_fought.iter().collect();
    bs.sort_by(|a, b| b.6.cmp(&a.6));
    for (y, place, wn, ln, wm, lm, d) in bs.into_iter().take(30) {
        out.push(format!("  {y}: the Battle of {place}: {wn} ({}) over {ln} ({}); {} fell", commas_i(*wm), commas_i(*lm), commas_i(*d)));
    }
    out
}

// ------------------------------------------------------------------ opinion and views

/// What `a` thinks of `b`, term by term (the sum is `State::opinion`).
fn opinion_terms(st: &State, a: usize, b: usize) -> Vec<(String, i64)> {
    let (ca, cb) = (&st.chars[a], &st.chars[b]);
    let y = st.year;
    let mut t: Vec<(String, i64)> = vec![];
    for &p in &ca.personality {
        let opp = p ^ 1;
        if cb.personality.contains(&p) { t.push((format!("shares their {} nature", PERSONALITY[p]), 10)); }
        else if cb.personality.contains(&opp) && opp < PERSONALITY.len() { t.push((format!("{}, and they are {}", PERSONALITY[p], PERSONALITY[opp]), -10)); }
    }
    t.push(("their diplomacy".into(), ((cb.attr(0, y) - 6.0) * 1.5).round() as i64));
    if cb.has(gene("comely")) { t.push(("beautiful".into(), 5)); }
    t.push(if ca.people == cb.people { ("same people".into(), 5) } else { ("a foreigner".into(), -10) });
    if ca.religion == cb.religion { t.push(("same faith".into(), 10)); }
    else if st.religions.get(ca.religion).is_some_and(|r| r.has("holy war")) { t.push(("a heathen".into(), -30)); }
    else { t.push(("another faith".into(), -15)); }
    if cb.id == ca.father || cb.id == ca.mother || ca.id == cb.father || ca.id == cb.mother { t.push(("parent or child".into(), 20)); }
    else if (ca.father != NONE && ca.father == cb.father) || (ca.mother != NONE && ca.mother == cb.mother) { t.push(("sibling".into(), 20)); }
    if ca.spouse == b { t.push(("spouse".into(), 25)); }
    if ca.house != NONE && ca.house == cb.house { t.push(("same house".into(), 10)); }
    if let Some(&(_, friend)) = ca.relations.iter().find(|r| r.0 == b) { t.push(if friend { ("friend".into(), 60) } else { ("rival".into(), -60) }); }
    if ca.rival_ruler == b { t.push(("rival ruler".into(), -40)); }
    if ca.realm < st.realms.len() && st.realms[ca.realm].ruler == b && a != b {
        let k = &st.realms[ca.realm];
        t.push(("legitimacy".into(), ((cb.legitimacy.unwrap_or(50.0) - 50.0) / 2.5).round() as i64));
        if cb.tyranny > 5.0 { t.push(("tyranny".into(), -((cb.tyranny / 2.0).round() as i64))); }
        if let Some((_, start, _)) = cb.ruled { let reign = y - start; if reign < 10 { t.push(("short reign".into(), -((10 - reign) * 2))); } }
        if k.council.contains(&a) { t.push(("council seat".into(), 10)); }
        if let Some(n) = ca.view_noise {
            let mut g = GOV_VIEWS[GOVS.iter().position(|x| *x == k.gov).unwrap_or(0)];
            g[0] += 0.15 * (k.crown_law as f64 - 1.0);
            let v = views_with(st, ca, n, false);
            let agree = 1.0 - (0..5).map(|i| (v[i] - g[i]).abs()).sum::<f64>() / 5.0;
            t.push(("how the realm is ruled".into(), (30.0 * (agree - 0.55)).round() as i64));
        }
    }
    for m in &ca.memories { if m.0 == b { t.push((m.1.to_string(), m.2.round() as i64)); } }
    t.into_iter().filter(|x| x.1 != 0).collect()
}

/// `by_trade` adds the pull of a merchant's or priest's calling and a hero's restlessness, as `State::views` does;
/// `State::opinion` leaves it out of how the realm is ruled, so the opinion terms do too.
fn views_with(st: &State, cc: &Character, noise: [f64; 5], by_trade: bool) -> [f64; 5] {
    let mut v = st.cultures[cc.people].trend;
    let p = |t: &str| cc.has_trait(t) as u8 as f64;
    v[0] += 0.3 * p("ambitious") + 0.2 * p("cruel") - 0.3 * p("kind") - 0.2 * p("trusting");
    v[2] += 0.4 * p("zealous") - 0.4 * p("cynical");
    v[3] += 0.3 * p("greedy") - 0.2 * p("generous");
    v[4] += 0.2 * p("content") - 0.3 * p("ambitious") - 0.1 * p("diligent");
    v[1] += if cc.house != NONE && st.houses.get(cc.house).is_some_and(|h| h.rulers > 0) { 0.3 } else { -0.1 };
    if by_trade {
        match cc.profession {
            Some("merchant") => { v[3] += 0.6; v[1] -= 0.4; }
            Some("priest") => v[2] += 0.6,
            _ => if cc.hero { v[4] -= 0.2 },
        }
    }
    for a in 0..5 { v[a] = (v[a] + noise[a]).clamp(-1.0, 1.0); }
    v
}

/// What a person thinks governance should be; a person whose views are not yet formed holds their people's trend.
fn views(st: &State, cc: &Character) -> [f64; 5] { views_with(st, cc, cc.view_noise.unwrap_or([0.0; 5]), true) }

fn ideology_label(v: [f64; 5]) -> String {
    let out: Vec<&str> = (0..5).filter(|&a| v[a].abs() >= 0.35).map(|a| if v[a] > 0.0 { AXIS_NAMES[a].0 } else { AXIS_NAMES[a].1 }).collect();
    if out.is_empty() { "moderate".to_string() } else { out.join(", ") }
}

// ------------------------------------------------------------------ legacies

fn legacy_lines(st: &State, c: &Character) -> Vec<String> {
    let house = if c.house != NONE { st.houses.get(c.house) } else { None };
    let mut kinds: BTreeSet<usize> = c.legacy.iter().map(|x| x.0).collect();
    let track = house.and_then(|h| LEGACY_KINDS.iter().position(|k| *k == h.track));
    if let Some(t) = track { kinds.insert(t); }
    let mut ordered: Vec<usize> = kinds.into_iter().collect();
    ordered.sort_by_key(|&k| LEGACY_KINDS[k]);
    let mut out = vec![];
    for kind in ordered {
        let lv = st.legacy_level(c, kind);
        if lv == 0 { continue; }
        let own = level_of(c.legacy.iter().find(|x| x.0 == kind).map(|x| x.1).unwrap_or(0.0), &LEGACY_LEVELS);
        let src = match house { Some(h) if track == Some(kind) && own < lv => format!(", from {}", h.name), _ => String::new() };
        out.push(format!("{} {}{src}", LEGACY_NAMES[kind], ROMAN[(lv as usize).min(10)]));
    }
    out
}

// ------------------------------------------------------------------ the sheet

fn gene_names(v: &[usize]) -> Vec<&'static str> { v.iter().map(|&g| GENETIC[g]).collect() }

/// The character sheet in Markdown.
pub fn sheet(st: &State, cid: usize) -> String {
    let Some(c) = st.chars.get(cid) else { return String::new() };
    let year = if c.alive { st.year } else { c.died.unwrap_or(st.year) };
    let half = c.genetic.iter().any(|&g| GENETIC[g].starts_with("half-"));
    let mut lines: Vec<String> = vec![];
    let mut head = format!("### {}", full_of(c));
    if let Some(h) = house_name(st, c.house) { head += &format!(" of {h}"); }
    head += &format!(": {}{}{}, level {}", if half { "Half-" } else { "" }, SINGULAR[c.people], if c.undead { " (undead)" } else { "" }, c.level);
    lines.push(head);
    lines.push(String::new());

    let origin = if !c.origin_story.is_empty() { c.origin_story.clone() } else if !c.origin.is_empty() { c.origin.to_string() } else { "unrecorded".to_string() };
    lines.push(format!("- **Origin:** {origin}"));
    let ambition = if c.ambition.is_empty() { "none yet".to_string() } else {
        let phrase = AMBITIONS.iter().find(|a| a.0 == c.ambition).map(|a| a.1).unwrap_or(c.ambition);
        match c.ambition_met { Some(y) => format!("{phrase}; fulfilled in year {y}"), None => phrase.to_string() }
    };
    lines.push(format!("- **Ambition:** {ambition}"));
    lines.push(String::new());

    let mut life = format!("Born in year {}", c.born);
    if c.alive { life += &format!("; living, aged {}.", c.age(st.year)); }
    else { let d = c.died.unwrap_or(st.year); life += &format!("; {}, year {d}, aged {}.", c.cause.as_deref().unwrap_or("died"), c.age(d)); }
    lines.push(life);
    if let Some((p, a, b)) = c.ruled {
        let to = b.map(|x| x.to_string()).unwrap_or_else(|| "the present".to_string());
        lines.push(format!("Reigned over {} from year {a} to {to}.", realm_name(st, p)));
    }
    lines.push(String::new());

    let cls_txt = c.classes.iter().filter(|x| x.1 > 0).map(|&(k, lv)| format!("{} {lv}", CLASSES[k].name)).collect::<Vec<_>>().join(", ");
    lines.push(format!("- **Classes:** {cls_txt} (total {})", c.level));
    if let Some(g) = c.gift { lines.push(format!("- **Gift:** {} ({})", GIFTS[g], GIFT_TEXT[g])); }
    if c.battle_arts > 0 {
        let arts: Vec<&str> = (0..c.battle_arts as usize).map(|i| BATTLE_ARTS[i.min(6)]).collect();
        lines.push(format!("- **Battle arts:** {}{}", arts.join(", "), if c.lost_art { "; and a lost art" } else { "" }));
    } else if c.lost_art {
        lines.push("- **Battle arts:** a lost art".to_string());
    }
    if let (Some(s), true) = (c.seal, c.hero) { lines.push(format!("- **Wardens' Hall:** {}", SEALS[s.min(6)])); }
    if c.alive {
        let nw = net_worth(st, cid);
        let rank = if nw > 30.0 || c.ruled.is_some() || !c.lordships.is_empty() { "upper" } else if nw > 3.0 || c.profession.is_some() { "middle" } else { "lower" };
        lines.push(format!("- **Wealth:** {} gold in hand; net worth {} (coin, gear, land and houses); {rank} class{}", commas(c.gold, 0), commas(nw, 1),
                           c.profession.map(|p| format!("; works as a {p}")).unwrap_or_default()));
    }
    let tier = st.tier(c);
    match (tier >= 0, st.school(c)) {
        (true, Some(s)) => {
            let spells: Vec<String> = (0..=tier).rev().take(3).map(|t| format!("{} {}", ROMAN[t as usize], SPELLS[s][t as usize])).collect();
            lines.push(format!("- **Magic:** {}, {} tier (peak {}). Spells: {}", SCHOOLS[s], TIER_WORDS[tier as usize], TIER_WORDS[tier as usize], spells.join(", ")));
        }
        _ => lines.push("- **Magic:** none".to_string()),
    }
    let cr = st.cr(c, false);
    lines.push(format!("- **Combat rating:** {} (peak {}); prowess {:.0}, magical talent {:.1}", commas(cr, 0), commas(c.peak_cr.max(cr), 0), c.prowess, c.talent));
    lines.push(format!("- **Attributes:** {}", (0..6).map(|a| format!("{} {}", ATTR_LABELS[a], c.attr(a, year) as i64)).collect::<Vec<_>>().join(", ")));
    lines.push(format!("- **Body and mind:** {}", (0..8).map(|a| format!("{} {}", ATTRIBUTE_LABELS[a], c.attributes[a] as i64)).collect::<Vec<_>>().join(", ")));
    let mut best: Vec<(f64, &str)> = (0..SKILLS.len().min(c.skills.len())).map(|s| (st.skill_val(c, s), SKILLS[s])).collect();
    best.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(b.1.cmp(a.1)));
    lines.push(format!("- **Best skills:** {}", best.iter().take(6).map(|(v, s)| format!("{s} {v:.0}")).collect::<Vec<_>>().join(", ")));
    if !c.perks.is_empty() {
        let from = c.perks.len().saturating_sub(10);
        let shown = c.perks[from..].iter().map(|&(s, i)| format!("{} ({})", PERKS[s][i], SKILLS[s])).collect::<Vec<_>>().join(", ");
        let more = if c.perks.len() > 10 { format!(", and {} more", c.perks.len() - 10) } else { String::new() };
        lines.push(format!("- **Perks:** {shown}{more}"));
    }

    let mut bits: Vec<String> = vec![];
    if let Some(p) = c.profession { bits.push(format!("profession {p}")); }
    if !c.guilds.is_empty() {
        let gs: Vec<String> = c.guilds.iter().filter_map(|&g| st.guilds_soc.get(g)).map(|g| g.name.clone()).collect();
        bits.push(format!("{} of {}", c.guild_rank.unwrap_or("member"), gs.join(", ")));
    }
    if let Some(r) = st.religions.get(c.religion) { bits.push(format!("follows {}", r.name)); }
    bits.push(format!("{} gold", commas(c.gold, 0)));
    if c.bounty > 0.0 && matches!(c.profession, Some("thief" | "assassin" | "bandit")) { bits.push(format!("bounty {}", commas(c.bounty, 0))); }
    lines.push(format!("- **Life:** {}", bits.join("; ")));

    if !c.fame.is_empty() {
        let mut f = c.fame.clone();
        f.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        lines.push(format!("- **Fame:** {}", f.iter().take(4).map(|(k, v)| format!("{k} {}", commas(*v, 0))).collect::<Vec<_>>().join(", ")));
    }
    if c.clan != NONE {
        if let Some(cl) = st.clans.get(c.clan) {
            let natures = c.natures.iter().map(|&(n, v)| format!("{} {}", NATURES[n].0, ROMAN[(v as usize).min(10)])).collect::<Vec<_>>().join(", ");
            let arts = c.sarts.iter().filter(|x| x.1 > 0).map(|&(a, v)| format!("{} {}", SHINOBI_ARTS[a].0, ROMAN[(v as usize).min(10)])).collect::<Vec<_>>().join(", ");
            lines.push(format!("- **Shinobi:** {} of {}; ki {}; natures {natures}; arts {arts}; signature {}; {} missions",
                               SHINOBI_RANKS[c.srank.min(4)], cl.name, commas(c.ki, 0), technique(c).1, c.missions));
        }
    }
    if c.beast.is_some() || c.merged.is_some() || !c.marks.is_empty() {
        let mut b: Vec<String> = vec![];
        if let Some(be) = &c.beast { b.push(format!("rides/keeps the {} {} since year {}", be.0, be.1, be.3)); }
        if let Some(m) = c.merged { b.push(format!("merged blood with a {m}; tames no more")); }
        if !c.marks.is_empty() { b.push(format!("bears {}", c.marks.join(" and "))); }
        lines.push(format!("- **Beasts and marks:** {}", b.join("; ")));
    }
    if c.element.is_some() {
        let txt = c.bend.iter().map(|&(e, v)| format!("{} {} ({v:.0})", ELEMENTS[e], BEND_TIER_NAMES[((v / 25.0) as usize).min(3)])).collect::<Vec<_>>().join(", ");
        let tribe = if c.tribe != NONE { st.tribes.get(c.tribe).map(|t| format!("; of {}", t.name)).unwrap_or_default() } else { String::new() };
        lines.push(format!("- **Bending:** {txt}{}{}{tribe}", c.high_art.map(|h| format!("; high art {h}")).unwrap_or_default(),
                           if c.concord { "; **the Concord**" } else { "" }));
    }
    {
        let mut b = vec![format!("stress {:.0} (level {})", c.stress, ((c.stress / 100.0) as i64).clamp(0, 3)), format!("happiness {:+.0}", c.happy), format!("health {:.1}/10", c.health)];
        if !c.coping.is_empty() { b.push(format!("copes as {}", c.coping.join(", "))); }
        let mut loves: Vec<&(usize, &'static str, f64)> = c.loves.iter().filter(|l| l.0 < st.chars.len()).collect();
        loves.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        if !loves.is_empty() { b.push(format!("loves {}", loves.iter().take(4).map(|l| format!("{} ({})", full_name(st, l.0), l.1)).collect::<Vec<_>>().join(", "))); }
        if let Some(p) = c.pet { b.push(format!("a {} called {}", p.0, p.1)); }
        if !c.vendettas.is_empty() {
            b.push(format!("sworn vengeance on {}", c.vendettas.iter().filter(|v| v.0 < st.chars.len()).map(|v| full_name(st, v.0)).collect::<Vec<_>>().join(", ")));
        }
        lines.push(format!("- **Heart:** {}; orientation {}", b.join("; "), ["straight", "gay", "bi"][(c.orientation as usize).min(2)]));
    }
    let edu = match c.education {
        Some((f, t)) if f < 5 && t < 4 => format!("{} ({} education, tier {})", EDUCATION[f][t], FOCUS_NAMES[f], t + 1),
        _ => "none yet".to_string(),
    };
    let perk_list = if c.perk_list.is_empty() { String::new() } else { format!("; lifestyle perks: {}", c.perk_list.iter().map(|p| p.2).collect::<Vec<_>>().join(", ")) };
    lines.push(format!("- **Education:** {edu}{perk_list}"));

    if c.alive && c.realm < st.realms.len() {
        let ruler = st.realms[c.realm].ruler;
        if ruler != NONE && ruler != cid && ruler < st.chars.len() {
            let terms = opinion_terms(st, cid, ruler);
            let total: i64 = terms.iter().map(|t| t.1).sum();
            let mut top = terms;
            top.sort_by_key(|t| std::cmp::Reverse(t.1.abs()));
            lines.push(format!("- **Opinion of their ruler:** {total:+} ({})", top.iter().take(6).map(|(n, v)| format!("{n} {v:+}")).collect::<Vec<_>>().join(", ")));
        }
    }
    if !st.cultures.is_empty() { lines.push(format!("- **Views on governance:** {}", ideology_label(views(st, c)))); }

    let mut tr: Vec<&str> = gene_names(&c.genetic);
    tr.sort();
    let mut pers: Vec<&str> = c.personality.iter().map(|&p| PERSONALITY[p]).collect();
    pers.sort();
    tr.extend(pers);
    let mut latent = gene_names(&c.latent);
    latent.sort();
    lines.push(format!("- **Traits:** {}{}", if tr.is_empty() { "none".to_string() } else { tr.join(", ") },
                       if latent.is_empty() { String::new() } else { format!("; carries latent {}", latent.join(", ")) }));
    let mut lim: Vec<&str> = vec![];
    for (g, text) in [("starblood", "tier cap broken (starblood)"), ("dragonblood", "level cap 130 (dragonblood)"), ("titanblood", "prowess cap broken (titanblood)"),
                      ("abyssblood", "talent cap broken (abyssblood)"), ("elderblood", "lifespan tripled (elderblood)")] {
        if c.has(gene(g)) { lim.push(text); }
    }
    let tier_cap = if c.custom { 10 } else { TIER_CAP[c.people] as i64 + c.has(gene("starblood")) as i64 };
    lines.push(format!("- **Limits:** tier cap {tier_cap}, level cap {}{}", c.level_cap(), if lim.is_empty() { String::new() } else { format!("; {}", lim.join(", ")) }));
    let leg = legacy_lines(st, c);
    if !leg.is_empty() { lines.push(format!("- **Legacies:** {}", leg.join("; "))); }
    let made: Vec<&world::works::Work> = st.works.iter().filter(|w| w.maker == cid && w.kind != "heirloom").collect();
    if !made.is_empty() {
        let mut best = made.clone();
        best.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap_or(std::cmp::Ordering::Equal));
        let shown = best.iter().take(5).map(|w| format!("*{}* ({}, {}{})", w.title, w.kind, w.year, if w.lost.is_some() { ", lost" } else { "" })).collect::<Vec<_>>().join("; ");
        lines.push(format!("- **Works ({}):** {shown}", made.len()));
    }
    let kept: Vec<String> = st.works.iter().filter(|w| w.kind == "heirloom" && w.holder == cid && w.lost.is_none()).map(|w| w.title.clone()).collect();
    if !kept.is_empty() { lines.push(format!("- **Heirlooms:** {}", plural_join(&kept, "; "))); }
    if let Some(l) = c.legitimacy {
        let state = if l < 30.0 { "corrupt" } else if l >= 70.0 { "trusted" } else { "accepted" };
        lines.push(format!("- **Governance:** legitimacy {l:.0} ({state}), tyranny {:.0}", c.tyranny));
    }
    if !c.lordships.is_empty() {
        lines.push(format!("- **Lordships held:** {}", c.lordships.iter().filter_map(|&s| st.towns.get(s)).map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")));
    }
    let mut br: Vec<&((usize, usize), u8)> = c.branch.iter().filter(|b| b.1 > 0).collect();
    br.sort_by(|a, b| CLASSES[a.0 .0].name.cmp(CLASSES[b.0 .0].name).then(a.0 .1.cmp(&b.0 .1)));
    let sk: Vec<String> = br.iter().map(|&&((cl, b), n)| {
        let (bname, _, nodes) = &CLASSES[cl].branches[b];
        format!("{} / {bname} {n}/4 ({})", CLASSES[cl].name, nodes[..(n as usize).min(4)].join(", "))
    }).collect();
    if !sk.is_empty() { lines.push(format!("- **Skill trees:** {}", sk.join("; "))); }
    if !c.capstones.is_empty() { lines.push(format!("- **Capstones:** {}", c.capstones.iter().map(|&x| CLASSES[x].capstone).collect::<Vec<_>>().join(", "))); }
    if !c.items.is_empty() {
        let mut items = c.items.clone();
        items.sort_by_key(|&i| std::cmp::Reverse(st.items[i].rarity));
        lines.push(format!("- **Equipment:** {}", items.iter().map(|&i| st.items[i].label()).collect::<Vec<_>>().join(", ")));
    }
    if !c.deeds.is_empty() {
        let mut d = c.deeds.clone();
        d.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        lines.push(format!("- **Deeds:** {}; renown {}", d.iter().map(|(k, v)| format!("{k} {}", *v as i64)).collect::<Vec<_>>().join(", "), commas(c.renown, 0)));
    }
    if !c.titles.is_empty() { lines.push(format!("- **Offices:** {}", c.titles.iter().take(6).cloned().collect::<Vec<_>>().join("; "))); }
    let mut fam: Vec<String> = vec![];
    let parents: Vec<String> = [c.father, c.mother].iter().filter(|&&p| p != NONE && p < st.chars.len()).map(|&p| full_name(st, p)).collect();
    if !parents.is_empty() { fam.push(format!("child of {}", parents.join(" and "))); }
    if c.spouse != NONE && c.spouse < st.chars.len() {
        let sp = &st.chars[c.spouse];
        fam.push(format!("married {}{}", full_of(sp), house_name(st, sp.house).map(|h| format!(" of {h}")).unwrap_or_default()));
    }
    if !c.children.is_empty() {
        let kids: Vec<String> = c.children.iter().filter(|&&k| k < st.chars.len()).map(|&k| full_name(st, k)).collect();
        fam.push(format!("{} children: {}", kids.len(), kids.iter().take(8).cloned().collect::<Vec<_>>().join(", ")));
    }
    if !fam.is_empty() { lines.push(format!("- **Family:** {}", fam.join("; "))); }
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_runs_on_a_founded_world() {
        let (s, i) = world::engine::seed_state(4);
        let mut st = State::new_world(480, 270, s, i);
        st.run_years(150);
        assert!(!realm_rows(&st).is_empty());
        assert!(!ledger_rows(&st, st.year - 100).is_empty());
        assert!(!tx_lines(&st, 1500).is_empty());
        assert!(!econ_rows(&st).is_empty());
        assert!(!class_rows(&st).is_empty());
        assert!(!rich_rows(&st, 40).is_empty());
        let _ = party_rows(&st);
        let _ = quest_rows(&st);
        let _ = hall_log(&st, 400);
        assert!(war_lines(&st).len() >= 4);
        let mut n = 0;
        for c in 0..st.chars.len() {
            let s = sheet(&st, c);
            assert!(s.starts_with("### ") && s.contains("- **Origin:**") && s.contains("- **Ambition:**"));
            let _ = full_name(&st, c);
            let _ = net_worth(&st, c);
            let ch = &st.chars[c];
            if ch.alive && ch.realm < st.realms.len() {
                let r = st.realms[ch.realm].ruler;
                if r != NONE && r != c {
                    let sum: i64 = opinion_terms(&st, c, r).iter().map(|t| t.1).sum();
                    assert_eq!(sum, st.opinion(c, r) as i64, "opinion breakdown of {c} towards {r}: {:?}", opinion_terms(&st, c, r));
                }
            }
            n += 1;
        }
        println!("{n} sheets; year {}", st.year);
        let k = (0..st.realms.len()).find(|&k| !st.realms[k].extinct && st.realms[k].ruler != NONE).unwrap();
        println!("{}", sheet(&st, st.realms[k].ruler));
        let hero = st.chars.iter().position(|c| c.alive && c.hero).unwrap_or(0);
        println!("{}", sheet(&st, hero));
        let dead = st.chars.iter().position(|c| !c.alive && c.ruled.is_some()).unwrap_or(0);
        println!("{}", sheet(&st, dead));
        for r in realm_rows(&st).iter().take(3) { println!("{r:?}"); }
        for r in ledger_rows(&st, st.year - 100).iter().take(2) { println!("{r:?}"); }
        for r in econ_rows(&st).iter().take(2) { println!("{r:?}"); }
        for r in class_rows(&st).iter().take(3) { println!("{r:?}"); }
        for r in rich_rows(&st, 3) { println!("{r:?}"); }
        for r in party_rows(&st).iter().take(2) { println!("{r:?}"); }
        for r in quest_rows(&st).iter().take(2) { println!("{r:?}"); }
        for l in hall_log(&st, 3) { println!("{l}"); }
        for l in tx_lines(&st, 3) { println!("{l:?}"); }
        for l in war_lines(&st).iter().take(12) { println!("{l}"); }
    }
}
