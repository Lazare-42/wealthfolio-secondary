#!/usr/bin/env python3
"""Build a Wealthfolio NAV-healing envelope from a broker holdings export.

The server's `nav_healing` watcher consumes JSON envelopes dropped into
`{data_root}/nav-inbox/` and writes a MANUAL quote per holding, refreshing
illiquid/manually-priced assets that no market provider can quote.

Envelope (version 1):
    {"version":1,"source":"...","asOf":"YYYY-MM-DD",
     "prices":[{"isin":"LU...","nav":142.83,"currency":"EUR","name":"..."}]}

Adapters:
  hsbc   HSBC "All holdings" export (UTF-16, tab-delimited): ISIN col + Market price col.
  csv    Generic delimited file with explicit column names/indexes.

Usage:
  nav-envelope.py hsbc HOLDINGS.csv [--as-of YYYY-MM-DD] [--out FILE|-]
  nav-envelope.py csv FILE --isin-col ISIN --nav-col Price [--ccy-col Cur]
                  [--name-col Name] [--delim ,] [--encoding utf-8]
                  [--as-of YYYY-MM-DD] [--out FILE|-]

Default --out is stdout. Pass a directory (e.g. the nav-inbox) and a dated
filename is generated. Nothing is written to the DB; the running server applies
the envelope on its next 30s poll.
"""
import argparse
import csv
import io
import json
import os
import re
import sys
from datetime import date


def parse_number(raw):
    """'+32,647.94' / '1 234,56' / '-' -> float or None."""
    if raw is None:
        return None
    s = raw.strip().strip('"').strip()
    if s in ("", "-", "n/a", "N/A"):
        return None
    s = s.replace(" ", "").replace(" ", "").lstrip("+")
    # If both separators present, assume ',' is thousands.
    if "," in s and "." in s:
        s = s.replace(",", "")
    elif "," in s:  # lone comma = decimal (FR) unless it looks like thousands
        s = s.replace(",", "." if s.rfind(",") >= len(s) - 3 else "")
    try:
        v = float(s)
        return v if v > 0 else None
    except ValueError:
        return None


def date_from_name(path):
    m = re.search(r"(20\d{6})", os.path.basename(path))
    if m:
        y = m.group(1)
        return f"{y[0:4]}-{y[4:6]}-{y[6:8]}"
    return None


def adapter_hsbc(path, as_of):
    text = open(path, encoding="utf-16").read()
    rows = list(csv.reader(io.StringIO(text), delimiter="\t"))
    header = rows[0]
    idx = {name: i for i, name in enumerate(header)}
    isin_i = idx["ISIN/Ref"]
    nav_i = idx["Market price"]
    ccy_i = idx.get("Holding currency")
    name_i = idx.get("Name", 0)
    prices = []
    for r in rows[1:]:
        if len(r) <= nav_i:
            continue
        isin = r[isin_i].strip().strip('"')
        nav = parse_number(r[nav_i])
        if not isin or isin == "-" or nav is None:
            continue
        p = {"isin": isin, "nav": nav}
        if ccy_i is not None and ccy_i < len(r):
            c = r[ccy_i].strip().strip('"')
            if c and c != "-":
                p["currency"] = c
        if name_i < len(r):
            p["name"] = r[name_i].strip().strip('"')
        prices.append(p)
    return prices, as_of or date_from_name(path)


def adapter_csv(path, args):
    text = open(path, encoding=args.encoding).read()
    rows = list(csv.DictReader(io.StringIO(text), delimiter=args.delim))
    prices = []
    for r in rows:
        isin = (r.get(args.isin_col) or "").strip()
        nav = parse_number(r.get(args.nav_col))
        if not isin or nav is None:
            continue
        p = {"isin": isin, "nav": nav}
        if args.ccy_col and r.get(args.ccy_col):
            p["currency"] = r[args.ccy_col].strip()
        if args.name_col and r.get(args.name_col):
            p["name"] = r[args.name_col].strip()
        prices.append(p)
    return prices, args.as_of or date_from_name(path)


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    h = sub.add_parser("hsbc")
    h.add_argument("file")
    h.add_argument("--as-of")
    h.add_argument("--out", default="-")

    c = sub.add_parser("csv")
    c.add_argument("file")
    c.add_argument("--isin-col", required=True)
    c.add_argument("--nav-col", required=True)
    c.add_argument("--ccy-col")
    c.add_argument("--name-col")
    c.add_argument("--delim", default=",")
    c.add_argument("--encoding", default="utf-8")
    c.add_argument("--as-of")
    c.add_argument("--out", default="-")

    args = ap.parse_args()

    if args.cmd == "hsbc":
        prices, as_of = adapter_hsbc(args.file, args.as_of)
        source = os.path.basename(args.file)
    else:
        prices, as_of = adapter_csv(args.file, args)
        source = os.path.basename(args.file)

    if not prices:
        sys.exit("no priceable holdings found")
    if not as_of:
        sys.exit("could not determine as-of date; pass --as-of YYYY-MM-DD")

    envelope = {"version": 1, "source": source, "asOf": as_of, "prices": prices}
    payload = json.dumps(envelope, indent=2, ensure_ascii=False)

    if args.out == "-":
        print(payload)
    else:
        out = args.out
        if os.path.isdir(out):
            stem = re.sub(r"[^A-Za-z0-9_.-]", "_", os.path.splitext(source)[0])
            out = os.path.join(out, f"nav-{as_of}-{stem}.json")
        with open(out, "w", encoding="utf-8") as f:
            f.write(payload)
        print(f"wrote {len(prices)} NAV(s) -> {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
