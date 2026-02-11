## Context
The goal is to provide native filler word removal for all 15 supported languages. 
Currently, the system only supports English filler words.

## Filler Word Research Findings
The following filler words have been identified for implementation:

- **Arabic (ar)**: `يعني` (ya'ni), `اممم` (ummm), `طيب` (tayyib)
- **Czech (cs)**: `ehm`, `no`, `jakoby`
- **German (de)**: `ähm`, `also`, `tja`, `halt`
- **Spanish (es)**: `eh`, `este`, `em`, `bueno`
- **French (fr)**: `euh`, `ben`, `bah`, `genre`
- **Italian (it)**: `ehm`, `cioè`, `allora`
- **Japanese (ja)**: `えっと` (etto), `あの` (ano), `んー` (n-)
- **Korean (ko)**: `음` (eum), `어` (eo), `그` (geu)
- **Polish (pl)**: `yyy`, `eee`, `no`
- **Portuguese (pt)**: `é`, `hã`, `tipo`
- **Russian (ru)**: `э-э` (eh), `ну` (nu), `как бы` (kak by)
- **Turkish (tr)**: `şey`, `yani`, `hımm`
- **Ukrainian (uk)**: `ну` (nu), `е-е` (eh)
- **Vietnamese (vi)**: `à`, `ừ`, `ừm`, `thì`
- **Chinese (zh)**: `嗯` (en), `那个` (nage)

## Decisions
- **Decision**: Hardcode these lists in `src-tauri/src/audio_toolkit/text.rs` for the initial implementation.
- **Alternatives considered**: Loading from external JSON files.
    - *Rationale*: Hardcoding is simpler, more performant, and safer for a fixed set of languages. External files introduce failure modes (missing files, parse errors).
- **Decision**: Fallback to safe defaults (empty list or strict English) for unsupported languages.
