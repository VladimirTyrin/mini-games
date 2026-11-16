# Word Hunt word lists

`generate.py` builds the word lists of Word Hunt:

| File | What | Used by |
|---|---|---|
| `crates/games/word-hunt/data/<lang>-answers.txt` | words the game may ask for | built into the game (WASM, server) |
| `client/public/words/<lang>.txt` | every accepted guess (answers included) | the client, to refuse non-words before sending |

The game itself accepts any letters of the right length: whether a guess is a real word is the
client's check, so replays never depend on a list version. The answer is sent as a world event
with the word itself.

## Sources and licenses

- **Frequency:** [wordfreq](https://github.com/rspeer/wordfreq) 3.1 (data CC BY-SA 4.0), the
  Zipf scale for both languages (3.0 ≈ once per million words).
- **Russian:** [OpenCorpora](https://opencorpora.org) dictionary (CC BY-SA) through
  `pymorphy3-dicts-ru`. Nouns in the nominative singular; names, surnames, patronymics, places,
  organisations, abbreviations and broken forms are dropped everywhere; informal, slang and
  archaic words are guesses only. An answer must also be the likeliest reading of its spelling
  and not a homonym of a place or surname (wordfreq ignores case, so "кабул" counts Кабул).
  `ё` is written `е`.
- **English:** [SCOWL](http://wordlist.aspell.net) 2020.12.07 by Kevin Atkinson (permissive
  license). Guesses: sizes up to 70; answers: up to 50, without plurals, past tenses, -ing,
  superlative and -ier forms of other listed words.

The generated lists are CC BY-SA 4.0; the game shows the attribution on its settings form.

## Thresholds

Answers need a Zipf frequency of 3.0 (English) or 2.8 (Russian, whose wordfreq frequencies run
lower), per length in `THRESHOLDS`. There is no stop-list on purpose (for now); a stray word
near the threshold is a difficulty choice.

## Regenerate

```bash
python -m venv .venv && .venv/bin/pip install -r tools/word-lists/requirements.txt
# SCOWL: https://downloads.sourceforge.net/wordlist/scowl-2020.12.07.tar.gz, unpacked anywhere
.venv/bin/python tools/word-lists/generate.py --scowl path/to/scowl-2020.12.07 --report /tmp/wh
```

`--report` writes, per language and length, how many answers each threshold keeps and the words
just above and below the chosen one. Changing the answers changes the daily words and the
golden hash of `mg-word-hunt` (`scripted_hash_is_pinned`).
