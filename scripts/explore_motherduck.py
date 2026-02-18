"""Explore the DigiLab MotherDuck shared tournament database.

Usage:
    pip install duckdb
    python scripts/explore_motherduck.py

Set env var `motherduck_token` if not using interactive browser auth.
Outputs schema + samples to both stdout and scripts/motherduck_schema.json.
"""

import json
import duckdb


DB_SHARE = "md:_share/digimon_tcg_tournament_db/2f86720e-c755-44c8-a3d6-dcf9b0d38913"
OUTPUT_FILE = "scripts/motherduck_schema.json"


def main():
    print("Connecting to MotherDuck...")
    con = duckdb.connect("md:")

    print(f"Attaching shared database:\n  {DB_SHARE}\n")
    con.execute(f"ATTACH '{DB_SHARE}'")

    # --- List all tables ---
    print("=" * 70)
    print("TABLES IN digimon_tcg_tournament_db")
    print("=" * 70)
    tables = con.execute("""
        SELECT table_schema, table_name, table_type
        FROM information_schema.tables
        WHERE table_catalog = 'digimon_tcg_tournament_db'
        ORDER BY table_schema, table_name
    """).fetchdf()
    print(tables.to_string(index=False))
    print()

    result = {}

    # --- Dump schema for each table ---
    for _, row in tables.iterrows():
        schema = row["table_schema"]
        table = row["table_name"]
        fqn = f"digimon_tcg_tournament_db.{schema}.{table}"

        print("-" * 70)
        print(f"TABLE: {fqn}")
        print("-" * 70)

        # Column info
        cols = con.execute(f"""
            SELECT column_name, data_type, is_nullable
            FROM information_schema.columns
            WHERE table_catalog = 'digimon_tcg_tournament_db'
              AND table_schema = '{schema}'
              AND table_name = '{table}'
            ORDER BY ordinal_position
        """).fetchdf()
        print(cols.to_string(index=False))

        # Row count
        count = con.execute(f"SELECT COUNT(*) AS row_count FROM {fqn}").fetchone()[0]
        print(f"\nRows: {count:,}")

        # Sample rows (up to 10)
        sample_df = None
        if count > 0:
            print("\nSample (5 rows):")
            sample_df = con.execute(f"SELECT * FROM {fqn} LIMIT 10").fetchdf()
            print(sample_df.head(5).to_string(index=False))
        print()

        # Store for JSON export
        result[fqn] = {
            "schema": schema,
            "table": table,
            "columns": cols.to_dict(orient="records"),
            "row_count": count,
            "sample_rows": (
                json.loads(sample_df.to_json(orient="records", date_format="iso"))
                if sample_df is not None
                else []
            ),
        }

    con.close()

    # Write JSON for later use
    with open(OUTPUT_FILE, "w") as f:
        json.dump(result, f, indent=2, default=str)
    print(f"Schema + samples written to {OUTPUT_FILE}")
    print("Done.")


if __name__ == "__main__":
    main()
