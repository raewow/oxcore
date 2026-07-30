#!/usr/bin/env python3
"""Generate the vanilla -> modern update-field index map.

Both protocols model object state the same way: an array of u32 values plus a bitmask saying
which entries are present. Only the *indices* moved between 1.12.1 and 1.14.x, so a modern
`SMSG_UPDATE_OBJECT` body can be produced from the vanilla field writes the game systems already
emit, provided we know where each field landed.

Sources are HermesProxy's two enum files, joined by field name:

    reference/HermesProxy/HermesProxy/World/Enums/V1_12_1_5875/UpdateFields.cs
    reference/HermesProxy/HermesProxy/World/Enums/V1_14_1_40688/UpdateFields.cs

40688 is the right modern table for build 42597 -- see `ModernVersion.GetUpdateFieldsDefiningBuild`
in reference/HermesProxy/HermesProxy/VersionChecker.cs, where every 1.14.2 build including 42597
returns V1_14_1_40688.

Output is committed, not generated at build time: reference/ is a reference checkout, not a build
input. Run this by hand when the reference is updated.

    python3 tools/updatefield-gen/updatefield_gen.py            # report only
    python3 tools/updatefield-gen/updatefield_gen.py --write    # rewrite the Rust table
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
ENUMS = REPO / "reference/HermesProxy/HermesProxy/World/Enums"
VANILLA_SRC = ENUMS / "V1_12_1_5875/UpdateFields.cs"
MODERN_SRC = ENUMS / "V1_14_1_40688/UpdateFields.cs"
OUT = REPO / "crates/shared/src/protocol/updates/modern/field_map.rs"

# Object-type families and the enums whose fields they inherit, outermost last. Vanilla has no
# separate ActivePlayer enum -- 1.12 keeps every player field in PlayerField -- so the modern
# ActivePlayer family reads from the same vanilla source as Player.
FAMILIES = [
    # (rust name, vanilla enum chain, modern enum chain)
    ("Item", ["ObjectField", "ItemField"], ["ObjectField", "ItemField"]),
    ("Container", ["ObjectField", "ItemField", "ContainerField"],
     ["ObjectField", "ItemField", "ContainerField"]),
    ("Unit", ["ObjectField", "UnitField"], ["ObjectField", "UnitField"]),
    ("Player", ["ObjectField", "UnitField", "PlayerField"],
     ["ObjectField", "UnitField", "PlayerField"]),
    ("ActivePlayer", ["ObjectField", "UnitField", "PlayerField"],
     ["ObjectField", "UnitField", "PlayerField", "ActivePlayerField"]),
    ("GameObject", ["ObjectField", "GameObjectField"], ["ObjectField", "GameObjectField"]),
    ("DynamicObject", ["ObjectField", "DynamicObjectField"],
     ["ObjectField", "DynamicObjectField"]),
    ("Corpse", ["ObjectField", "CorpseField"], ["ObjectField", "CorpseField"]),
]

# Fields whose names differ across the two tables, as {vanilla: (modern, element index)}. Vanilla
# numbers its array members individually (UNIT_FIELD_POWER1..5) where 1.14 uses a real array, and
# some fields moved between the Unit and Player enums.
#
# Only add an entry when the two really are the same field. A wrong alias silently writes a value
# into an unrelated slot, which the client renders as garbage rather than rejecting -- there is no
# error to trace back from.
ALIASES = {
    **{f"UNIT_FIELD_POWER{i + 1}": ("UNIT_FIELD_POWER", i) for i in range(5)},
    **{f"UNIT_FIELD_MAXPOWER{i + 1}": ("UNIT_FIELD_MAXPOWER", i) for i in range(5)},
    **{f"UNIT_FIELD_STAT{i}": ("UNIT_FIELD_STAT", i) for i in range(5)},
    **{f"PLAYER_FIELD_POSSTAT{i}": ("UNIT_FIELD_POSSTAT", i) for i in range(5)},
    **{f"PLAYER_FIELD_NEGSTAT{i}": ("UNIT_FIELD_NEGSTAT", i) for i in range(5)},
    "PLAYER_FIELD_RESISTANCEBUFFMODSPOSITIVE": ("UNIT_FIELD_RESISTANCEBUFFMODSPOSITIVE", 0),
    "PLAYER_FIELD_RESISTANCEBUFFMODSNEGATIVE": ("UNIT_FIELD_RESISTANCEBUFFMODSNEGATIVE", 0),
    # 1.14 moved the dynamic flags up to the shared Object block.
    "UNIT_DYNAMIC_FLAGS": ("OBJECT_DYNAMIC_FLAGS", 0),
    # UNIT_FIELD_CHANNEL_DATA is {SpellID, SpellVisual}; only the spell id has a vanilla source.
    "UNIT_CHANNEL_SPELL": ("UNIT_FIELD_CHANNEL_DATA", 0),
}

# Fields that must never be resolved by name, with the reason. These are the dangerous ones: the
# name survived into 1.14 but the meaning did not, so a mechanical join produces a plausible-looking
# packet carrying wrong values.
#
# Anything here needs a deliberate value transform (see `modern/repack.rs`) rather than a slot move.
DENIED = {
    "PLAYER_BYTES":
        "vanilla packs skin/face/hair here; 1.14 packs "
        "(PartyType, NumBankSlots, NativeSex, Inebriation) and moved appearance to "
        "PLAYER_FIELD_CUSTOMIZATION_CHOICES",
    "PLAYER_BYTES_2":
        "vanilla packs facial hair and rest state; 1.14 packs (PvpTitle, ArenaFaction, PvpRank)",
    "PLAYER_BYTES_3":
        "vanilla packs gender and drunkenness; no 1.14 field of the same shape",
    "UNIT_FIELD_BYTES_0":
        "byte 2 diverges: vanilla (race, class, gender, powertype) vs 1.14 "
        "(Race, ClassId, PlayerClassId, Sex); power type moved to UNIT_FIELD_DISPLAY_POWER",
    "UNIT_FIELD_BYTES_1":
        "byte 2 diverges: vanilla shapeshift form vs 1.14 VisFlags",
    "UNIT_FIELD_ATTACK_POWER_MODS":
        "vanilla TWO_SHORT (pos, neg) in one slot; 1.14 splits into two i32 fields",
    "UNIT_FIELD_RANGED_ATTACK_POWER_MODS":
        "vanilla TWO_SHORT (pos, neg) in one slot; 1.14 splits into two i32 fields",
    "UNIT_VIRTUAL_ITEM_SLOT_DISPLAY":
        "vanilla holds display ids; 1.14 UNIT_VIRTUAL_ITEM_SLOT_ID holds item entry ids",
    "UNIT_VIRTUAL_ITEM_INFO":
        "vanilla packs class/subclass/material/sheath bytes; no 1.14 equivalent",
}

# Modern fields emitted as named constants, for the hand-written transforms in `modern/repack.rs`
# that cannot go through the mechanical slot map.
EXPORTED_MODERN = [
    "UNIT_FIELD_BYTES_0",
    "UNIT_FIELD_BYTES_1",
    "UNIT_FIELD_BYTES_2",
    "UNIT_FIELD_DISPLAY_POWER",
    "UNIT_FIELD_ATTACK_POWER_MOD_POS",
    "UNIT_FIELD_ATTACK_POWER_MOD_NEG",
    "UNIT_FIELD_RANGED_ATTACK_POWER_MOD_POS",
    "UNIT_FIELD_RANGED_ATTACK_POWER_MOD_NEG",
    # Fields 1.14 requires to be non-zero on a freshly created object. See `modern/placeholders.rs`
    # -- vanilla has no source for any of them, and leaving them zero gives the client a unit with
    # zero scale and zero haste divisors.
    "UNIT_FIELD_MOD_POWER_REGEN",
    "UNIT_FIELD_FLAGS_2",
    "UNIT_FIELD_DISPLAY_SCALE",
    "UNIT_FIELD_NATIVE_X_DISPLAY_SCALE",
    "UNIT_MOD_CAST_HASTE",
    "UNIT_FIELD_MOD_HASTE",
    "UNIT_FIELD_MOD_RANGED_HASTE",
    "UNIT_FIELD_MOD_HASTE_REGEN",
    "UNIT_FIELD_MOD_TIME_RATE",
    "UNIT_FIELD_HOVERHEIGHT",
    "UNIT_FIELD_SCALE_DURATION",
    "UNIT_FIELD_LOOK_AT_CONTROLLER_ID",
    "PLAYER_WOW_ACCOUNT",
    "PLAYER_FIELD_VIRTUAL_PLAYER_REALM",
    "PLAYER_FIELD_HONOR_LEVEL",
    "PLAYER_FIELD_AVG_ITEM_LEVEL",
    "ACTIVE_PLAYER_FIELD_REST_INFO",
    "ACTIVE_PLAYER_FIELD_MOD_DAMAGE_DONE_PCT",
    "ACTIVE_PLAYER_FIELD_MOD_HEALING_PCT",
    "ACTIVE_PLAYER_FIELD_MOD_HEALING_DONE_PCT",
    "ACTIVE_PLAYER_FIELD_MOD_PERIODIC_HEALING_DONE_PERCENT",
    "ACTIVE_PLAYER_FIELD_WEAPON_DMG_MULTIPLIERS",
    "ACTIVE_PLAYER_FIELD_WEAPON_ATK_SPEED_MULTIPLIERS",
    "ACTIVE_PLAYER_FIELD_MOD_SPELL_POWER_PCT",
    "ACTIVE_PLAYER_FIELD_MAX_LEVEL",
    "ACTIVE_PLAYER_FIELD_MOD_PET_HASTE",
    "ACTIVE_PLAYER_FIELD_HONOR_NEXT_LEVEL",
    "ACTIVE_PLAYER_FIELD_PVP_TIER_MAX_FROM_WINS",
    "ACTIVE_PLAYER_FIELD_PVP_LAST_WEEKS_TIER_MAX_FROM_WINS",
    # Byte-packed: MultiActionBars sits at byte 1 of BYTES, NumBackpackSlots at byte 2 of BYTES_6.
    "ACTIVE_PLAYER_FIELD_BYTES",
    "ACTIVE_PLAYER_FIELD_BYTES_6",
]

ENUM_RE = re.compile(r"public enum (\w+)")
# `NAME = 0x1,` or `NAME = OtherEnum.OTHER_END + 0x1,`
MEMBER_RE = re.compile(
    r"^\s*(?P<name>[A-Z][A-Z0-9_]*)\s*=\s*"
    r"(?:(?P<base_enum>\w+)\.(?P<base_member>\w+)\s*\+\s*)?"
    r"(?P<offset>0x[0-9A-Fa-f]+|\d+)\s*,?"
)
SIZE_RE = re.compile(r"Size:\s*(\d+)")
TYPE_RE = re.compile(r"Type:\s*(\w+)")


@dataclass
class Field:
    name: str
    enum: str
    index: int
    size: int
    is_guid: bool


def parse(path: Path) -> dict[str, list[Field]]:
    """Parse one UpdateFields.cs into {enum name: [Field]}, resolving `Other.OTHER_END + n`."""
    enums: dict[str, list[Field]] = {}
    values: dict[tuple[str, str], int] = {}
    current: str | None = None

    for line in path.read_text(encoding="utf-8-sig").splitlines():
        found = ENUM_RE.search(line)
        if found:
            current = found.group(1)
            enums[current] = []
            continue
        if current is None:
            continue
        member = MEMBER_RE.match(line)
        if not member:
            continue

        index = int(member.group("offset"), 0)
        if member.group("base_enum"):
            key = (member.group("base_enum"), member.group("base_member"))
            if key not in values:
                raise SystemExit(f"{path.name}: unresolved base {key[0]}.{key[1]}")
            index += values[key]

        name = member.group("name")
        values[(current, name)] = index
        # `*_END` members are bounds, not fields.
        if name.endswith("_END"):
            continue

        comment = line.split("//", 1)[1] if "//" in line else ""
        size = SIZE_RE.search(comment)
        kind = TYPE_RE.search(comment)
        enums[current].append(Field(
            name=name,
            enum=current,
            index=index,
            size=int(size.group(1)) if size else 1,
            is_guid=bool(kind and kind.group(1) == "GUID"),
        ))

    # Keep the *_END bounds reachable for the caller.
    enums["__ends__"] = [
        Field(name=f"{enum}.{name}", enum=enum, index=value, size=0, is_guid=False)
        for (enum, name), value in values.items() if name.endswith("_END")
    ]
    return enums


def chain(enums: dict[str, list[Field]], names: list[str]) -> list[Field]:
    out: list[Field] = []
    for name in names:
        out.extend(enums.get(name, []))
    return out


def end_of(enums: dict[str, list[Field]], enum: str) -> int:
    for field in enums["__ends__"]:
        if field.name.startswith(f"{enum}."):
            return field.index
    raise SystemExit(f"no *_END found for {enum}")


def main() -> int:
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument("--write", action="store_true", help="rewrite the Rust table in place")
    opts = args.parse_args()

    vanilla = parse(VANILLA_SRC)
    modern = parse(MODERN_SRC)

    lines: list[str] = []
    report: list[str] = []
    total_mapped = total_fields = 0

    every_modern = {f.name: f for group in modern.values() for f in group}
    lines.append("// Individual modern slots referenced by hand-written transforms.")
    for name in EXPORTED_MODERN:
        field = every_modern.get(name)
        if field is None:
            raise SystemExit(f"EXPORTED_MODERN names a field that does not exist: {name}")
        lines.append(f"/// Modern `{name}` (size {field.size}).")
        lines.append(f"pub const MODERN_{name}: u16 = {field.index};")
    lines.append("")

    for rust_name, v_chain, m_chain in FAMILIES:
        v_fields = chain(vanilla, v_chain)
        m_by_name = {f.name: f for f in chain(modern, m_chain)}
        v_end = end_of(vanilla, v_chain[-1])

        # Direct-index table: vanilla index -> modern index. One entry per u32 slot, so multi-slot
        # fields (arrays, GUIDs) expand element-wise and callers need no size knowledge.
        slots: list[tuple[int, str] | None] = [None] * v_end
        mapped = unmapped = 0

        for field in v_fields:
            if field.name in DENIED:
                unmapped += 1
                report.append(f"  {rust_name:<14} {field.name}  DENIED: {DENIED[field.name]}")
                continue

            alias_name, element = ALIASES.get(field.name, (field.name, 0))
            target = m_by_name.get(alias_name)
            if target is None and rust_name == "ActivePlayer":
                # 1.14 split the vanilla player block in two and prefixed the self-only half.
                # PLAYER_XP -> ACTIVE_PLAYER_FIELD_XP, PLAYER_FIELD_COINAGE -> ..._COINAGE.
                suffix = re.sub(r"^PLAYER_(FIELD_)?", "", field.name)
                target = m_by_name.get(f"ACTIVE_PLAYER_FIELD_{suffix}")
            if target is None:
                unmapped += 1
                report.append(f"  {rust_name:<14} {field.name}  (no modern counterpart)")
                continue
            base = target.index + element
            remaining = target.size - element
            mapped += 1

            if field.is_guid:
                # A 64-bit vanilla GUID becomes a 128-bit modern one, so an *array* of them
                # changes stride: 2 slots per entry becomes 4. Walking slot-for-slot would put
                # entry 2 onward at the wrong offset, so step entry by entry.
                #
                # Both vanilla halves point at the same modern base. The encoder needs both before
                # it can widen the value -- the modern high half is derived from the object type
                # and realm, and writing only the low half leaves the client holding a GUID whose
                # type reads as Null.
                entries = min(field.size // 2, remaining // 4)
                for entry in range(entries):
                    slots[field.index + entry * 2] = (base + entry * 4, "GuidLow")
                    slots[field.index + entry * 2 + 1] = (base + entry * 4, "GuidHigh")
                continue

            # Arrays can differ in length between versions (item enchantments went 21 -> 39).
            # Map the elements both sides have and drop the rest rather than running off the end
            # of the modern field into whatever follows it.
            for k in range(min(field.size, remaining)):
                slots[field.index + k] = (base + k, "Plain")

        total_mapped += mapped
        total_fields += mapped + unmapped

        const = to_screaming(rust_name)
        m_end = end_of(modern, m_chain[-1])
        m_dyn_end = end_of(modern, m_chain[-1].replace("Field", "DynamicField"))

        lines.append(f"/// Total modern field slots for {rust_name}, including inherited ones.")
        lines.append(f"///")
        lines.append(f"/// The mask is always sent at full width for the object type, so this drives")
        lines.append(f"/// its block count even when only one field changed.")
        lines.append(f"pub const {const}_FIELD_COUNT: u16 = {m_end};")
        lines.append("")
        lines.append(f"/// Total modern *dynamic* field slots for {rust_name}.")
        lines.append(f"pub const {const}_DYNAMIC_FIELD_COUNT: u16 = {m_dyn_end};")
        lines.append("")
        lines.append(f"/// Vanilla slot -> modern slot for {rust_name} objects.")
        lines.append(f"///")
        lines.append(f"/// Index with a vanilla field number; [`UNMAPPED`] means the field does not")
        lines.append(f"/// exist in 1.14 and must be dropped rather than guessed at.")
        lines.append(f"pub const {const}_MAP: [ModernSlot; {v_end}] = [")
        for index, slot in enumerate(slots):
            if slot is None:
                lines.append(f"    UNMAPPED,")
            else:
                target, kind = slot
                lines.append(f"    ModernSlot::{kind}({target}),")
        lines.append("];")
        lines.append("")

        report.append(f"{rust_name:<14} {mapped:>3} mapped, {unmapped:>3} unmapped "
                      f"(vanilla slots {v_end}, modern end {end_of(modern, m_chain[-1])})")

    body = HEADER + "\n".join(lines)
    print("\n".join(report))
    print(f"\n{total_mapped}/{total_fields} field names resolved")

    if opts.write:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(body)
        print(f"wrote {OUT.relative_to(REPO)}")
    else:
        print(f"\n(dry run -- pass --write to update {OUT.relative_to(REPO)})")
    return 0


def to_screaming(name: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).upper()


HEADER = '''//! Vanilla -> modern update-field index maps.
//!
//! GENERATED by `tools/updatefield-gen/updatefield_gen.py` -- do not edit by hand.
//!
//! Both protocols store object state as an array of u32 slots plus a presence bitmask; only the
//! slot numbers moved between 1.12.1 and 1.14.x. That means a modern `SMSG_UPDATE_OBJECT` body can
//! be built from the vanilla field writes the game systems already emit, by translating each slot
//! number through the table for the object's type.
//!
//! Source: HermesProxy `V1_12_1_5875/UpdateFields.cs` and `V1_14_1_40688/UpdateFields.cs`, joined
//! by field name. 40688 is the correct modern table for build 42597.

/// Where one vanilla field slot lands in the modern layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModernSlot {
    /// No modern counterpart; the value must be dropped.
    None,
    /// A plain u32 slot that copies across unchanged.
    Plain(u16),
    /// Low 32 bits of a vanilla 64-bit GUID, and the modern slot its widened form starts at.
    ///
    /// Vanilla stores a GUID in two slots, modern in four. The encoder buffers both vanilla halves,
    /// rebuilds the 64-bit value, then widens it to 128 bits across `base..base + 4`. Writing only
    /// what vanilla sent would leave the upper two slots zero, which the client reads as a GUID of
    /// type `Null` -- an object whose own identity contradicts the one in the block header.
    GuidLow(u16),
    /// High 32 bits of the same vanilla GUID, pointing at the same modern base slot.
    GuidHigh(u16),
}

/// Shorthand for a vanilla field with no modern equivalent.
pub const UNMAPPED: ModernSlot = ModernSlot::None;

'''


if __name__ == "__main__":
    sys.exit(main())
