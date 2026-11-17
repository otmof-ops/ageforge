//! The chronicle's kinds of event, for filtering: each entry is sorted by the first rule its text matches.

use regex::Regex;
use std::sync::OnceLock;

pub const CATEGORIES: [&str; 16] = ["god", "war", "politics", "economy", "people", "lives", "culture", "heart", "magic", "monsters", "heroes",
                                     "sea", "faith", "crime", "nature", "misc"];

const RULES: [(&str, &str); 16] = [
    ("god", r"\*\*god\b|by the gods' hand|divine"),
    ("politics", r"is crowned|a new dynasty|new seat|succession war|swears fealty|is born\b.*realm|regent"),
    ("crime", r"heist|quiet hand|assassin|bounty|hanged|thieves|shinobi|needle-mark|black feather"),
    ("sea", r"pirate|colony|sights |charts |naval|fleet|privateer|mutiny|the battle of (the )?(sea|gulf|straits|deeps|shallows)|kraken|serpent|leviathan|haven"),
    ("people", r"rising|commune|pestilence|burns out|plague|sickness|fever|cough|flux|pox|wasting|migrat"),
    ("war", r"\*\*war\.\*\*|battle of|siege|host of|white peace|wins its|throws back|warscore|musters|declares war|peace\.\*\*|vengeance for"),
    ("economy", r"famine|harvest|bankrupt|buys |forges|great work|market|treasury|gold"),
    ("heart", r"love|lovers|marry|married|scandal|swears vengeance|vengeance on|grief|goes mad|cracks under|friends|rivals|affair|a child is born"),
    ("magic", r"tier|rite|spell|blood wakes|curse|legendary forging|returns\*\*|seals|casts down|concord|lich|the fall\.|bending|bender|sworn against"),
    ("nature", r"extinction|new breed|alpha of|species"),
    ("monsters", r"dragon|troll|wraith|demon|calamity|clutch|wyrm|beast|sinks back"),
    ("heroes", r"slays|\bdelves?\b|the delver|bottom of|floors? |\bhero|artifact|relic|dungeon|masters |level"),
    ("faith", r"faith|proclaims|prophet|religion|holy war|temple|heresy|holy book|scripture"),
    ("culture", r" learn |tradition|lifestyle|golden age|dark age|new era|saga|thinkers|old ways|masterwork|monument|finishes .* \(|pull down|brings .* out of"),
    ("lives", r"died of|died in|is born|midwives|comes of age|aged \d+"),
    ("politics", r"crown|dynasty|faction|ultimatum|new realm|duke|duchess|council|regent|succession|throne|is no more| found |guildmaster|cadet branch|hook on|nobility|royal court|empire|unites|tributary|new order|reform|charter|tyrant|purge|rises|chiefdom|realm|kingdom|alliance|abdicat|deposed|usurp"),
];

fn compiled() -> &'static Vec<(&'static str, Regex)> {
    static C: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    C.get_or_init(|| RULES.iter().map(|(c, p)| (*c, Regex::new(&format!("(?i){p}")).expect("rule"))).collect())
}

pub fn classify(text: &str) -> &'static str {
    compiled().iter().find(|(_, rx)| rx.is_match(text)).map(|(c, _)| *c).unwrap_or("misc")
}
