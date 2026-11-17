//! The world as Python reads it: one snapshot of everything the screens and the books show, in the Python engine's
//! own names, so the TUI and the books work on it unchanged.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::people::*;
use crate::state::State;
use crate::tables::*;

fn o(x: usize) -> Option<usize> { if x == NONE { None } else { Some(x) } }

macro_rules! d { ($py:expr, $( $k:expr => $v:expr ),* $(,)?) => {{ let d = PyDict::new($py); $( d.set_item($k, $v).expect("set_item"); )* d }} }

impl State {
    pub fn net_worth(&self, c: &Character) -> f64 {
        let mut w = c.gold.max(0.0) + c.items.iter().map(|&i| self.items[i].value).sum::<f64>() * 0.01;
        for &t in &c.lordships { let s = &self.towns[t]; if s.alive { w += 0.25 * (s.land_value * if matches!(self.realms[s.realm].gov, "tribal" | "clan") { 0.35 } else { 0.85 } + s.housing.unwrap_or(0.0) * 0.02 * 0.35); } }
        if c.ruled.is_some_and(|r| r.2.is_none()) && c.realm < self.realms.len() { w += 0.05 * self.realms[c.realm].gold.max(0.0); }
        w
    }

    fn item_dict<'py>(&self, py: Python<'py>, i: usize) -> PyResult<Bound<'py, PyDict>> {
        let it = &self.items[i];
        Ok(d!(py, "id" => i, "name" => &it.name, "rarity" => it.rarity, "year" => it.year, "origin" => &it.origin, "special" => it.special,
              "note" => &it.note, "bonus" => it.bonus, "desc" => it.desc.clone(), "kind" => it.kind, "value" => it.value, "label" => it.label(),
              "enchants" => it.enchants.iter().map(|(e, t)| (*e, *t)).collect::<Vec<_>>(), "bearers" => it.bearers.clone(),
              "maker" => o(it.maker), "enchanter" => o(it.enchanter)))
    }

    /// Whether the books need a character in full: the living, and the dead who left a mark.
    pub fn notable(&self, c: &Character) -> bool {
        c.alive || c.hero || c.ruled.is_some() || c.renown > 0.0 || c.concord || c.pirate || c.clan != NONE || c.element.is_some()
            || c.beast.is_some() || c.custom || !c.duchies.is_empty() || !c.items.is_empty()
    }

    /// One character's full record, as the snapshot gives the living.
    pub fn character_full<'py>(&self, py: Python<'py>, cid: usize) -> PyResult<Bound<'py, PyDict>> { self.char_dict(py, &self.chars[cid]) }

    fn char_dict<'py>(&self, py: Python<'py>, c: &Character) -> PyResult<Bound<'py, PyDict>> {
        let y = self.year;
        let tier = self.tier(c);
        let school = self.school(c);
        let attrs = PyDict::new(py);
        let attr_now = PyDict::new(py);
        for (i, a) in ATTRS.iter().enumerate() { attrs.set_item(*a, c.attrs[i] as i64)?; attr_now.set_item(*a, c.attr(i, if c.alive { y } else { c.died.unwrap_or(y) }) as i64)?; }
        let attributes = PyDict::new(py);
        for (i, a) in ATTRIBUTES.iter().enumerate() { attributes.set_item(*a, c.attributes[i] as i64)?; }
        let skills = PyDict::new(py);
        for (i, s) in SKILLS.iter().enumerate() { skills.set_item(*s, c.skills[i])?; }
        let skill_now = PyDict::new(py);
        for (i, s) in SKILLS.iter().enumerate() { skill_now.set_item(*s, self.skill_val(c, i))?; }
        let deeds = PyDict::new(py); for (k, v) in &c.deeds { deeds.set_item(*k, *v as i64)?; }
        let fame = PyDict::new(py); for (k, v) in &c.fame { fame.set_item(*k, *v)?; }
        let legacy = PyDict::new(py); for &(k, v) in &c.legacy { legacy.set_item(LEGACY_KINDS[k], v)?; }
        let loves = PyDict::new(py); for &(w, k, s) in &c.loves { loves.set_item(w, (k, s))?; }
        let bend = PyDict::new(py); for &(e, lv) in &c.bend { bend.set_item(ELEMENTS[e], lv)?; }
        let nature_lv = PyDict::new(py); for &(n, lv) in &c.natures { nature_lv.set_item(NATURES[n].0, lv)?; }
        let arts = PyDict::new(py); for &(a, lv) in &c.sarts { arts.set_item(SHINOBI_ARTS[a].0, lv)?; }
        let spells: Vec<String> = match school { Some(s) if tier >= 0 => (0..=tier).rev().take(3).map(|t| format!("{} {}", ROMAN[t as usize], SPELLS[s][t as usize])).collect(), _ => vec![] };
        let tech = crate::shinobi::technique(c);
        Ok(d!(py, "id" => c.id, "name" => &c.name, "people" => c.people, "female" => c.female, "born" => c.born, "realm" => c.realm,
            "house" => o(c.house), "father" => o(c.father), "mother" => o(c.mother), "spouse" => o(c.spouse), "children" => c.children.clone(),
            "genetic" => c.genetic.iter().map(|&g| GENETIC[g]).collect::<Vec<_>>(), "latent" => c.latent.iter().map(|&g| GENETIC[g]).collect::<Vec<_>>(),
            "personality" => c.personality.iter().map(|&p| PERSONALITY[p]).collect::<Vec<_>>(), "attrs" => attrs, "attr_now" => attr_now,
            "prowess" => c.prowess, "talent" => c.talent, "life" => c.life,
            "classes" => c.classes.iter().map(|&(k, n)| (CLASSES[k].name, n)).collect::<Vec<_>>(), "level" => c.level, "xp" => c.xp, "hp" => c.hp,
            "renown" => c.renown, "capstones" => c.capstones.iter().map(|&k| CLASSES[k].name).collect::<Vec<_>>(),
            "items" => c.items.clone(), "deeds" => deeds, "titles" => c.titles.clone(), "epithet" => c.epithet.clone(), "hero" => c.hero,
            "undead" => c.undead, "alive" => c.alive, "died" => c.died, "cause" => c.cause.clone(), "home" => o(c.home), "regnal" => c.regnal.clone(),
            "ruled" => c.ruled.map(|(k, a, b)| (k, a, b)), "legacy" => legacy, "legitimacy" => c.legitimacy, "tyranny" => c.tyranny,
            "lordships" => c.lordships.clone(), "gold" => c.gold, "fame" => fame, "profession" => c.profession, "bounty" => c.bounty,
            "religion" => c.religion, "guilds" => c.guilds.clone(), "guild_rank" => c.guild_rank, "clan" => o(c.clan), "element" => c.element,
            "tribe" => o(c.tribe).map(|t| ELEMENTS[t]), "concord" => c.concord, "gift" => c.gift.map(|g| GIFTS[g]),
            "battle_arts" => (0..c.battle_arts as usize).map(|i| ["Steel Breath", "Iron Fortress Stance", "Flowing Step", "Twin Fang Strike", "Mind's-Eye Parry", "Thousand Cuts", "Heaven-Cleaver"][i.min(6)]).collect::<Vec<_>>(),
            "lost_art" => if c.lost_art { Some("a lost art") } else { None }, "seal" => c.seal.map(|s| s as i64).unwrap_or(-1),
            "attributes" => attributes, "skills" => skills, "skill_now" => skill_now,
            "perks" => c.perks.iter().map(|&(s, i)| (SKILLS[s], PERKS[s][i])).collect::<Vec<_>>(),
            "stress" => c.stress, "happy" => c.happy, "health" => c.health, "grief" => c.grief, "loves" => loves, "coping" => c.coping.clone(),
            "pet" => c.pet.map(|(k, n, s, l)| (k, n, s, l)), "marks" => c.marks.clone(),
            "beast" => c.beast.as_ref().map(|b| (b.0, b.1.clone(), b.2, b.3, b.4)), "merged" => c.merged,
            "bend" => bend, "high_art" => c.high_art, "ki" => c.ki, "nature_lv" => nature_lv, "arts" => arts, "srank" => c.srank, "missions" => c.missions,
            "technique" => (tech.0, tech.1), "pirate" => c.pirate, "ship" => (c.ship.0.clone(), c.ship.1, c.ship.2), "infamy" => c.infamy,
            "resume" => c.resume.clone(), "education" => c.education.map(|(f, t)| (["dip", "mar", "ste", "intr", "lrn"][f], t)),
            "perk_list" => c.perk_list.clone(), "duchies" => c.duchies.len(), "custom" => c.custom, "peak_cr" => c.peak_cr,
            "tier" => tier, "school" => school, "cr" => self.cr(c, false), "cr_now" => self.cr(c, true), "known_spells" => spells,
            "net_worth" => if c.alive { self.net_worth(c) } else { 0.0 }, "kills" => c.kills, "merit" => c.merit, "orientation" => ["straight", "gay", "bi"][c.orientation as usize],
            "branch" => { let b = PyDict::new(py); for &((ci, bi), n) in &c.branch { b.set_item((CLASSES[ci].name, bi), n).unwrap(); } b },
            "vendettas" => c.vendettas.iter().map(|v| v.0).collect::<Vec<_>>(),
            "eff_talent" => self.eff_talent(c), "eff_prowess" => self.eff_prowess(c), "hp_mult" => self.hp_mult(c), "gate" => self.power_gate(c, false), "gate_martial" => self.power_gate(c, true)))
    }

    pub fn snapshot<'py>(&self, py: Python<'py>, since: i64) -> PyResult<Bound<'py, PyDict>> {
        let s = PyDict::new(py);
        s.set_item("year", self.year)?;
        s.set_item("W", self.map.w)?;
        s.set_item("H", self.map.h)?;
        let realms = PyList::empty(py);
        for k in &self.realms {
            let towns = self.alive_towns_of(k.id);
            let pop: f64 = towns.iter().map(|&t| self.towns[t].pop).sum::<f64>().max(1.0);
            // the realm's economy by class (stewardship.realm_economy)
            let classes = PyDict::new(py);
            let mut tot_assets = 0.0;
            let mut rows = vec![];
            for (ci, cl) in ["lower", "middle", "upper"].iter().enumerate() {
                let (mut n, mut inc, mut pov, mut land, mut house, mut stock, mut coin) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
                for &t in &towns {
                    let tt = &self.towns[t];
                    let (pp, per, pv) = tt.cls[ci];
                    n += pp; inc += pp * per; pov += pp * pv;
                    let communal = matches!(k.gov, "tribal" | "clan");
                    land += tt.land_value * [if communal { 0.5 } else { 0.0 }, 0.15, if communal { 0.35 } else { 0.85 }][ci];
                    house += tt.housing.unwrap_or(0.0) * 0.02 * [0.3, 0.35, 0.35][ci];
                    stock += tt.stock[ci];
                    coin += tt.coin.unwrap_or(0.0) * [0.10, 0.35, 0.55][ci];
                }
                tot_assets += land + house + stock + coin;
                rows.push((cl, n, inc, pov, land, house, stock, coin));
            }
            let mut poverty = 0.0;
            for (cl, n, inc, pov, land, house, stock, coin) in rows {
                let assets = d!(py, "land" => land, "housing" => house, "stock" => stock, "coin" => coin);
                let sum = land + house + stock + coin;
                poverty += pov;
                classes.set_item(cl, d!(py, "people" => n, "income" => inc / n.max(1.0), "poverty" => pov / n.max(1.0), "assets" => assets,
                                         "per_head" => sum / n.max(1.0), "wealth_share" => sum / tot_assets.max(1e-9)))?;
            }
            let econ = d!(py, "classes" => classes, "poverty" => poverty / pop, "distress" => k.econ_distress);
            let court = k.court.as_ref().map(|c| d!(py, "since" => c.since, "grandeur" => c.grandeur, "type" => c.kind,
                "amen" => d!(py, "food" => c.amen[0], "lodging" => c.amen[1], "fashion" => c.amen[2], "servants" => c.amen[3]),
                "pos" => { let p = PyDict::new(py); for (n, id) in &c.pos { p.set_item(*n, *id).unwrap(); } p }, "events" => c.events.clone(),
                "artifacts" => {
                    let mut a: Vec<String> = k.treasury.iter().filter(|&&i| self.items[i].rarity >= 3).map(|&i| self.items[i].name.clone()).collect();
                    if k.ruler != NONE { a.extend(self.chars[k.ruler].items.iter().filter(|&&i| self.items[i].rarity >= 3).map(|&i| self.items[i].name.clone())); }
                    a
                }));
            let maa = PyDict::new(py); for &(m, n) in &k.maa { maa.set_item(MAA[m].0, n)?; }
            let parts = d!(py, "hide" => k.parts[0], "ash" => k.parts[1], "horn" => k.parts[2], "scale" => k.parts[3]);
            let council = d!(py, "marshal" => o(k.council[0]), "steward" => o(k.council[1]), "spymaster" => o(k.council[2]), "mage" => o(k.council[3]),
                             "loremaster" => o(k.council[4]), "admiral" => o(k.council[5]));
            let reigns: Vec<Bound<PyDict>> = k.reigns.iter().map(|r| d!(py, "id" => r.id, "start" => r.start, "end" => r.end, "how" => r.how.clone(),
                "epithet" => r.epithet.clone(), "legit" => r.legit, "tyranny" => r.tyranny)).collect();
            realms.append(d!(py, "id" => k.id, "people" => k.people, "name" => &k.name, "short" => &k.short, "rank" => k.rank, "gov" => k.gov, "law" => k.law,
                "color" => (k.color[0], k.color[1], k.color[2]), "ruler" => o(k.ruler), "house" => o(k.house), "regent" => o(k.regent), "place" => &k.place,
                "extinct" => k.extinct, "gold" => k.gold, "income" => k.income, "tax_now" => k.tax_now, "collect" => k.collect, "real_gdp" => k.real_gdp,
                "money" => k.money, "rate" => k.rate, "credit" => k.credit.unwrap_or(0.0), "coin" => k.credit.is_some(), "partners" => k.partners, "top_scroll" => k.top_scroll,
                "parts" => parts, "bound_share" => k.bound_share, "army_power" => k.army_power, "levy" => k.levy, "maa" => maa, "council" => council,
                "court" => court, "econ" => econ, "faith" => k.faith, "agenda" => k.agenda, "crown" => k.crown_law, "overlord" => o(k.overlord),
                "parent" => o(k.parent), "founded_year" => k.founded_year, "ended" => k.ended, "reigns" => reigns, "civil_wars" => k.civil_wars,
                "corruption" => k.corruption, "warships" => k.warships, "merchants" => k.merchants, "ship_tier" => k.ship_tier, "colonies" => k.colonies,
                "great_works" => k.great_works.clone(), "treasury" => k.treasury.clone(), "market" => k.market.clone(), "dragons" => k.dragons,
                "domain_penalty" => k.domain_penalty, "vassal_limit" => k.vassal_limit, "domain_limit" => k.domain_limit, "factions" => k.factions.iter().map(|f| d!(py, "kind" => f.0, "land" => if f.1 >= 0 { Some(f.1) } else { None }, "members" => f.2.clone(), "leader" => f.3, "power" => f.4)).collect::<Vec<_>>(),
                "known" => k.known.clone(), "home_land" => k.home_land, "realm_tier" => k.realm_tier, "great" => k.great.clone(),
                "corrupt_seats" => k.corrupt_seats.iter().map(|&i| ["marshal", "steward", "spymaster", "mage", "loremaster", "admiral"][i]).collect::<Vec<_>>(),
                "over_vassals" => k.over_vassals, "aggr" => k.aggr, "agreement" => k.agreement, "battle_mult" => k.battle_mult, "defaults" => k.defaults,
                "discoveries" => k.discoveries.iter().map(|&(y, l, by)| (y, l, o(by))).collect::<Vec<_>>(), "garrison_bonus" => k.garrison_bonus, "last_default" => k.last_default, "last_forced" => k.last_forced,
                "tax_mult" => k.tax_mult, "tax_relief" => k.tax_relief, "trade_net" => k.trade_net, "trade_val" => k.trade_val, "mint_skill" => k.mint_skill,
                "allies" => k.allies.clone(), "reform" => k.reform, "tolerant" => k.tolerant, "expansionist" => k.expansionist, "protectionist" => k.trade_protectionist,
                "resources" => { let r = PyDict::new(py); for &x in &k.held { r.set_item(RESOURCES[x].name, 1)?; } r },
                "sector_boost" => { let b = PyDict::new(py); for (i, n) in ["farm", "mine", "craft", "trade", "clergy", "service", "arcane"].iter().enumerate() { if k.sector_boost[i] > 0.0 { b.set_item(*n, k.sector_boost[i])?; } } b }))?;
        }
        s.set_item("kingdoms", realms)?;
        let towns = PyList::empty(py);
        for t in &self.towns {
            towns.append(d!(py, "id" => t.id, "name" => &t.name, "faction" => t.realm, "culture" => t.culture, "pos" => t.pos, "pop" => t.pop, "wealth" => t.wealth,
                "walls" => t.walls, "alive" => t.alive, "founded" => t.founded, "capital" => t.capital, "landmass" => t.landmass, "port" => t.port,
                "lord" => o(t.lord), "famine" => t.famine, "poverty" => t.poverty, "crowded" => t.crowded, "jobless" => t.jobless, "threat" => t.threat,
                "income" => t.income, "real_income" => t.real_income, "granary" => t.granary, "soil" => t.soil, "colony" => t.colony,
                "occupied_by" => o(t.occupied_by), "rising" => t.rising, "mood" => d!(py, "peasants" => t.mood_p.unwrap_or(10.0), "burghers" => t.mood_b),
                "crime" => t.crime, "housing" => t.housing, "food_prod" => t.food_prod, "faith" => t.faith.clone(), "bld" => { let b = PyDict::new(py); for (i, &lv) in t.bld.iter().enumerate() { if lv > 0 { b.set_item(crate::stewardship::BUILDINGS[i].0, lv)?; } } b },
                "magic" => t.magic, "guard" => t.guard, "garrison" => t.garrison, "game" => t.game, "duchy" => t.duchy))?;
        }
        s.set_item("towns", towns)?;
        // full records for the living and the notable dead; the rest of the dead as shades (World.character_full
        // fetches one in full when a screen asks for more than a shade holds)
        let chars = PyList::empty(py);
        let mut shades = vec![];
        for c in &self.chars {
            if self.notable(c) { chars.append(self.char_dict(py, c)?)?; continue; }
            let cls = c.classes.last().map(|x| CLASSES[x.0].name).unwrap_or("Commoner");
            shades.push(((c.id, &c.name, c.regnal.clone(), c.epithet.clone(), o(c.house), c.people),
                         (c.female, c.born, c.died, c.cause.clone(), o(c.father), o(c.mother)),
                         (o(c.spouse), c.children.clone(), c.realm, c.level, c.profession, cls, c.genetic.iter().map(|&g| GENETIC[g]).collect::<Vec<_>>())));
        }
        s.set_item("chars", chars)?;
        s.set_item("shades", shades)?;
        let houses: Vec<Bound<PyDict>> = self.houses.iter().map(|h| d!(py, "id" => h.id, "name" => &h.name, "people" => h.people, "founder" => h.founder,
            "founded" => h.founded, "prestige" => h.prestige, "rulers" => h.rulers, "track" => &h.track, "legacy" => h.legacy, "origin" => &h.origin, "curse" => if h.cursed { Some(h.name.clone()) } else { None }, "reign_years" => h.reign_years, "parent" => None::<usize>, "dynasty" => h.id)).collect();
        s.set_item("houses", houses)?;
        let items: Vec<Bound<PyDict>> = (0..self.items.len()).map(|i| self.item_dict(py, i)).collect::<PyResult<_>>()?;
        s.set_item("items", items)?;
        let monsters: Vec<Bound<PyDict>> = self.monsters.iter().enumerate().map(|(i, m)| d!(py, "mid" => i, "kind" => m.kind, "name" => &m.name, "pos" => m.pos,
            "alive" => m.alive, "base" => m.base, "power" => m.power, "hoard" => m.hoard, "born" => m.born, "landmass" => m.landmass, "world" => m.world,
            "title" => m.title, "traits" => m.traits.clone(), "temper" => m.temper, "size" => m.size, "kills" => m.kills, "items" => m.items.clone(),
            "slain_by" => o(m.slain_by), "sea" => m.sea, "gate" => m.gate, "lifespan" => m.lifespan, "generation" => m.generation,
            "tributaries" => m.tributaries.iter().map(|t| t.0).collect::<Vec<_>>(), "bounty" => m.bounties.iter().map(|b| b.1).sum::<f64>())).collect();
        s.set_item("monsters", monsters)?;
        let ruins: Vec<Bound<PyDict>> = self.ruins.iter().map(|r| d!(py, "name" => &r.name, "pos" => r.pos, "riches" => r.riches, "depth" => r.depth,
            "loot" => r.loot.clone(), "kind" => r.kind, "floors" => r.floors, "grand" => r.grand, "floor" => r.floor, "conquered" => r.conquered,
            "boss" => (r.boss.0.clone(), r.boss.1, r.boss.2), "conquests" => r.conquests.clone())).collect();
        s.set_item("ruins", ruins)?;
        let lands: Vec<Bound<PyDict>> = self.map.lands.iter().map(|l| d!(py, "id" => l.id, "name" => &l.name, "size" => l.size, "kind" => l.kind,
            "center" => l.center, "found_by" => l.found_by.clone())).collect();
        s.set_item("lands", lands)?;
        let cultures: Vec<Bound<PyDict>> = self.cultures.iter().map(|c| d!(py, "people" => c.people, "era" => c.era, "innovations" => c.learned.clone(),
            "traditions" => c.traditions.iter().map(|&t| TRADITIONS[t].0).collect::<Vec<_>>(), "trend" => c.trend.to_vec(), "golden" => c.golden, "dark" => c.dark,
            "history" => c.history.clone(), "progress" => c.progress, "focus" => c.focus.map(|f| INNOVATIONS[f].name))).collect();
        s.set_item("cultures", cultures)?;
        let religions: Vec<Bound<PyDict>> = self.religions.iter().map(|r| d!(py, "id" => r.id, "name" => &r.name, "deity" => &r.deity, "tenets" => r.tenets.clone(),
            "founder" => o(r.founder), "founded" => r.founded, "people" => r.people, "parent" => o(r.parent), "holy_site" => o(r.holy_site), "peak" => r.peak,
            "followers" => r.followers, "dead" => r.dead)).collect();
        s.set_item("religions", religions)?;
        s.set_item("guilds_soc", self.guilds_soc.iter().map(|g| (g.name.clone(), g.kind, o(g.people), g.members.clone(), o(g.master), g.gold)).collect::<Vec<_>>())?;
        s.set_item("epidemics", self.epidemics.iter().map(|e| (e.name.clone(), e.start, e.end, e.fatality, e.great, e.deaths, e.towns.len())).collect::<Vec<_>>())?;
        let wars: Vec<Bound<PyDict>> = self.wars.iter().map(|w| d!(py, "attacker" => w.attacker, "defender" => w.defender, "cb" => &w.cb, "start" => w.start, "score" => w.score)).collect();
        s.set_item("wars", wars)?;
        let wl: Vec<Bound<PyDict>> = self.war_log.iter().map(|w| d!(py, "year" => w.0, "attacker" => &w.1, "defender" => &w.2, "result" => &w.3, "years" => w.4, "outcome" => w.5, "cb" => &w.6)).collect();
        s.set_item("war_log", wl)?;
        s.set_item("battles_fought", self.battles_fought.clone())?;
        s.set_item("armies", self.armies.iter().map(|a| (a.name.clone(), a.men, a.pos, a.siege, o(a.realm), a.rebel)).collect::<Vec<_>>())?;
        s.set_item("war", self.war.clone())?;
        s.set_item("relation", self.relation.clone())?;
        s.set_item("war_since", self.war_since.clone())?;
        s.set_item("famines", self.famines.clone())?;
        s.set_item("risings", self.risings.clone())?;
        s.set_item("bankruptcies", self.bankruptcies.clone())?;
        s.set_item("defaults", self.stats_defaults.clone())?;
        s.set_item("forced", self.stats_forced.clone())?;
        s.set_item("mined", self.mined)?;
        s.set_item("migrants", self.migrants)?;
        s.set_item("naval_battles", self.naval_battles.clone())?;
        s.set_item("tier_events", self.tier_events.clone())?;
        s.set_item("rites", self.rites.clone())?;
        s.set_item("myths", self.myths.clone())?;
        s.set_item("sovereign_line", self.sovereign_line.clone())?;
        s.set_item("calamity_log", self.calamity_log.clone())?;
        s.set_item("world_tier", self.world_tier)?;
        s.set_item("people_tier", self.people_tier.to_vec())?;
        s.set_item("breakaways", self.breakaways.clone())?;
        s.set_item("dungeon_log", self.dungeon_log.clone())?;
        s.set_item("dragon_riders", self.dragon_riders.clone())?;
        s.set_item("merges", self.merges.clone())?;
        s.set_item("pirates", self.pirates.clone())?;
        s.set_item("pirate_kings", self.pirate_kings.clone())?;
        s.set_item("havens", self.havens.iter().map(|h| (h.name.clone(), h.pos, h.land)).collect::<Vec<_>>())?;
        s.set_item("tribes", self.tribes.iter().map(|t| (t.name, ELEMENTS[t.element], t.pos, t.members.clone(), o(t.chief))).collect::<Vec<_>>())?;
        s.set_item("clans", self.clans.iter().map(|c| (c.name, c.pos, c.members.clone(), o(c.head), c.gold, c.standing, c.pact, c.won, c.lost)).collect::<Vec<_>>())?;
        s.set_item("concord", o(self.concord))?;
        s.set_item("concord_line", self.concord_line.iter().map(|c| (c.id, c.born, ELEMENTS[c.element], c.need.clone(), c.revealed, c.died.clone(), c.deeds.clone())).collect::<Vec<_>>())?;
        let cycles: Vec<Bound<PyDict>> = self.cycles.iter().map(|c| d!(py, "villain" => c.villain.clone(), "title" => c.title, "landmass" => c.landmass,
            "born" => c.born, "power" => c.power, "period" => c.period, "next_return" => c.next_return, "state" => c.state, "ages" => c.ages.clone(),
            "hero" => o(c.hero), "seer" => o(c.seer), "relic" => self.items[c.relic].name.clone(), "blade" => o(c.blade).map(|b| self.items[b].name.clone()))).collect();
        s.set_item("cycles", cycles)?;
        let curses: Vec<Bound<PyDict>> = self.curses.iter().map(|c| {
            let d = d!(py, "kind" => c.kind, "source" => c.source.clone(), "since" => c.since, "lifted" => c.lifted);
            match c.kind {
                "blood" => d.set_item("house", self.houses[c.house].name.clone()).unwrap(),
                "item" => d.set_item("item", self.items[c.item].name.clone()).unwrap(),
                "land" => { d.set_item("land", c.land).unwrap(); d.set_item("pos", c.pos).unwrap(); d.set_item("radius", c.radius).unwrap(); }
                _ => d.set_item("cycle", c.cycle).unwrap(),
            }
            d
        }).collect();
        s.set_item("curses", curses)?;
        let hall = d!(py, "parties" => self.hall.parties.iter().map(|p| (p.name.clone(), p.rank, p.members.clone(), p.done, p.lost, p.formed)).collect::<Vec<_>>(),
            "quests" => self.hall.quests.iter().map(|q| (q.rank, q.kind, q.title.clone(), q.reward, q.posted, q.expires)).collect::<Vec<_>>(),
            "log" => self.hall.log.clone());
        s.set_item("hall", hall)?;
        s.set_item("hero_guilds", self.hero_guilds.iter().map(|g| (g.heroes_total, g.deaths_total, g.acts.to_vec(), g.hunts, g.hunt_wins, g.slain, g.q.len(),
            { let mut v: Vec<(String, f64, u32)> = g.branch_val.iter().filter(|x| x.1 .1 >= 20).map(|(&(c, b), &(m, n))| (format!("{} / {}", CLASSES[c].name, CLASSES[c].branches[b].0), m, n)).collect();
              v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); v.truncate(3); v })).collect::<Vec<_>>())?;
        s.set_item("eco", self.eco.species.iter().map(|sp| d!(py, "name" => sp.name.clone(), "troph" => sp.troph, "mythic" => sp.mythic, "extinct" => sp.extinct,
            "origin" => sp.origin.clone(), "peak" => sp.peak, "size" => sp.size, "tame" => sp.tame, "danger" => sp.danger)).collect::<Vec<_>>())?;
        s.set_item("eco_shape", (self.eco.eh, self.eco.ew))?;
        s.set_item("eco_pop", self.eco.pop.concat())?;
        s.set_item("eco_events", self.eco.events.clone())?;
        s.set_item("eco_regions", self.eco.region_names.iter().map(|((a, b), n)| ((*a, *b), n.clone())).collect::<Vec<_>>())?;
        s.set_item("greatest", self.greatest.clone())?;
        s.set_item("gates", self.gates.as_ref().map(|g| (g.open, g.start, g.end, g.lord_name.clone(), g.count)))?;
        s.set_item("gate_history", self.gate_history.clone())?;
        s.set_item("blight", self.blight)?;
        let duchies = PyDict::new(py);
        for (k, d) in &self.duchies { duchies.set_item(*k, d!(py, "name" => &d.name, "holder" => o(d.holder), "kt" => d.kt, "since" => d.since, "land" => d.land, "ctrl" => o(d.ctrl)))?; }
        s.set_item("duchies", duchies)?;
        let kt = PyDict::new(py);
        for (k, d) in &self.ktitles { kt.set_item(*k, d!(py, "name" => &d.name, "ctrl" => o(d.ctrl)))?; }
        s.set_item("ktitles", kt)?;
        let op = PyDict::new(py);
        for c in self.chars.iter().filter(|c| c.alive && !c.memories.is_empty()) {
            let mut by: Vec<(usize, Vec<(&str, f64, f64)>)> = vec![];
            for m in &c.memories { match by.iter_mut().find(|x| x.0 == m.0) { Some(x) => x.1.push((m.1, m.2, m.3)), None => by.push((m.0, vec![(m.1, m.2, m.3)])) } }
            for (about, v) in by { op.set_item((c.id, about), v)?; }
        }
        s.set_item("op_mods", op)?;
        let works: Vec<Bound<PyDict>> = self.works.iter().map(|w| d!(py, "id" => w.id, "kind" => w.kind, "title" => &w.title, "maker" => w.maker,
            "year" => w.year, "subject" => &w.subject, "quality" => w.quality, "realm" => o(w.realm), "town" => o(w.town), "house" => o(w.house),
            "holder" => o(w.holder), "hero" => o(w.hero), "sector" => crate::works::sector_name(w.sector), "faith" => o(w.faith), "field" => w.field, "copies" => w.copies.clone(),
            "lost" => w.lost, "history" => w.history.clone())).collect();
        s.set_item("works", works)?;
        let ledger = PyDict::new(py);
        for ((y, k), row) in self.ledger.range((since, 0)..) { let r = PyDict::new(py); for (c, v) in row { r.set_item(*c, *v)?; } ledger.set_item((*y, *k), r)?; }
        s.set_item("ledger", ledger)?;
        s.set_item("tx", self.tx.iter().map(|t| (t.0, t.1, t.2, t.3, t.4.clone(), o(t.5))).collect::<Vec<_>>())?;
        Ok(s)
    }
}
