"""Generates in-game test mods (byte-patch JSON) for candidate table fields.

Writes into the given folder (keep it out of the repository: the mods hold
original game bytes). Usage: PYLUNE_LAYOUT_WORK=<work> testmods.py <out-dir>
The batch definitions below are the record of what each test confirms.
"""
import sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from tbl import records

ITEM = "gamedata/binarystaticinfo__/bin/iteminfo.staticinfobody"
DROP = "gamedata/binarystaticinfo__/bin/dropsetinfo.staticinfobody"
# DropInfoData entry (64 bytes): +1 item key, +26 rate (1 000 000 = 100 %),
# +42 amount A, +50 amount B (which is min/max is what test 2 decides).
ENTRY, RATE, AMOUNT_A, AMOUNT_B = 64, 26, 42, 50

def main(out):
    items = {n: p for _, n, p in records('iteminfo')[1]}
    drops = {n: p for _, n, p in records('dropsetinfo')[1]}
    ikey = {n: k.hex() for k, n, _ in records('iteminfo')[1]}

    def change(table, entry, rel, n, new, label):
        return {"entry": entry, "rel_offset": rel, "original": table[entry][rel:rel + n].hex(),
                "patched": new.to_bytes(n, 'little', signed=True).hex(), "label": label}

    def entry_off(drop, item):
        p = drops[drop]
        for i in range(int.from_bytes(p[14:18], 'little')):
            if p[18 + i * ENTRY + 1:18 + i * ENTRY + 5].hex() == ikey[item]:
                return 18 + i * ENTRY
        raise KeyError((drop, item))

    def amounts(drop, item, a, b):
        o = entry_off(drop, item)
        return [change(drops, drop, o + AMOUNT_A, 8, a, f"{drop} {item} amount A -> {a}"),
                change(drops, drop, o + AMOUNT_B, 8, b, f"{drop} {item} amount B -> {b}")]

    def rate(drop, item):
        o = entry_off(drop, item)
        return [change(drops, drop, o + RATE, 4, 1000000, f"{drop} {item} rate -> 100%")]

    batch = {
        "Pylune Test 1 - Arrow Stacks": (ITEM, [change(items, n, 1, 8, 999, f"{n} max stack -> 999") for n in ("Arrow", "Pyeonjeon_Arrow")]),
        "Pylune Test 2 - Hunting Amounts": (DROP, sum([amounts(d, m, 30, 30) + amounts(d, l, 20, 20) + amounts(d, "Animal_Bone", 6, 12)
            for d, m, l in (("DropSet_Middle_Animal_Common", "Meat_II", "Deer_Leather"), ("DropSet_Small_Animal_Common", "Meat_I", "Leather"))], [])),
        "Pylune Test 3 - Drop Rates": (DROP, rate("DropSet_Normal_Animal_Fish_Common", "Money_Copper") + rate("DropSet_Animal_Bear", "Honey")),
        # Every crate, barrel and pot gives exactly 100 copper (vanilla: 10-15 % for 1-2).
        "Pylune Test 4 - Coin Pouch": (DROP, sum([rate(d, "Money_Copper") + amounts(d, "Money_Copper", 100, 100)
            for d in ("DropSet_BoxBarrel_01", "DropSet_BoxBarrel_02", "DropSet_Pot_01")], [])),
    }
    for name, (game_file, changes) in batch.items():
        os.makedirs(f"{out}/{name}", exist_ok=True)
        mod = {"modinfo": {"title": name, "version": "1", "author": "Pylune"},
               "patches": [{"game_file": game_file, "changes": changes}]}
        json.dump(mod, open(f"{out}/{name}/{name.split(' - ')[1].lower().replace(' ', '_')}.json", "w"), indent=2)

if __name__ == "__main__":
    main(sys.argv[1])
