"""Generates the Word Hunt word lists (see write_lists for the files).

Answers are common words: a morphology filter plus a word-frequency threshold per length.
Guesses are every valid word of the length, so a rare but real word is never rejected.

Sources (see README.md for versions and licenses):
  * frequency: wordfreq (Zipf scale, data CC BY-SA 4.0), same scale for both languages;
  * Russian morphology: OpenCorpora through pymorphy3-dicts-ru (CC BY-SA);
  * English words: SCOWL (Kevin Atkinson, permissive license), size buckets 10..95.

Usage:
  python generate.py --scowl PATH/TO/scowl-2020.12.07 [--report DIR]

--report writes, per language and length, how many answers each threshold keeps and the words
just above and below the chosen threshold, for review.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

import pymorphy3
from wordfreq import zipf_frequency

LENGTHS = range(4, 8)
ROOT = Path(__file__).resolve().parents[2]
WORDS_OUTPUT = ROOT / "client" / "public" / "words"
ANSWERS_OUTPUT = ROOT / "crates" / "games" / "word-hunt" / "data"

# Zipf frequency an answer needs, per language and length (3.0 = about once per million words).
# Chosen from the review samples; see README.md.
THRESHOLDS: dict[str, dict[int, float]] = {
    # wordfreq's Russian frequencies run lower than its English ones.
    "ru": {4: 2.8, 5: 2.8, 6: 2.8, 7: 2.8},
    "en": {4: 3.0, 5: 3.0, 6: 3.0, 7: 3.0},
}
REVIEW_THRESHOLDS = (2.5, 3.0, 3.5, 4.0)

RUSSIAN_LETTERS = re.compile(r"^[а-я]+$")
ENGLISH_LETTERS = re.compile(r"^[a-z]+$")

# Not words at all for this game: names, surnames, patronymics, places, organisations, brands,
# abbreviations, misspellings and distorted forms. Neither answers nor guesses.
RUSSIAN_PROPER_OR_BROKEN_TAGS = frozenset(
    {"Name", "Surn", "Patr", "Geox", "Orgn", "Trad", "Abbr", "Init", "Erro", "Dist"}
)
# Real words, so valid guesses, but too informal or old for an answer.
RUSSIAN_NOT_AN_ANSWER_TAGS = frozenset({"Infr", "Slng", "Arch"})
# A homonym of a place, surname or organisation gets its frequency from the name (wordfreq is
# case-insensitive: "кабул" counts Кабул), so it is not an answer. First names are left alone:
# роман, лев and вера are words first.
RUSSIAN_NAME_HOMONYM_TAGS = frozenset({"Surn", "Patr", "Geox", "Orgn", "Trad"})

# SCOWL sizes: words up to ANSWER_SIZE may be answers, up to GUESS_SIZE are accepted guesses.
SCOWL_ANSWER_SIZE = 50
SCOWL_GUESS_SIZE = 70
SCOWL_SIZES = (10, 20, 35, 40, 50, 55, 60, 70, 80, 95)
SCOWL_CATEGORIES = ("english", "american")


@dataclass(frozen=True)
class Candidate:
    word: str
    zipf: float


@dataclass
class LanguageWords:
    """Answer candidates (with frequency) and all accepted guesses, per length."""

    candidates: dict[int, list[Candidate]]
    guesses: dict[int, set[str]]


def is_plain_noun(morph: pymorphy3.MorphAnalyzer, word: str) -> bool:
    """The word's likeliest reading is this noun, and it is not also a place or a surname.

    Frequency counts the string, so a noun that is mostly another word ("дуло" is mostly the
    verb дуть) or a name would rank far above how common the noun is.
    """
    parses = morph.parse(word)
    if not parses:
        return False
    best = parses[0].tag
    if best.POS != "NOUN" or "nomn" not in best or "sing" not in best:
        return False
    return not any(parse.tag.grammemes & RUSSIAN_NAME_HOMONYM_TAGS for parse in parses)


def russian_words() -> LanguageWords:
    morph = pymorphy3.MorphAnalyzer()
    candidates: dict[int, dict[str, float]] = {length: {} for length in LENGTHS}
    guesses: dict[int, set[str]] = {length: set() for length in LENGTHS}
    for word, tag, normal_form, _paradigm, _index in morph.dictionary.iter_known_words():
        if tag.POS != "NOUN" or "nomn" not in tag or "sing" not in tag or word != normal_form:
            continue
        spelled = word.replace("ё", "е")
        if len(spelled) not in LENGTHS or not RUSSIAN_LETTERS.match(spelled):
            continue
        if tag.grammemes & RUSSIAN_PROPER_OR_BROKEN_TAGS:
            continue
        guesses[len(spelled)].add(spelled)
        if tag.grammemes & RUSSIAN_NOT_AN_ANSWER_TAGS or not is_plain_noun(morph, word):
            continue
        # wordfreq keeps ё; texts write it either way, so take the better of both spellings.
        zipf = max(zipf_frequency(word, "ru"), zipf_frequency(spelled, "ru"))
        known = candidates[len(spelled)]
        known[spelled] = max(zipf, known.get(spelled, 0.0))
    return LanguageWords(
        candidates={length: [Candidate(w, z) for w, z in words.items()] for length, words in candidates.items()},
        guesses=guesses,
    )


def scowl_words(scowl: Path, max_size: int) -> set[str]:
    words: set[str] = set()
    for category in SCOWL_CATEGORIES:
        for size in SCOWL_SIZES:
            if size > max_size:
                break
            path = scowl / "final" / f"{category}-words.{size}"
            if path.exists():
                words.update(path.read_text(encoding="latin-1").split())
    return {word for word in words if ENGLISH_LETTERS.match(word)}


def is_inflected_form(word: str, words: set[str]) -> bool:
    """A plural, past tense, -ing or superlative form of another listed word (cats, boxes, tried,
    making, bravest, scarier). Plain -er is kept: tower, power and water are words of their own."""
    stems = []
    if word.endswith("ier") or word.endswith("iest"):
        stems.append(word[: -3 if word.endswith("ier") else -4] + "y")
    if word.endswith("est"):
        stems += [word[:-3], word[:-2]]
        if len(word) > 5 and word[-4] == word[-5]:
            stems.append(word[:-4])
    if word.endswith("ies"):
        stems.append(word[:-3] + "y")
    if word.endswith("es"):
        stems.append(word[:-2])
    if word.endswith("s") and not word.endswith("ss"):
        stems.append(word[:-1])
    if word.endswith("ied"):
        stems.append(word[:-3] + "y")
    if word.endswith("ed"):
        stems += [word[:-2], word[:-1]]
        if len(word) > 4 and word[-3] == word[-4]:
            stems.append(word[:-3])
    if word.endswith("ing"):
        stems += [word[:-3], word[:-3] + "e"]
        if len(word) > 5 and word[-4] == word[-5]:
            stems.append(word[:-4])
    return any(stem in words for stem in stems if len(stem) >= 2)


def english_words(scowl: Path) -> LanguageWords:
    accepted = scowl_words(scowl, SCOWL_GUESS_SIZE)
    common = scowl_words(scowl, SCOWL_ANSWER_SIZE)
    candidates: dict[int, list[Candidate]] = {length: [] for length in LENGTHS}
    guesses: dict[int, set[str]] = {length: set() for length in LENGTHS}
    for word in accepted:
        if len(word) in LENGTHS:
            guesses[len(word)].add(word)
    for word in common:
        if len(word) in LENGTHS and not is_inflected_form(word, accepted):
            candidates[len(word)].append(Candidate(word, zipf_frequency(word, "en")))
    return LanguageWords(candidates=candidates, guesses=guesses)


def answers(words: LanguageWords, length: int, threshold: float) -> list[str]:
    return sorted(c.word for c in words.candidates[length] if c.zipf >= threshold)


def source_lines(language: str) -> list[str]:
    return [
        "# Word frequencies: wordfreq (CC BY-SA 4.0), https://github.com/rspeer/wordfreq.",
        (
            "# Words: OpenCorpora (CC BY-SA), https://opencorpora.org."
            if language == "ru"
            else "# Words: SCOWL, Copyright 2000-2018 Kevin Atkinson, http://wordlist.aspell.net."
        ),
        "# This file is licensed under CC BY-SA 4.0.",
    ]


def write_lists(language: str, words: LanguageWords) -> None:
    """Two UTF-8 files per language, each `<section> <length>` lines followed by sorted words:

    * crates/games/word-hunt/data/<language>-answers.txt — the answers, built into the game;
    * client/public/words/<language>.txt — every accepted guess (answers included), which only
      the client loads to check a guess before sending it.
    """
    answer_lines = [f"# Word Hunt answers ({language}), generated by tools/word-lists/generate.py."]
    word_lines = [f"# Word Hunt accepted words ({language}), generated by tools/word-lists/generate.py."]
    answer_lines += source_lines(language)
    word_lines += source_lines(language)
    for length in LENGTHS:
        chosen = answers(words, length, THRESHOLDS[language][length])
        answer_lines += [f"answers {length}", *chosen]
        word_lines += [f"words {length}", *sorted(words.guesses[length] | set(chosen))]
    for path, lines in (
        (ANSWERS_OUTPUT / f"{language}-answers.txt", answer_lines),
        (WORDS_OUTPUT / f"{language}.txt", word_lines),
    ):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


def write_report(language: str, words: LanguageWords, directory: Path) -> None:
    out = [f"# {language}"]
    for length in LENGTHS:
        ranked = sorted(words.candidates[length], key=lambda c: -c.zipf)
        counts = ", ".join(f"≥{t}: {sum(c.zipf >= t for c in ranked)}" for t in REVIEW_THRESHOLDS)
        threshold = THRESHOLDS[language][length]
        above = [c for c in ranked if c.zipf >= threshold][-30:]
        below = [c for c in ranked if c.zipf < threshold][:30]
        out += [
            "",
            f"## {length} letters: guesses {len(words.guesses[length])}, answers {counts}",
            f"last 30 kept (Zipf ≥ {threshold}):",
            "  " + ", ".join(f"{c.word} {c.zipf:.2f}" for c in above),
            "first 30 dropped:",
            "  " + ", ".join(f"{c.word} {c.zipf:.2f}" for c in below),
        ]
    directory.mkdir(parents=True, exist_ok=True)
    (directory / f"report-{language}.md").write_text("\n".join(out) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--scowl", type=Path, required=True, help="unpacked SCOWL release directory")
    parser.add_argument("--report", type=Path, help="also write threshold review reports here")
    args = parser.parse_args()
    for language, words in (("ru", russian_words()), ("en", english_words(args.scowl))):
        write_lists(language, words)
        if args.report:
            write_report(language, words, args.report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
