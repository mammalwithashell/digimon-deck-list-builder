import requests
from bs4 import BeautifulSoup
import urllib.parse
from collections import Counter
import re
import sys

# Base URL for Egman Events
BASE_URL = "https://egmanevents.com"
MAIN_PAGE_URL = "https://egmanevents.com/digimon-bt23-tournaments"

def get_tournament_links(main_url):
    print(f"Fetching main page: {main_url}")
    try:
        response = requests.get(main_url)
        response.raise_for_status()
    except requests.RequestException as e:
        print(f"Error fetching main page: {e}")
        return []

    soup = BeautifulSoup(response.content, 'html.parser')
    links = set()

    # Look for links that contain 'digimon-bt23-tournaments' but are not the main page itself or rss/categories
    for a in soup.find_all('a', href=True):
        href = a['href']
        # Normalize URL
        if not href.startswith('http'):
            href = urllib.parse.urljoin(BASE_URL, href)

        if '/digimon-bt23-tournaments/' in href and 'category' not in href and 'rss' not in href and '#' not in href:
            links.add(href)

    print(f"Found {len(links)} tournament links.")
    return list(links)

def get_deck_links(tournament_url):
    print(f"  Scanning tournament: {tournament_url}")
    try:
        response = requests.get(tournament_url)
        response.raise_for_status()
    except requests.RequestException as e:
        print(f"  Error fetching tournament page: {e}")
        return []

    soup = BeautifulSoup(response.content, 'html.parser')
    deck_links = set()

    for a in soup.find_all('a', href=True):
        href = a['href']
        if 'deckbuilder.egmanevents.com' in href and 'deck=' in href:
            deck_links.add(href)

    print(f"  Found {len(deck_links)} deck links.")
    return list(deck_links)

def parse_deck_link(url):
    parsed = urllib.parse.urlparse(url)
    query = urllib.parse.parse_qs(parsed.query)

    if 'deck' not in query:
        return {}

    deck_string = query['deck'][0]
    # Format is CardID:Count,CardID:Count...
    # e.g. ST12-12:2,BT23-054:4

    card_counts = {}
    items = deck_string.split(',')
    for item in items:
        if ':' in item:
            card_id, count_str = item.split(':')
            try:
                count = int(count_str)
                card_counts[card_id] = count
            except ValueError:
                continue
    return card_counts

def extract_set_id(card_id):
    # Usually SET-NUMBER, e.g. BT23-054 -> BT23
    # P-001 -> P
    # ST1-01 -> ST1
    if '-' in card_id:
        return card_id.split('-')[0]
    return "UNKNOWN"

def main():
    tournament_links = get_tournament_links(MAIN_PAGE_URL)

    total_set_counts = Counter()
    total_decks_processed = 0

    for t_link in tournament_links:
        deck_links = get_deck_links(t_link)
        for d_link in deck_links:
            card_counts = parse_deck_link(d_link)
            if card_counts:
                total_decks_processed += 1
                for card_id, count in card_counts.items():
                    set_id = extract_set_id(card_id)
                    total_set_counts[set_id] += count

    print("\n" + "="*40)
    print(f"Processed {total_decks_processed} decks.")
    print("Set Priority List (by total card usage count):")
    print("="*40)

    sorted_sets = total_set_counts.most_common()

    for set_id, count in sorted_sets:
        print(f"{set_id}: {count}")

    # Write to file
    with open('python_impl/scraper/priority_sets.txt', 'w') as f:
        f.write(f"Processed {total_decks_processed} decks.\n")
        f.write("Set Priority List (by total card usage count):\n")
        for set_id, count in sorted_sets:
            f.write(f"{set_id}: {count}\n")

if __name__ == "__main__":
    main()
