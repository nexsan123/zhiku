"""
One-time batch reclassification of news stuck with default "market" category.
Sends only the title to Groq for lightweight category-only classification.
"""

import sqlite3
import json
import time
import sys
import io
import os
import subprocess
from collections import Counter

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

# --- Config ---
DB_PATH = os.path.join(os.environ['APPDATA'], 'com.zhiku.app', 'zhiku.db')
SETTINGS_PATH = os.path.join(os.environ['APPDATA'], 'com.zhiku.app', 'settings.json')

with open(SETTINGS_PATH, 'r') as f:
    settings = json.load(f)

groq_model = None
groq_key = None
for m in settings.get('ai_models', []):
    if m.get('provider') == 'groq' and m.get('enabled'):
        groq_key = m['apiKey']
        groq_model = m['modelName']
        break

if not groq_key:
    print("ERROR: No enabled Groq model found in settings.json")
    sys.exit(1)

GROQ_URL = "https://api.groq.com/openai/v1/chat/completions"

VALID_CATEGORIES = {
    'macro_policy', 'market', 'geopolitical', 'central_bank',
    'trade', 'crypto', 'energy', 'supply_chain'
}

NORMALIZE_MAP = {
    'macros': 'macro_policy', 'macro': 'macro_policy', 'policy': 'macro_policy',
    'geo_political': 'geopolitical', 'geopolitic': 'geopolitical', 'geopolitics': 'geopolitical',
    'centralbank': 'geopolitical', 'central': 'central_bank',
    'crypt': 'crypto', 'cryptocurrency': 'crypto', 'blockchain': 'crypto',
    'oil': 'energy', 'gas': 'energy',
    'tariff': 'trade', 'tariffs': 'trade',
    'supplychain': 'supply_chain', 'logistics': 'supply_chain',
    'markets': 'market', 'stock': 'market', 'stocks': 'market',
    'stock_market': 'market', 'finance': 'market',
}

PROMPT_TEMPLATE = """只根据标题判断新闻类别，回复 JSON: {{"category":"xxx"}}

标题: {title}

category 必须是以下8个值之一:
macro_policy | market | geopolitical | central_bank | trade | crypto | energy | supply_chain

判断标准:
- geopolitical: 国际关系、制裁、军事、领土争端、外交、战争、联盟
- macro_policy: 货币政策、财政政策、利率、通胀、GDP、就业数据
- central_bank: 央行声明、利率决议、QE/QT、央行官员讲话
- trade: 关税、贸易协定、进出口、WTO
- energy: 石油、天然气、OPEC、可再生能源、能源价格
- crypto: 加密货币、区块链、DeFi、稳定币
- supply_chain: 航运、芯片短缺、物流中断、制造业转移
- market: 仅用于纯市场行情（股价、指数、IPO、财报）

只输出 JSON:"""


def normalize_category(raw: str) -> str:
    lower = raw.lower().replace('-', '_').replace(' ', '_')
    if lower in VALID_CATEGORIES:
        return lower
    return NORMALIZE_MAP.get(lower, 'market')


def call_groq(title: str, retries: int = 1) -> str | None:
    # Escape title for JSON safety
    safe_title = title.replace('\\', '\\\\').replace('"', '\\"')
    prompt = PROMPT_TEMPLATE.replace('{title}', safe_title)
    payload = json.dumps({
        "model": groq_model,
        "messages": [
            {"role": "system", "content": "You are a news classifier. Only output JSON."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.1,
        "max_tokens": 30,
    })

    for attempt in range(retries + 1):
        try:
            result = subprocess.run(
                ['curl', '-s', '-w', '\nHTTP_CODE:%{http_code}', GROQ_URL,
                 '-H', f'Authorization: Bearer {groq_key}',
                 '-H', 'Content-Type: application/json',
                 '-d', payload],
                capture_output=True, text=True, timeout=20
            )
            output = result.stdout.strip()
            # Split HTTP code from body
            parts = output.rsplit('\nHTTP_CODE:', 1)
            body = parts[0] if len(parts) == 2 else output
            http_code = int(parts[1]) if len(parts) == 2 else 0

            if http_code == 429 and attempt < retries:
                print(f"  Rate limited, waiting 30s...", flush=True)
                time.sleep(30)
                continue
            if http_code != 200:
                print(f"  HTTP {http_code}", flush=True)
                return None

            data = json.loads(body)
            content = data['choices'][0]['message']['content'].strip()
            if '{' in content:
                json_str = content[content.index('{'):content.rindex('}') + 1]
                parsed = json.loads(json_str)
                return normalize_category(parsed.get('category', 'market'))
            return None
        except Exception as e:
            print(f"  Error: {e}", flush=True)
            return None
    return None


def main():
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()

    # Count candidates
    c.execute("SELECT COUNT(*) FROM news WHERE category = 'market' AND ai_summary IS NOT NULL")
    total = c.fetchone()[0]
    print(f"=== Reclassify Stale News ===", flush=True)
    print(f"Candidates: {total} (market + has ai_summary)", flush=True)
    print(f"Model: {groq_model}", flush=True)
    print(flush=True)

    if total == 0:
        print("Nothing to reclassify.")
        return

    # Fetch all candidates
    c.execute("""
        SELECT id, title FROM news
        WHERE category = 'market' AND ai_summary IS NOT NULL
        ORDER BY published_at ASC
    """)
    candidates = c.fetchall()

    changed = 0
    unchanged = 0
    failed = 0
    category_changes = Counter()
    batch_size = 50

    for i, (news_id, title) in enumerate(candidates):
        if i > 0 and i % batch_size == 0:
            print(f"\n--- Progress: {i}/{total} (changed: {changed}, unchanged: {unchanged}, failed: {failed}) ---\n", flush=True)

        # Throttle: 1.5s between requests (Groq has generous TPM for short prompts)
        if i > 0:
            time.sleep(1.5)

        new_cat = call_groq(title)

        if new_cat is None:
            failed += 1
            continue

        if new_cat != 'market':
            c.execute("UPDATE news SET category = ? WHERE id = ?", (new_cat, news_id))
            conn.commit()
            changed += 1
            category_changes[new_cat] += 1
            safe_title = title[:60]
            try:
                print(f"  [{i+1}/{total}] market -> {new_cat:15s} | {safe_title}", flush=True)
            except UnicodeEncodeError:
                print(f"  [{i+1}/{total}] market -> {new_cat:15s} | (title has special chars)", flush=True)
        else:
            unchanged += 1

    # Final report
    print(f"\n{'='*60}")
    print(f"=== RECLASSIFICATION COMPLETE ===")
    print(f"{'='*60}")
    print(f"Total candidates:  {total}")
    print(f"Changed:           {changed}")
    print(f"Unchanged (market):{unchanged}")
    print(f"Failed:            {failed}")
    print(f"\nCategory distribution of changes:")
    for cat, cnt in category_changes.most_common():
        print(f"  {cat:20s} +{cnt}")

    # Show final overall distribution
    print(f"\n--- Final DB category distribution ---")
    c.execute("SELECT category, COUNT(*) FROM news GROUP BY category ORDER BY COUNT(*) DESC")
    for row in c.fetchall():
        print(f"  {row[0]:20s} {row[1]:>6d}")

    conn.close()


if __name__ == '__main__':
    main()
