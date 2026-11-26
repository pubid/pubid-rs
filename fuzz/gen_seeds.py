#!/usr/bin/env python3
"""Generate the structure-aware fuzz seed corpora from the vendored slices.

The seeds ARE the corpus rows: every canonical human spelling, alias and
negative input of the vendored testsuite slices becomes a seed file, so
libFuzzer starts from attested input space instead of garbage. Re-run
after any fixture refresh (CI does this before every smoke run):

    python3 fuzz/gen_seeds.py                  full corpus (local rounds)
    python3 fuzz/gen_seeds.py --limit 40       head subset per flavor (CI)

--limit N keeps the first N rows per flavor: libFuzzer replays every
seed once before the timed loop starts, and the full 41k-file corpus
takes ~20 minutes of that replay on a 2-core runner - the CI smoke
caps it so the 20-second budget actually fuzzes.

Outputs (gitignored, deterministic given the fixtures):
    fuzz/corpus/parse_flavor/<id>.seed   humans + aliases + negatives
    fuzz/corpus/roundtrip/<id>.seed      canonical humans only
"""

import argparse
import pathlib
import sys

try:
    import yaml
except ImportError:
    sys.exit("PyYAML is required: pip install pyyaml")

ROOT = pathlib.Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "crates" / "pubid-core" / "fixtures" / "testsuite"


def rows_of(path):
    try:
        docs = yaml.safe_load(path.read_text())
    except yaml.YAMLError as err:
        sys.exit(f"{path}: invalid YAML: {err}")
    if not isinstance(docs, list):
        return []
    return [row for row in docs if isinstance(row, dict)]


def write_seed(target, row_id, text):
    slug = row_id.replace("/", "-")
    out = ROOT / "fuzz" / "corpus" / target / f"{slug}.seed"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--limit",
        type=int,
        default=0,
        help="keep the first N rows per flavor (0 = the full corpus)",
    )
    options = parser.parse_args()

    if not FIXTURES.is_dir():
        sys.exit(f"no vendored slices at {FIXTURES}")
    # The corpus dirs are generated output: a fresh run replaces them
    # wholesale (a --limit run must not inherit the previous full run's
    # 41k files, or libFuzzer replays them anyway).
    for target in ("parse_flavor", "roundtrip"):
        corpus_dir = ROOT / "fuzz" / "corpus" / target
        if corpus_dir.is_dir():
            for stale in corpus_dir.glob("*.seed"):
                stale.unlink()
    counts = {}
    for flavor_dir in sorted(FIXTURES.iterdir()):
        if not flavor_dir.is_dir():
            continue
        flavor = flavor_dir.name
        parsed = 0
        for path in sorted(flavor_dir.glob("*.yaml")):
            for row in rows_of(path):
                if options.limit and parsed >= options.limit:
                    break
                row_id = f"{flavor}.{row.get('id', 'x')}"
                representations = row.get("representations") or {}
                human = representations.get("human") if isinstance(representations, dict) else None
                if row.get("expect", {}).get("error") is not None:
                    # A negative row in the positive files, or the
                    # negative file itself: a reject-input seed.
                    write_seed("parse_flavor", row_id, row.get("input", ""))
                    parsed += 1
                    continue
                if not human:
                    continue
                write_seed("parse_flavor", row_id, human)
                # The render-closure target only accepts rows whose
                # canonical spelling re-parses (the corpus marks the rest).
                if "identifier" in row and not row.get("render_unreparseable"):
                    write_seed("roundtrip", row_id, human)
                for alias in row.get("non_normalized_aliases") or []:
                    spelling = alias.get("spelling")
                    if spelling:
                        write_seed("parse_flavor", f"{row_id}.alias", spelling)
                parsed += 1
        counts[flavor] = parsed
    total = sum(counts.values())
    suffix = f" (limit {options.limit}/flavor)" if options.limit else ""
    print(f"seeds written: {total} rows across {len(counts)} flavors{suffix}")


if __name__ == "__main__":
    main()
