#!/usr/bin/env python3
"""
Scrape Slay the Spire Wiki for event data.

This script:
1. Fetches event pages from slay-the-spire.fandom.com using cloudscraper (bypasses Cloudflare)
2. Extracts Options and Dialogue sections
3. Saves raw data to JSON for later AI processing
"""

import json
import os
import re
import time
from pathlib import Path
from urllib.parse import quote, unquote

import cloudscraper
from bs4 import BeautifulSoup

# Configuration - Use Fandom wiki (more scraper-friendly than wiki.gg)
WIKI_BASE = "https://slay-the-spire.fandom.com/wiki/"
OUTPUT_DIR = Path(__file__).parent.parent / "data" / "wiki_scrape"
RAW_HTML_DIR = OUTPUT_DIR / "raw_html"
EXTRACTED_DIR = OUTPUT_DIR / "extracted"

# Rate limiting
REQUEST_DELAY = 2.0  # seconds between requests (be polite)

# All events to scrape (from the Events page)
EVENTS = {
    # Shrines (All Acts)
    "A_Note_For_Yourself": {"name": "A Note For Yourself", "acts": ["All"], "shrine": True},
    "Bonfire_Spirits": {"name": "Bonfire Spirits", "acts": ["All"], "shrine": True},
    "Duplicator": {"name": "Duplicator", "acts": ["All"], "shrine": True},
    "Golden_Shrine": {"name": "Golden Shrine", "acts": ["All"], "shrine": True},
    "Lab": {"name": "Lab", "acts": ["All"], "shrine": True},
    "Match_and_Keep": {"name": "Match and Keep", "acts": ["All"], "shrine": True},
    "Ominous_Forge": {"name": "Ominous Forge", "acts": ["All"], "shrine": False},
    "Purifier": {"name": "Purifier", "acts": ["All"], "shrine": True},
    "The_Divine_Fountain": {"name": "The Divine Fountain", "acts": ["All"], "shrine": True},
    "The_Woman_in_Blue": {"name": "The Woman in Blue", "acts": ["All"], "shrine": True},
    "Transmogrifier": {"name": "Transmogrifier", "acts": ["All"], "shrine": True},
    "Upgrade_Shrine": {"name": "Upgrade Shrine", "acts": ["All"], "shrine": True},
    "We_Meet_Again!": {"name": "We Meet Again!", "acts": ["All"], "shrine": True},
    "Wheel_of_Change": {"name": "Wheel of Change", "acts": ["All"], "shrine": True},
    
    # Act 1
    "Big_Fish": {"name": "Big Fish", "acts": [1], "shrine": False},
    "Dead_Adventurer": {"name": "Dead Adventurer", "acts": [1], "shrine": False},
    "Face_Trader": {"name": "Face Trader", "acts": [1, 2], "shrine": True},
    "Golden_Idol": {"name": "Golden Idol", "acts": [1], "shrine": False},
    "Hypnotizing_Colored_Mushrooms": {"name": "Hypnotizing Colored Mushrooms", "acts": [1], "shrine": False},
    "Living_Wall": {"name": "Living Wall", "acts": [1], "shrine": False},
    "Scrap_Ooze": {"name": "Scrap Ooze", "acts": [1], "shrine": False},
    "Shining_Light": {"name": "Shining Light", "acts": [1], "shrine": False},
    "The_Cleric": {"name": "The Cleric", "acts": [1], "shrine": False},
    "The_Ssssserpent": {"name": "The Ssssserpent", "acts": [1], "shrine": False},
    "Wing_Statue": {"name": "Wing Statue", "acts": [1], "shrine": False},
    "World_of_Goop": {"name": "World of Goop", "acts": [1], "shrine": False},
    
    # Act 2
    "Ancient_Writing": {"name": "Ancient Writing", "acts": [2], "shrine": False},
    "Augmenter": {"name": "Augmenter", "acts": [2], "shrine": False},
    "Council_of_Ghosts": {"name": "Council of Ghosts", "acts": [2], "shrine": False},
    "Cursed_Tome": {"name": "Cursed Tome", "acts": [2], "shrine": False},
    "Designer_In-Spire": {"name": "Designer In-Spire", "acts": [2, 3], "shrine": True},
    "Forgotten_Altar": {"name": "Forgotten Altar", "acts": [2], "shrine": False},
    "Knowing_Skull": {"name": "Knowing Skull", "acts": [2], "shrine": True},
    "Masked_Bandits": {"name": "Masked Bandits", "acts": [2], "shrine": False},
    "N'loth": {"name": "N'loth", "acts": [2], "shrine": True},
    "Old_Beggar": {"name": "Old Beggar", "acts": [2], "shrine": False},
    "Pleading_Vagrant": {"name": "Pleading Vagrant", "acts": [2], "shrine": False},
    "The_Colosseum": {"name": "The Colosseum", "acts": [2], "shrine": False},
    "The_Joust": {"name": "The Joust", "acts": [2], "shrine": True},
    "The_Library": {"name": "The Library", "acts": [2], "shrine": False},
    "The_Mausoleum": {"name": "The Mausoleum", "acts": [2], "shrine": False},
    "The_Nest": {"name": "The Nest", "acts": [2], "shrine": False},
    "Vampires": {"name": "Vampires", "acts": [2], "shrine": False},
    
    # Act 3
    "Falling": {"name": "Falling", "acts": [3], "shrine": False},
    "Mind_Bloom": {"name": "Mind Bloom", "acts": [3], "shrine": False},
    "Mysterious_Sphere": {"name": "Mysterious Sphere", "acts": [3], "shrine": False},
    "Secret_Portal": {"name": "Secret Portal", "acts": [3], "shrine": True},
    "Sensory_Stone": {"name": "Sensory Stone", "acts": [3], "shrine": False},
    "The_Moai_Head": {"name": "The Moai Head", "acts": [3], "shrine": False},
    "Tomb_of_Lord_Red_Mask": {"name": "Tomb of Lord Red Mask", "acts": [3], "shrine": False},
    "Winding_Halls": {"name": "Winding Halls", "acts": [3], "shrine": False},
}


def setup_dirs():
    """Create output directories if they don't exist."""
    RAW_HTML_DIR.mkdir(parents=True, exist_ok=True)
    EXTRACTED_DIR.mkdir(parents=True, exist_ok=True)


def get_session():
    """Create a cloudscraper session to bypass Cloudflare protection."""
    scraper = cloudscraper.create_scraper(
        browser={
            'browser': 'chrome',
            'platform': 'windows',
            'desktop': True
        }
    )
    return scraper


# Global session
SESSION = None

def fetch_page(wiki_id: str) -> str | None:
    """Fetch a wiki page and return its HTML content."""
    global SESSION
    if SESSION is None:
        SESSION = get_session()
    
    url = WIKI_BASE + wiki_id
    print(f"  Fetching: {url}")
    
    try:
        response = SESSION.get(url, timeout=30)
        response.raise_for_status()
        return response.text
    except Exception as e:
        print(f"    ERROR: {e}")
        return None


def extract_event_data(html: str, event_info: dict) -> dict:
    """Extract structured data from event page HTML."""
    soup = BeautifulSoup(html, "html.parser")
    
    result = {
        "name": event_info["name"],
        "acts": event_info["acts"],
        "shrine": event_info["shrine"],
        "options": [],
        "dialogue": {},
        "notes": [],
        "raw_options_text": "",
        "raw_dialogue_text": "",
    }
    
    # Find the main content
    content = soup.find("div", {"class": "mw-parser-output"})
    if not content:
        content = soup.find("main") or soup.find("body")
    
    if not content:
        return result
    
    # Extract Options section
    options_header = content.find(["h2", "span"], string=re.compile(r"Options", re.I))
    if options_header:
        options_section = []
        # Get all elements until next h2
        sibling = options_header.find_next()
        while sibling and sibling.name != "h2":
            if sibling.name in ["p", "ul", "li", "div"]:
                text = sibling.get_text(strip=True)
                if text:
                    options_section.append(text)
            sibling = sibling.find_next_sibling()
        result["raw_options_text"] = "\n".join(options_section)
    
    # Try alternate method - find section by id
    if not result["raw_options_text"]:
        options_span = content.find("span", {"id": "Options"})
        if options_span:
            parent = options_span.find_parent("h2")
            if parent:
                options_section = []
                sibling = parent.find_next_sibling()
                while sibling and sibling.name != "h2":
                    text = sibling.get_text(separator=" ", strip=True)
                    if text:
                        options_section.append(text)
                    sibling = sibling.find_next_sibling()
                result["raw_options_text"] = "\n".join(options_section)
    
    # Extract Dialogue section
    dialogue_header = content.find(["h2", "span"], string=re.compile(r"Dialogue", re.I))
    if dialogue_header:
        dialogue_section = []
        sibling = dialogue_header.find_next()
        while sibling and sibling.name != "h2":
            if sibling.name in ["p", "ul", "li", "div", "blockquote"]:
                text = sibling.get_text(separator=" ", strip=True)
                if text:
                    dialogue_section.append(text)
            sibling = sibling.find_next_sibling()
        result["raw_dialogue_text"] = "\n".join(dialogue_section)
    
    # Try alternate method
    if not result["raw_dialogue_text"]:
        dialogue_span = content.find("span", {"id": "Dialogue"})
        if dialogue_span:
            parent = dialogue_span.find_parent("h2")
            if parent:
                dialogue_section = []
                sibling = parent.find_next_sibling()
                while sibling and sibling.name != "h2":
                    text = sibling.get_text(separator=" ", strip=True)
                    if text:
                        dialogue_section.append(text)
                    sibling = sibling.find_next_sibling()
                result["raw_dialogue_text"] = "\n".join(dialogue_section)
    
    # Extract Notes section if exists
    notes_header = content.find(["h2", "span"], string=re.compile(r"Notes", re.I))
    if notes_header:
        notes_section = []
        sibling = notes_header.find_next()
        while sibling and sibling.name != "h2":
            if sibling.name in ["p", "ul", "li"]:
                text = sibling.get_text(strip=True)
                if text:
                    notes_section.append(text)
            sibling = sibling.find_next_sibling()
        result["notes"] = notes_section
    
    return result


def scrape_all_events(skip_existing: bool = True):
    """Scrape all events and save data."""
    setup_dirs()
    
    all_extracted = {}
    total = len(EVENTS)
    
    for i, (wiki_id, event_info) in enumerate(EVENTS.items(), 1):
        print(f"\n[{i}/{total}] Processing: {event_info['name']}")
        
        # Check if already scraped
        html_file = RAW_HTML_DIR / f"{wiki_id}.html"
        extracted_file = EXTRACTED_DIR / f"{wiki_id}.json"
        
        if skip_existing and html_file.exists() and extracted_file.exists():
            print("  Skipping (already exists)")
            # Load existing extracted data
            with open(extracted_file, "r", encoding="utf-8") as f:
                all_extracted[wiki_id] = json.load(f)
            continue
        
        # Fetch page
        html = fetch_page(wiki_id)
        if not html:
            print("  Failed to fetch, skipping...")
            continue
        
        # Save raw HTML
        with open(html_file, "w", encoding="utf-8") as f:
            f.write(html)
        print(f"  Saved HTML: {html_file.name}")
        
        # Extract data
        extracted = extract_event_data(html, event_info)
        all_extracted[wiki_id] = extracted
        
        # Save individual extracted file
        with open(extracted_file, "w", encoding="utf-8") as f:
            json.dump(extracted, f, indent=2, ensure_ascii=False)
        print(f"  Saved extracted: {extracted_file.name}")
        
        # Rate limiting
        time.sleep(REQUEST_DELAY)
    
    # Save combined file
    combined_file = OUTPUT_DIR / "all_events_raw.json"
    with open(combined_file, "w", encoding="utf-8") as f:
        json.dump(all_extracted, f, indent=2, ensure_ascii=False)
    print(f"\n✅ Saved combined data to: {combined_file}")
    
    return all_extracted


def print_summary(data: dict):
    """Print summary of scraped data."""
    print("\n" + "=" * 60)
    print("SCRAPING SUMMARY")
    print("=" * 60)
    
    total = len(data)
    with_options = sum(1 for e in data.values() if e.get("raw_options_text"))
    with_dialogue = sum(1 for e in data.values() if e.get("raw_dialogue_text"))
    
    print(f"Total events: {total}")
    print(f"With Options text: {with_options}")
    print(f"With Dialogue text: {with_dialogue}")
    
    # Show sample
    print("\n--- Sample Event (Big_Fish) ---")
    if "Big_Fish" in data:
        bf = data["Big_Fish"]
        print(f"Name: {bf['name']}")
        print(f"Acts: {bf['acts']}")
        print(f"Options text (first 300 chars):")
        print(f"  {bf.get('raw_options_text', '')[:300]}...")


if __name__ == "__main__":
    print("=" * 60)
    print("Slay the Spire Wiki Event Scraper")
    print("=" * 60)
    
    # Check for required packages
    try:
        import cloudscraper
        from bs4 import BeautifulSoup
    except ImportError as e:
        print(f"\nMissing required package: {e}")
        print("Install with: pip install cloudscraper beautifulsoup4")
        exit(1)
    
    # Run scraper
    data = scrape_all_events(skip_existing=True)
    print_summary(data)
    
    print("\n✅ Done! Next step: Run clean_event_data.py to process with AI")
