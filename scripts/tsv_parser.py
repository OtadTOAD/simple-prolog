import json
from collections import defaultdict
import re

# Input / output filenames
INPUT_FILE = "eng.tsv"
OUTPUT_FILE = "prolog_database.json"

# Map UniMorph POS tags to your word_type names
POS_MAP = {
    "V": "Verb",
    "N": "Noun",
    "ADJ": "Adjective",
    "ADV": "Adverb"
}

def is_valid_word(lemma):
    """Filter out non-standard words"""
    # Skip if starts with apostrophe (dialectal forms like 'Arry)
    if lemma.startswith("'"):
        return False
    # Skip if contains special characters, numbers, or symbols
    if re.search(r'[^a-zA-Z\-\']', lemma):
        return False
    # Skip if starts with uppercase (likely proper noun)
    if lemma[0].isupper():
        return False
    # Skip very short entries
    if len(lemma) < 2:
        return False
    # Skip entries with more than one hyphen or apostrophe
    if lemma.count('-') > 1 or lemma.count("'") > 1:
        return False
    return True

# Use (lemma, word_type) as key to keep separate entries
words = defaultdict(lambda: {"lemma": None, "word_type": None, "forms": set()})

# Read TSV lines
with open(INPUT_FILE, "r", encoding="utf8") as f:
    for line in f:
        if not line.strip():
            continue
        try:
            lemma, form, feats = line.strip().split("\t")
        except ValueError:
            # Skip malformed lines
            continue

        # Filter out invalid words
        if not is_valid_word(lemma):
            continue

        pos = feats.split(";")[0]  # e.g. "V", "N", "ADJ"
        
        # Only include mapped POS tags (skip "Other")
        if pos not in POS_MAP:
            continue
            
        word_type = POS_MAP[pos]

        # Key by both lemma AND word_type
        key = (lemma, word_type)
        entry = words[key]
        entry["lemma"] = lemma
        entry["word_type"] = word_type
        entry["forms"].add(lemma)
        if is_valid_word(form):
            entry["forms"].add(form)

# Convert sets to sorted lists
word_list = []
for (lemma, word_type), entry in sorted(words.items()):
    word_list.append({
        "lemma": entry["lemma"],
        "word_type": entry["word_type"],
        "forms": sorted(entry["forms"])
    })

# Final structure
output = {
    "words": word_list,
    "patterns": []
}

# Write to file
with open(OUTPUT_FILE, "w", encoding="utf8") as out:
    json.dump(output, out, indent=2, ensure_ascii=False)

print(f"✅ Saved to {OUTPUT_FILE} with {len(word_list)} entries.")