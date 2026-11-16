"""Realm of Ages, the game: a terminal interface for running a world a century or a millennium at a time.

    realm-tui                  (or: python3 -m realm_of_ages.tui)

Start a new world or load one; move it forward 100, 200, 300, 500 or 1000 years at a time, as often as you
like; read every log (the chronicle, every event by kind, the ledger of every crown's money, the wars, any
person's sheet and journal, the map, the books); reach in with god mode; end the world when you choose."""
import os

from rich.text import Text
from textual import work
from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.screen import ModalScreen, Screen
from textual.widgets import (Button, DataTable, Footer, Header, Input, Label, Markdown, OptionList, ProgressBar,
                             RichLog, Select, Static, TabbedContent, TabPane)
from textual.widgets.option_list import Option

from . import godmode
from .data import PEOPLES
from .events import CATEGORIES, classify
from .session import PRESETS, Session, list_worlds

SIZES = [("Small (240 x 135)", (240, 135)), ("Standard (480 x 270)", (480, 270)), ("Huge (640 x 360)", (640, 360))]
BOOKS = ["summary.md", "realms.md", "dynasties.md", "legends.md", "myths.md", "world.md", "magic.md", "seas.md",
         "pirates.md", "society.md", "dungeons.md", "bestiary.md", "calibration.md", "tracked.md"]
CAT_STYLE = {"god": "bold magenta", "war": "red", "politics": "yellow", "economy": "green", "people": "cyan",
             "lives": "white", "culture": "bright_blue", "heart": "bright_magenta", "magic": "magenta",
             "monsters": "bright_red", "heroes": "bright_yellow", "sea": "blue", "faith": "bright_cyan",
             "crime": "grey70", "nature": "bright_green", "misc": "grey50"}


def opts(pairs):
    return [(str(a), b) for a, b in pairs]


def num(value, default=0.0):
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


# ---------------------------------------------------------------- home
class Home(Screen):
    def compose(self) -> ComposeResult:
        yield Header()
        with Vertical(id="home"):
            yield Static("[b]REALM OF AGES[/b]\nA world that runs a century at a time, for as long as you let it.",
                         id="title")
            yield Button("Start new world", id="new", variant="success")
            yield Label("Saved worlds:")
            yield OptionList(id="worlds")
            yield Button("Load selected world", id="load", variant="primary")
            yield Button("Quit", id="quit", variant="error")
        yield Footer()

    def on_mount(self):
        self.refresh_worlds()

    def refresh_worlds(self):
        ol = self.query_one("#worlds", OptionList)
        ol.clear_options()
        self.worlds = list_worlds()
        for w in self.worlds:
            state = "ended" if w.get("ended") else f"year {w.get('year', 0)}"
            ol.add_option(Option(f"{w['name']}  ({state}, seed {w.get('seed')})", id=w["dir"]))

    def on_button_pressed(self, ev: Button.Pressed):
        if ev.button.id == "new":
            self.app.push_screen(NewWorld(), self.created)
        elif ev.button.id == "load":
            ol = self.query_one("#worlds", OptionList)
            if ol.highlighted is None:
                self.notify("Pick a world first.")
                return
            path = ol.get_option_at_index(ol.highlighted).id
            self.notify("Opening the world...")
            self.app.push_screen(WorldScreen(Session.load(path)))
        elif ev.button.id == "quit":
            self.app.exit()

    def created(self, s):
        if s is not None:
            self.app.push_screen(WorldScreen(s))


class NewWorld(ModalScreen):
    def compose(self) -> ComposeResult:
        with Vertical(id="dialog"):
            yield Label("[b]A new world[/b]")
            yield Input(placeholder="World name", id="name")
            yield Input(placeholder="Seed (a number)", id="seed", value="1")
            yield Select(opts(SIZES), value=(480, 270), id="size", allow_blank=False)
            with Horizontal():
                yield Button("Create", id="ok", variant="success")
                yield Button("Cancel", id="cancel")

    def on_button_pressed(self, ev: Button.Pressed):
        if ev.button.id == "cancel":
            self.dismiss(None)
            return
        name = self.query_one("#name", Input).value.strip() or "Untitled World"
        seed = int(num(self.query_one("#seed", Input).value, 1))
        w, h = self.query_one("#size", Select).value
        self.notify("The world wakes (this takes a few seconds)...")
        self.dismiss(Session.new(name, seed, w, h))


# ---------------------------------------------------------------- the world
class WorldScreen(Screen):
    """One world: the controls, the status, and every log."""
    BINDINGS = [(str(i + 1), f"fwd({n})", f"+{n}y") for i, n in enumerate(PRESETS)] + [("g", "god", "God mode")]

    def __init__(self, session):
        super().__init__()
        self.s = session
        self.running = False

    def compose(self) -> ComposeResult:
        yield Header()
        yield Static(id="status")
        with Horizontal(id="controls"):
            for n in PRESETS:
                yield Button(f"+{n}y", id=f"fwd{n}", variant="success")
            yield Button("Stop", id="stop", variant="warning")
            yield Button("God mode", id="god", variant="primary")
            yield Button("End world", id="end", variant="error")
            yield Button("Close", id="close")
        yield ProgressBar(id="progress", show_eta=True)
        with TabbedContent(id="tabs"):
            with TabPane("Realms", id="t-realms"):
                yield DataTable(id="realms", zebra_stripes=True, cursor_type="row")
            with TabPane("Chronicle", id="t-chron"):
                with Horizontal(classes="filters"):
                    yield Select(opts([("all kinds", "all")] + [(c, c) for c in CATEGORIES]), value="all",
                                 id="chron-cat", allow_blank=False)
                    yield Input(placeholder="search...", id="chron-q")
                    yield Select(opts([("major only", "major"), ("everything", "all")]), value="major",
                                 id="chron-level", allow_blank=False)
                yield RichLog(id="chron", wrap=True, markup=False, max_lines=4000, min_width=40)
            with TabPane("Transactions", id="t-money"):
                yield Label("Each crown's treasury over the last 100 years (in, out, by kind), then notable dealings:")
                yield DataTable(id="ledger", zebra_stripes=True)
                yield RichLog(id="tx", wrap=True, max_lines=3000)
            with TabPane("Economy", id="t-econ"), VerticalScroll():
                yield Label("Realms: stage, output, gold in circulation, crown debt and credit, work, poverty, trade")
                yield DataTable(id="econ", zebra_stripes=True)
                yield Label("Classes in each realm: people, income a head, poverty, and what they own (land, houses, "
                            "stock, coin)")
                yield DataTable(id="classes", zebra_stripes=True)
                yield Label("The richest people in the world, by everything they own")
                yield DataTable(id="rich", zebra_stripes=True)
            with TabPane("Wars", id="t-wars"):
                yield RichLog(id="wars", wrap=True, max_lines=3000)
            with TabPane("People", id="t-people"):
                with Horizontal():
                    with Vertical(id="people-left"):
                        yield Input(placeholder="name or id (prefix dead: to include the dead)", id="people-q")
                        yield OptionList(id="people-list")
                        with Horizontal():
                            yield Button("Track", id="track")
                            yield Button("Untrack", id="untrack")
                            yield Button("Edit (god)", id="edit", variant="primary")
                    with VerticalScroll(id="people-right"):
                        yield Markdown(id="sheet")
            with TabPane("Tracked", id="t-tracked"):
                yield RichLog(id="tracked", wrap=True, max_lines=5000)
            with TabPane("Map", id="t-map"):
                yield Static(id="map")
            with TabPane("Books", id="t-books"):
                with Horizontal():
                    yield OptionList(*[Option(b, id=b) for b in BOOKS], id="books")
                    with VerticalScroll():
                        yield Markdown(id="book")
        yield Footer()

    def on_mount(self):
        t = self.query_one("#realms", DataTable)
        t.add_columns("id", "Realm", "People", "Rank", "Rule", "Ruler", "Towns", "Population", "Gold", "Army",
                      "At war with")
        self.query_one("#ledger", DataTable).add_columns("Realm", "Income", "Spending", "Taxes", "Loot", "Tribute",
                                                          "Armies", "Court", "Interest", "Treasury now")
        self.refresh_all()

    # ---------------- refreshing every panel
    def refresh_all(self):
        s = self.s
        o = s.overview()
        ended = " [b red]ENDED[/b red]" if s.meta.get("ended") else ""
        self.query_one("#status", Static).update(
            f"[b]{s.meta['name']}[/b]{ended}   Year [b]{o['year']}[/b]   Population {o['pop']:,.0f} in "
            f"{o['towns']} towns   Realms {o['realms']}   Wars {o['wars']}   Notables {o['people']}   Heroes "
            f"{o['heroes']}   Monsters {o['monsters']} (calamities {o['calamities']})   Tracked {o['tracked']}")
        t = self.query_one("#realms", DataTable)
        t.clear()
        for row in s.realm_rows():
            t.add_row(*[f"{x:,}" if isinstance(x, int) and i > 5 else str(x) for i, x in enumerate(row)])
        self.refresh_chronicle()
        self.refresh_money()
        self.refresh_wars()
        self.refresh_econ()
        self.refresh_tracked()
        self.refresh_map()
        self.refresh_people()

    def refresh_chronicle(self):
        log = self.query_one("#chron", RichLog)
        log.clear()
        cat = self.query_one("#chron-cat", Select).value
        q = self.query_one("#chron-q", Input).value.lower().strip()
        level = self.query_one("#chron-level", Select).value
        rows = getattr(self.s.realm, "event_log", [])
        picked = []
        for y, c, major, text in reversed(rows):                     # the newest 1500 that match...
            if (cat != "all" and c != cat) or (level == "major" and not major) or (q and q not in text.lower()):
                continue
            picked.append((y, c, major, text))
            if len(picked) >= 1500:
                break
        if len(picked) >= 1500:
            log.write(Text("... (older events are in chronicle.md and details.md)", style="grey50"))
        for y, c, major, text in reversed(picked):                   # ...written oldest first, newest at the bottom
            line = Text(f"{y:>5}  ", style="grey50")
            line.append(f"[{c}] ", style=CAT_STYLE.get(c, ""))
            line.append(text.replace("**", ""), style="bold" if major else "")
            log.write(line, expand=True)

    def refresh_money(self):
        r = self.s.realm
        t = self.query_one("#ledger", DataTable)
        t.clear()
        led = getattr(r, "ledger", {})
        since = r.year - 100
        for k in sorted((k for k in r.kingdoms if not r.extinct[k.id]), key=lambda k: -len(r.alive_towns(k.id))):
            sums = {}
            for (y, kid), row in led.items():
                if kid == k.id and y > since:
                    for cat, v in row.items():
                        sums[cat] = sums.get(cat, 0.0) + v
            inc = sum(v for v in sums.values() if v > 0)
            out = -sum(v for v in sums.values() if v < 0)
            arm = sums.get("army upkeep", 0) + sums.get("regiments", 0) + sums.get("field armies", 0) + \
                sums.get("muster", 0)
            t.add_row(k.name[:40], f"{inc:,.0f}", f"{out:,.0f}", f"{sums.get('taxes', 0):,.0f}",
                      f"{sums.get('loot', 0):,.0f}", f"{sums.get('tribute in', 0) + sums.get('tribute out', 0):,.0f}",
                      f"{arm:,.0f}", f"{sums.get('court', 0):,.0f}", f"{sums.get('interest', 0):,.0f}",
                      f"{getattr(k, 'gold', 0):,.0f}")
        log = self.query_one("#tx", RichLog)
        log.clear()
        for y, kid, cat, amt, note, who in getattr(r, "tx", [])[-1500:]:
            name = r.kingdoms[kid].short if kid is not None and kid < len(r.kingdoms) else "-"
            line = Text(f"{y:>5}  ", style="grey50")
            line.append(f"{amt:>+10,.0f}  ", style="green" if amt > 0 else "red")
            line.append(f"[{cat}] {name}: {note}")
            log.write(line, expand=True)

    def refresh_econ(self):
        from .markets import CLASSES, econ_rows, net_worth
        r = self.s.realm
        t = self.query_one("#econ", DataTable)
        t.clear(columns=True)
        t.add_columns("Realm", "Stage", "Output", "Gold about", "Crown debt", "Rate", "Credit", "Jobless",
                      "Poverty", "Middle class", "Trade partners", "Top scroll", "Monster stores", "Bound dead")
        for row in econ_rows(r):
            t.add_row(*row)
        c = self.query_one("#classes", DataTable)
        c.clear(columns=True)
        c.add_columns("Realm", "Class", "People", "Share", "Income a head (1/1000 gold)", "Poor", "Land", "Houses", "Stock", "Coin",
                      "Owns a head (1/1000 gold)", "Share of wealth")
        for k in sorted((k for k in r.kingdoms if not r.extinct[k.id] and getattr(k, "econ", None)),
                        key=lambda k: -getattr(k, "real_gdp", 0))[:12]:
            e = k.econ
            pop = sum(v["people"] for v in e["classes"].values()) or 1
            for cl in CLASSES:
                v = e["classes"][cl]
                a = v["assets"]
                c.add_row(k.name[:34], cl, f"{v['people']:,.0f}", f"{v['people'] / pop:.0%}", f"{v['income'] * 1000:.2f}",
                          f"{v['poverty']:.0%}", f"{a['land']:,.0f}", f"{a['housing']:,.0f}", f"{a['stock']:,.0f}",
                          f"{a['coin']:,.0f}", f"{v['per_head'] * 1000:.1f}", f"{v['wealth_share']:.0%}")
        rich = self.query_one("#rich", DataTable)
        rich.clear(columns=True)
        rich.add_columns("Person", "Realm", "Net worth", "Gold in hand", "Work", "#")
        people = sorted((r.chars[i] for i in r.living), key=lambda ch: -net_worth(r, ch))[:40]
        for ch in people:
            rich.add_row(ch.full_name()[:40], r.kingdoms[ch.realm].short, f"{net_worth(r, ch):,.1f}",
                         f"{ch.gold:,.0f}", ch.profession or ("ruler" if ch.ruled and ch.ruled[2] is None else
                                                              "lord" if getattr(ch, "lordships", None) else "-"),
                         str(ch.id))

    def refresh_wars(self):
        r = self.s.realm
        log = self.query_one("#wars", RichLog)
        log.clear()
        log.write(Text("WARS NOW", style="bold red"))
        for w in r.wars.values():
            log.write(f"  {r.kingdoms[w['attacker']].name} against {r.kingdoms[w['defender']].name}: {w['cb']}, since "
                      f"{w['start']}, warscore {w['score']:+.0f}")
        log.write(Text("\nHOSTS IN THE FIELD", style="bold red"))
        for a in r.armies:
            log.write(f"  {a.name}: {int(a.men):,} men at {a.pos}" + (f", siege {a.siege:.0f}%" if a.siege else ""))
        log.write(Text("\nRECENT WARS", style="bold red"))
        for w in reversed(r.war_log[-60:]):
            log.write(f"  {w['year']}: {w['attacker']} vs {w['defender']} ({w['cb']}, {w['years']}y): "
                      f"{w.get('result', w['outcome'])}")
        log.write(Text("\nGREATEST BATTLES", style="bold red"))
        for y, where, wn, ln, wm, lm, d in sorted(r.battles_fought, key=lambda b: -b[6])[:30]:
            log.write(f"  {y}: the Battle of {where}: {wn} ({wm:,}) over {ln} ({lm:,}); {d:,} fell")

    def refresh_tracked(self):
        r = self.s.realm
        log = self.query_one("#tracked", RichLog)
        log.clear()
        ids = list(getattr(r, "tracked", ()))
        if not ids:
            log.write("No one is tracked. Track people from the People tab, or create one in God mode.")
        for cid in ids:
            c = r.chars[cid]
            log.write(Text(f"\n{c.full_name()}" + ("" if c.alive else f" (died {c.died}: {c.cause})"),
                           style="bold yellow"))
            for y, cat, text in getattr(c, "journal", [])[-300:]:
                line = Text(f"{y:>5}  ", style="grey50")
                line.append(f"[{cat}] ", style=CAT_STYLE.get(cat, ""))
                line.append(text.replace("**", ""))
                log.write(line, expand=True)

    def refresh_map(self):
        r = self.s.realm
        import math
        w = self.query_one("#map", Static).size.width or self.app.size.width or 140
        h = (self.app.size.height or 50) - 14
        cols, rows = max(40, w - 2), max(15, h)
        sx = max(1, math.ceil(r.W / cols))
        sy = max(1, math.ceil(r.H / rows), sx // 2)                     # terminal cells are about twice as tall as wide
        out = Text()
        from .data import COLORS, TERRAIN_COLORS
        pal = [k.color or COLORS[k.people] for k in r.kingdoms]
        caps = {(t.pos[0] // sy, t.pos[1] // sx) for t in r.alive_towns() if t.capital}
        hosts = {(a.pos[0] // sy, a.pos[1] // sx) for a in r.armies}
        for yy in range(0, r.H - sy + 1, sy):
            for xx in range(0, r.W - sx + 1, sx):
                cell = (yy // sy, xx // sx)
                f = r.owner_fac[yy + sy // 2, xx + sx // 2]
                t = r.terrain[yy + sy // 2, xx + sx // 2]
                if t == 0:
                    out.append("~", style="rgb(40,70,130) on rgb(20,35,70)")
                    continue
                col = pal[f] if f >= 0 else TERRAIN_COLORS[t]
                ch = "♛" if cell in caps else "⚔" if cell in hosts else "█" if f >= 0 else "░"
                out.append(ch, style=f"rgb({col[0]},{col[1]},{col[2]})")
            out.append("\n")
        out.append(f"\nFull map: {os.path.join(self.s.path, 'maps', 'latest.png')}   ♛ seat  ⚔ host  █ held  ░ wild",
                   style="grey70")
        self.query_one("#map", Static).update(out)

    def refresh_people(self):
        q = self.query_one("#people-q", Input).value
        ol = self.query_one("#people-list", OptionList)
        ol.clear_options()
        for c in self.s.find_characters(q):
            tag = " ★" if getattr(c, "tracked", False) else ""
            ruler = " ♛" if c.ruled and c.ruled[2] is None else ""
            ol.add_option(Option(f"{c.full_name()}{ruler}{tag}  ({PEOPLES[c.people]}, {c.age(self.s.realm.year)}, "
                                 f"lv {c.level}, #{c.id})", id=str(c.id)))

    def show_sheet(self, cid):
        from .characters import sheet
        r = self.s.realm
        c = r.chars[int(cid)]
        self.selected = c.id
        text = sheet(c, r)
        if getattr(c, "journal", None):
            text += "\n### Journal\n\n" + "\n".join(f"- **{y}** [{cat}] {t}" for y, cat, t in c.journal[-60:])
        self.query_one("#sheet", Markdown).update(text)

    # ---------------- events
    def on_tabbed_content_tab_activated(self, ev: TabbedContent.TabActivated):
        """Logs are drawn at the width of the screen they are shown on, so draw them when their tab opens."""
        tab = ev.pane.id if getattr(ev, "pane", None) is not None else ""
        {"t-chron": self.refresh_chronicle, "t-money": self.refresh_money, "t-wars": self.refresh_wars,
         "t-econ": self.refresh_econ,
         "t-tracked": self.refresh_tracked, "t-map": self.refresh_map}.get(tab, lambda: None)()

    def on_select_changed(self, ev: Select.Changed):
        if ev.select.id in ("chron-cat", "chron-level"):
            self.refresh_chronicle()

    def on_input_submitted(self, ev: Input.Submitted):
        if ev.input.id == "chron-q":
            self.refresh_chronicle()
        elif ev.input.id == "people-q":
            self.refresh_people()

    def on_option_list_option_selected(self, ev: OptionList.OptionSelected):
        if ev.option_list.id == "people-list":
            self.show_sheet(ev.option.id)
        elif ev.option_list.id == "books":
            p = os.path.join(self.s.path, ev.option.id)
            text = open(p).read() if os.path.exists(p) else "_Not written yet: move the world forward first._"
            if len(text) > 200_000:
                text = text[:200_000] + "\n\n_(truncated; open the file for the rest)_"
            self.query_one("#book", Markdown).update(text)

    def on_button_pressed(self, ev: Button.Pressed):
        bid = ev.button.id or ""
        if bid.startswith("fwd"):
            self.action_fwd(int(bid[3:]))
        elif bid == "stop":
            self.s.stop = True
            self.notify("Stopping at the end of this year...")
        elif bid == "god":
            self.action_god()
        elif bid == "end":
            self.app.push_screen(Confirm("End this world for good? Its books and logs stay on disk."), self.ended)
        elif bid == "close":
            if self.running:
                self.notify("Wait for the years to finish, or press Stop.")
                return
            self.s.save()
            self.app.pop_screen()
            self.app.screen.refresh_worlds() if hasattr(self.app.screen, "refresh_worlds") else None
        elif bid in ("track", "untrack", "edit"):
            cid = getattr(self, "selected", None)
            if cid is None:
                self.notify("Pick someone in the list first.")
                return
            c = self.s.realm.chars[cid]
            if bid == "edit":
                self.app.push_screen(EditCharacter(self.s, c), lambda _: (self.refresh_all(), self.show_sheet(c.id)))
                return
            self.notify(godmode.track(self.s.realm, c, bid == "track"))
            self.refresh_people()
            self.refresh_tracked()
            self.show_sheet(cid)

    def ended(self, yes):
        if yes:
            self.s.end()
            self.refresh_all()
            self.notify("The world has ended. Its books are written.")

    def action_god(self):
        if self.running:
            self.notify("Wait for the years to finish first.")
            return
        self.app.push_screen(GodMode(self.s), lambda _: self.refresh_all())

    def action_fwd(self, years):
        if self.running:
            self.notify("Already running.")
            return
        if self.s.meta.get("ended"):
            self.notify("This world has ended.")
            return
        self.running = True
        bar = self.query_one("#progress", ProgressBar)
        bar.update(total=years, progress=0)
        self.notify(f"Moving the world forward {years} years...")
        self.run_years(years)

    @work(thread=True, exclusive=True)
    def run_years(self, years):
        def progress(done, total, realm):
            if done % 5 == 0 or done == total:
                self.app.call_from_thread(self.tick, done, realm.year)
        try:
            self.s.advance(years, progress)
        except Exception as e:
            self.app.call_from_thread(self.notify, f"The world stumbled: {e}", severity="error")
        self.app.call_from_thread(self.finished)

    def tick(self, done, year):
        self.query_one("#progress", ProgressBar).update(progress=done)
        self.query_one("#status", Static).update(f"[b]{self.s.meta['name']}[/b]   running... year [b]{year}[/b]")

    def finished(self):
        self.running = False
        self.refresh_all()
        self.notify(f"Year {self.s.realm.year}. Books and map written.")


class Confirm(ModalScreen):
    def __init__(self, question):
        super().__init__()
        self.q = question

    def compose(self) -> ComposeResult:
        with Vertical(id="dialog"):
            yield Label(self.q)
            with Horizontal():
                yield Button("Yes", id="yes", variant="error")
                yield Button("No", id="no")

    def on_button_pressed(self, ev: Button.Pressed):
        self.dismiss(ev.button.id == "yes")


# ---------------------------------------------------------------- god mode
class GodMode(ModalScreen):
    def __init__(self, session):
        super().__init__()
        self.s = session

    def compose(self) -> ComposeResult:
        r = self.s.realm
        realms = [(k.name[:45], k.id) for k in godmode.living_realms(r)]
        lands = [(d["name"] + f" ({d['kind']})", d["id"]) for d in sorted(r.lands, key=lambda d: -d["size"])
                 if d["kind"] != "isle"]
        peoples = [(p, i) for i, p in enumerate(PEOPLES)]
        from .data import CLASSES
        with VerticalScroll(id="godbox"):
            yield Label("[b magenta]GOD MODE[/b magenta]  (every act is written into the chronicle)")
            yield Label("[b]The world[/b]")
            with Horizontal(classes="row"):
                yield Select(opts(realms), prompt="Realm A", id="ra")
                yield Select(opts(realms), prompt="Realm B", id="rb")
                yield Select(opts(lands), prompt="Continent", id="land")
            with Horizontal(classes="row"):
                yield Select(opts(peoples), prompt="People", id="cul")
                yield Select(opts([(g, g) for g in godmode.GOVERNMENTS]), prompt="Government", id="gov")
                yield Select(opts([(m, m) for m in godmode.MONSTERS]), prompt="Monster", id="mon")
                yield Input(placeholder="amount / years", id="amt")
            with Horizontal(classes="row"):
                for bid, label in (("calamity", "Wake calamity (continent)"), ("wcalamity", "World calamity"),
                                   ("monster", "Loose monster on A"), ("plague", "Plague on A"),
                                   ("famine", "Famine on A"), ("bless", "Bless harvest of A")):
                    yield Button(label, id=bid)
            with Horizontal(classes="row"):
                for bid, label in (("gold", "Gold to A"), ("war", "A declares war on B"), ("peace", "Peace A and B"),
                                   ("govset", "Set A's government"), ("era", "Raise people's era"),
                                   ("golden", "Golden age for people")):
                    yield Button(label, id=bid)
            with Horizontal(classes="row"):
                for bid, label in (("break", "Break A on continent"), ("curse", "Curse A's ruling house"),
                                   ("smite", "Smite A's seat")):
                    yield Button(label, id=bid, variant="error")
            yield Label("[b]Make a person[/b] (tracked by default; traits comma-separated)")
            with Horizontal(classes="row"):
                yield Input(placeholder="Name", id="cname")
                yield Select(opts(peoples), prompt="People", id="cpeople")
                yield Select(opts([("woman", True), ("man", False)]), prompt="Sex", id="csex")
                yield Input(placeholder="Age", id="cage", value="20")
                yield Select(opts(realms), prompt="Realm", id="crealm")
            with Horizontal(classes="row"):
                yield Select(opts([(c, c) for c in CLASSES]), prompt="Class", id="cclass")
                yield Input(placeholder="Level", id="clevel", value="1")
                yield Input(placeholder="Prowess", id="cprow")
                yield Input(placeholder="Talent 0-10", id="ctal")
                yield Input(placeholder="Gold", id="cgold", value="0")
            with Horizontal(classes="row"):
                yield Input(placeholder="dip,mar,ste,intr,lrn,lore e.g. 12,10,8,6,14,9", id="cattrs")
                yield Input(placeholder="personality e.g. brave,ambitious,just,gregarious", id="cpers")
            with Horizontal(classes="row"):
                yield Input(placeholder="genetic / bloodlines e.g. genius,starblood,dragon-merged", id="cgen")
                yield Input(placeholder="hidden (latent) bloodlines e.g. elderblood", id="clat")
                yield Select(opts([("hero", True), ("not a hero", False)]), value=True, id="chero", allow_blank=False)
            yield Label(f"Personality: {', '.join(godmode.ALL_PERSONALITY)}", classes="hint")
            yield Label(f"Genetic: {', '.join(godmode.ALL_GENETIC)}", classes="hint")
            with Horizontal(classes="row"):
                yield Button("Create person", id="create", variant="success")
                yield Button("Close", id="close")

    def val(self, wid):
        v = self.query_one(f"#{wid}", Select).value
        return None if v is Select.NULL or v is None else v

    def text(self, wid):
        return self.query_one(f"#{wid}", Input).value.strip()

    def on_button_pressed(self, ev: Button.Pressed):
        r = self.s.realm
        b = ev.button.id
        if b == "close":
            self.dismiss(None)
            return
        a, bb, land, cul = self.val("ra"), self.val("rb"), self.val("land"), self.val("cul")
        amt = num(self.text("amt"), 0)
        try:
            msg = None
            need = {"monster": a, "plague": a, "famine": a, "bless": a, "gold": a, "govset": a, "break": a,
                    "curse": a, "smite": a, "war": a, "peace": a}
            if b in need and need[b] is None:
                raise ValueError("pick Realm A first")
            if b == "calamity" or b == "wcalamity":
                if land is None:
                    raise ValueError("pick a continent")
                msg = godmode.wake_calamity(r, land, world=(b == "wcalamity"))
            elif b == "monster":
                msg = godmode.loose_monster(r, self.val("mon") or "dragon", a)
            elif b == "plague":
                msg = godmode.send_plague(r, a, great=True)
            elif b == "famine":
                msg = godmode.set_harvest(r, a, 0.35, int(amt) or 3)
            elif b == "bless":
                msg = godmode.set_harvest(r, a, 1.6, int(amt) or 10)
            elif b == "gold":
                msg = godmode.grant_gold(r, a, amt or 1000)
            elif b in ("war", "peace"):
                if bb is None or bb == a:
                    raise ValueError("pick two different realms")
                msg = godmode.declare_war(r, a, bb) if b == "war" else godmode.impose_peace(r, a, bb)
            elif b == "govset":
                msg = godmode.change_government(r, a, self.val("gov") or "feudal")
            elif b in ("era", "golden"):
                if cul is None:
                    raise ValueError("pick a people")
                msg = godmode.raise_era(r, cul) if b == "era" else godmode.golden_age(r, cul)
            elif b == "break":
                if land is None:
                    raise ValueError("pick a continent")
                msg = godmode.break_realm(r, a, land)
            elif b == "curse":
                k = r.kingdoms[a]
                msg = godmode.curse_house(r, k.house)
            elif b == "smite":
                cap = r.capital_of(a)
                msg = godmode.smite_town(r, cap) if cap else "no seat"
            elif b == "create":
                msg = self.create()
            if msg:
                self.notify(msg)
                self.s.flush()
                self.s.save()
        except Exception as e:
            self.notify(str(e), severity="error")

    def create(self):
        r = self.s.realm
        name = self.text("cname") or "Nameless"
        people = self.val("cpeople")
        realm_id = self.val("crealm")
        if people is None or realm_id is None:
            raise ValueError("pick a people and a realm")
        attrs = None
        if self.text("cattrs"):
            vals = [int(num(x, 5)) for x in self.text("cattrs").split(",")]
            attrs = dict(zip(["dip", "mar", "ste", "intr", "lrn", "lore"], vals))
        pers = [t.strip() for t in self.text("cpers").split(",") if t.strip()] or None
        gen = [t.strip() for t in self.text("cgen").split(",") if t.strip()]
        lat = [t.strip() for t in self.text("clat").split(",") if t.strip()]
        bad = [t for t in (pers or []) if t not in godmode.ALL_PERSONALITY] + \
              [t for t in gen if t not in godmode.ALL_GENETIC] + [t for t in lat if t not in godmode.ALL_GENETIC]
        if bad:
            raise ValueError(f"unknown traits: {', '.join(bad)}")
        c = godmode.create_character(
            r, name, people, bool(self.val("csex")), int(num(self.text("cage"), 20)), realm_id,
            cls=self.val("cclass"), level=int(num(self.text("clevel"), 1)), attrs=attrs,
            prowess=num(self.text("cprow"), None) if self.text("cprow") else None,
            talent=num(self.text("ctal"), None) if self.text("ctal") else None, personality=pers, genetic=gen,
            latent=lat, gold=num(self.text("cgold"), 0), hero=bool(self.val("chero")), tracked=True)
        return f"{c.full_name()} (#{c.id}) walks into the world, tracked."


class EditCharacter(ModalScreen):
    STATS = ["dip", "mar", "ste", "intr", "lrn", "lore", "prowess", "talent", "level", "renown", "gold", "stress",
             "happy", "health"]

    def __init__(self, session, c):
        super().__init__()
        self.s, self.c = session, c

    def compose(self) -> ComposeResult:
        from .data import CLASSES, PRESTIGE
        from .data2 import SKILLS
        stats = [(s, s) for s in self.STATS] + [(f"skill: {s}", s) for s in SKILLS]
        traits = [(t, t) for t in godmode.ALL_PERSONALITY + godmode.ALL_GENETIC]
        with VerticalScroll(id="godbox"):
            yield Label(f"[b magenta]GOD MODE: {self.c.full_name()}[/b magenta] (#{self.c.id})")
            with Horizontal(classes="row"):
                yield Select(opts(stats), prompt="Stat", id="stat")
                yield Input(placeholder="value", id="sval")
                yield Button("Set stat", id="setstat", variant="success")
            with Horizontal(classes="row"):
                yield Select(opts(traits), prompt="Trait", id="trait")
                yield Button("Give trait", id="addtrait", variant="success")
                yield Button("Take trait", id="deltrait", variant="warning")
            with Horizontal(classes="row"):
                yield Select(opts([(c, c) for c in list(CLASSES) + list(PRESTIGE)]), prompt="Class", id="cls")
                yield Button("Add class", id="addclass")
                yield Input(placeholder="relic name", id="relic")
                yield Select(opts([(n, i) for i, n in enumerate(["Common", "Fine", "Rare", "Epic", "Legendary",
                                                                   "Relic", "Worldforged"])]), value=5, id="rar",
                             allow_blank=False)
                yield Button("Give relic", id="give")
            with Horizontal(classes="row"):
                yield Input(placeholder="another person's id", id="other")
                yield Button("Marry them", id="marry")
                yield Button("Make them fall in love", id="love")
            with Horizontal(classes="row"):
                for bid, label in (("track", "Track"), ("untrack", "Untrack"), ("hero", "Make hero"),
                                   ("crown", "Crown in their realm"), ("immortal", "Make ageless"),
                                   ("concord", "Name the Concord")):
                    yield Button(label, id=bid)
            with Horizontal(classes="row"):
                yield Button("Make custom", id="custom", variant="primary")
                yield Button("Strike down", id="kill", variant="error")
                yield Button("Raise from the dead", id="raise", variant="warning")
                yield Button("Close", id="close")

    def on_button_pressed(self, ev: Button.Pressed):
        r, c, b = self.s.realm, self.c, ev.button.id
        if b == "close":
            self.dismiss(None)
            return
        try:
            msg = None
            if b == "setstat":
                st = self.query_one("#stat", Select).value
                if st is Select.NULL:
                    raise ValueError("pick a stat")
                godmode.set_stat(r, c, st, self.query_one("#sval", Input).value)
                msg = f"{st} set."
            elif b in ("addtrait", "deltrait"):
                t = self.query_one("#trait", Select).value
                if t is Select.NULL:
                    raise ValueError("pick a trait")
                msg = godmode.set_trait(r, c, t, b == "addtrait")
            elif b == "addclass":
                cl = self.query_one("#cls", Select).value
                if cl is Select.NULL:
                    raise ValueError("pick a class")
                msg = godmode.set_class(r, c, cl)
            elif b == "give":
                msg = godmode.give_item(r, c, self.query_one("#relic", Input).value or "a nameless relic",
                                        self.query_one("#rar", Select).value)
            elif b in ("marry", "love"):
                oid = int(num(self.query_one("#other", Input).value, -1))
                o = r.chars.get(oid)
                if o is None:
                    raise ValueError("no one has that id")
                msg = godmode.marry_characters(r, c, o) if b == "marry" else godmode.make_love(r, c, o)
            elif b in ("track", "untrack"):
                msg = godmode.track(r, c, b == "track")
            elif b == "hero":
                msg = godmode.make_hero(r, c)
            elif b == "crown":
                msg = godmode.crown_character(r, c)
            elif b == "immortal":
                msg = godmode.make_immortal(r, c, True)
            elif b == "concord":
                msg = godmode.name_concord(r, c)
            elif b == "custom":
                msg = godmode.make_custom(r, c, not getattr(c, "custom", False))
            elif b == "kill":
                msg = godmode.strike_down(r, c)
            elif b == "raise":
                msg = godmode.raise_dead(r, c)
            if msg:
                self.notify(msg)
                self.s.flush()
                self.s.save()
        except Exception as e:
            self.notify(str(e), severity="error")


# ---------------------------------------------------------------- the app
class RealmApp(App):
    TITLE = "Realm of Ages"
    CSS = """
    #home { align: center middle; width: 80; height: auto; padding: 1 2; }
    #title { text-align: center; padding: 1; }
    #worlds { height: 12; }
    #dialog { width: 70; height: auto; padding: 1 2; border: thick $accent; background: $surface; }
    ModalScreen { align: center middle; }
    #godbox { width: 95%; height: 90%; padding: 1 2; border: thick magenta; background: $surface; }
    .row { height: auto; margin-bottom: 1; }
    .row Select { width: 1fr; }
    .row Input { width: 1fr; }
    .hint { color: $text-muted; }
    #status { height: 1; padding: 0 1; background: $boost; }
    #controls { height: auto; }
    #controls Button { min-width: 10; margin-right: 1; }
    .filters { height: auto; }
    .filters Select { width: 30; }
    .filters Input { width: 50; }
    #people-left { width: 60; }
    #people-list { height: 1fr; }
    #books { width: 24; }
    #map { padding: 0 1; }
    #econ { height: 16; }
    #classes { height: 24; }
    #rich { height: 24; }
    """

    def on_mount(self):
        self.push_screen(Home())


def main():
    RealmApp().run()


if __name__ == "__main__":
    main()
