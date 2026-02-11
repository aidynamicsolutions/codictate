# custom-word-correction Specification

## Purpose
TBD - created by archiving change improve-custom-word-algorithm. Update Purpose after archive.
## Requirements
### Requirement: Exact Match Priority

Custom word matching MUST check for exact case-insensitive matches before performing fuzzy matching.

#### Scenario: Exact match is found

- **Given** the user has added "Handy" to custom words
- **When** the transcription contains the word "handy" (lowercase)
- **Then** the word is replaced with "Handy" (preserving the custom word's casing)
- **And** no fuzzy matching is performed for this word

#### Scenario: Exact match with different casing

- **Given** the user has added "ChatGPT" to custom words
- **When** the transcription contains "CHATGPT"
- **Then** the word is replaced with "CHATGPT" (preserving original ALL CAPS pattern)
- **And** no fuzzy matching is performed

### Requirement: Phonetic Matching with Double Metaphone

Custom word matching MUST use Double Metaphone algorithm for phonetic similarity detection.

#### Scenario: Primary phonetic code match

- **Given** the user has added "Smith" to custom words
- **When** the transcription contains "Smyth"
- **Then** the words match phonetically via primary code comparison
- **And** the match is weighted appropriately

#### Scenario: Secondary phonetic code match

- **Given** the user has added "Schmidt" to custom words
- **When** the transcription contains "Smith"
- **Then** the words may match via secondary code comparison
- **And** the match is weighted appropriately

#### Scenario: Non-English pronunciation handling

- **Given** the user has added "Nguyen" to custom words
- **When** the transcription contains a phonetically similar variant
- **Then** Double Metaphone handles the non-English pronunciation
- **And** matching is more accurate than Soundex would provide

### Requirement: Transposition-Aware Edit Distance

Custom word matching MUST treat character transpositions as a single edit operation.

#### Scenario: Adjacent character swap

- **Given** the user has added "the" to custom words
- **When** the transcription contains "teh"
- **Then** the edit distance is calculated as 1 (not 2)
- **And** matching sensitivity is improved for common typos

#### Scenario: Multiple transpositions

- **Given** the user has added "receive" to custom words
- **When** the transcription contains "recieve" (common misspelling)
- **Then** the transposition "ie" → "ei" counts as 1 edit
- **And** the word matches more reliably

### Requirement: Threshold-Based Acceptance

Matches MUST only be accepted if the combined score is below the configured threshold.

#### Scenario: Score below threshold

- **Given** the threshold is set to 0.18
- **When** a word's combined score is 0.15
- **Then** the correction is applied

#### Scenario: Score above threshold

- **Given** the threshold is set to 0.18
- **When** a word's combined score is 0.25
- **Then** the original word is kept unchanged

### Requirement: Case Pattern Preservation

Corrections MUST preserve the case pattern of the original word.

#### Scenario: All uppercase original

- **Given** the custom word is "hello"
- **When** the original word is "HELO"
- **Then** the correction is "HELLO"

#### Scenario: Title case original

- **Given** the custom word is "world"
- **When** the original word is "Wrold"
- **Then** the correction is "World"

### Requirement: Punctuation Preservation

Corrections MUST preserve punctuation before and after the word.

#### Scenario: Trailing punctuation

- **Given** the custom word is "hello"
- **When** the original word is "helo?"
- **Then** the correction is "hello?"

#### Scenario: Surrounding punctuation

- **Given** the custom word is "hello"
- **When** the original word is "...helo!"
- **Then** the correction is "...hello!"

