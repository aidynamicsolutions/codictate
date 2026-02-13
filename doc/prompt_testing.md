# AI Prompt Testing Documentation

This document outlines the procedure for testing and benchmarking AI correction prompts in Handy. The goal is to ensure high accuracy in correcting ASR errors while minimizing latency and token usage.

## Objective
- **Accuracy:** >90% on benchmark test set.
- **Latency:** Minimized (target < 2s).
- **Token Usage:** Minimized (target < 400 chars).
- **Stability:** Consistent results across multiple runs.

## Tools
Two primary scripts are used for testing:

1.  **`scripts/optimize_prompt.py`**:
    - Used for rapid iteration during prompt development.
    - Tests a single prompt against a subset of critical test cases.
    - Good for checking if a change breaks fundamental behavior.

2.  **`scripts/benchmark_correction.py`**:
    - Used for final validation and comparison.
    - Runs a comprehensive test suite (homophones, grammar, hallucinations) multiple times.
    - Compares a "Candidate" prompt against a "Baseline" prompt.
    - Calculates accuracy, average latency, and pass rates per category.

## Methodology

### 1. Establish a Baseline
Before making changes, establish the performance of the current production prompt.
```bash
# Run baseline benchmark
uv run scripts/benchmark_correction.py --prompts correct-text-v5 --runs 10
```

### 2. Develop Candidates
Create new prompt files in `prompts/experiments/`. Use `optimize_prompt.py` to quick-test them.
```bash
# Quick test a candidate
uv run scripts/optimize_prompt.py --prompt prompts/experiments/v6_candidate.md
```

### 3. Comparative Benchmarking
Run a side-by-side comparison of the candidate against the baseline using optimal sampling parameters.

**Sampling Parameters:**
- **Temperature:** `0.0` (Greedy) - Critical for stability and reproducibility.
- **Top P:** `1.0`
- **Min P:** `0.0` (Disabled)

```bash
uv run scripts/benchmark_correction.py \
  --prompts correct-text-v6 \
  --baseline correct-text-v5 \
  --runs 20 \
  --temp 0.0 \
  --top-p 1.0 \
  --min-p 0.0
```

### 4. Analyze Results
Check the output for:
- **Accuracy Delta:** Is the candidate better or equal?
- **Latency Delta:** Is the candidate faster?
- **Failure Cases:** Did it regress on specific categories (e.g., "their" -> "they're")?

## Key Test Cases
The benchmark suite covers:
- **Homophones:** `their/they're`, `hear/here`, `know/no`.
- **Grammar/Spelling:** `cant` -> `can't`, `should of` -> `should have`.
- **Hallucinations:** Ensuring the model doesn't hallucinate new text or over-correct valid text.
- **Selection Boundaries:** Correcting only the selected text within its context.

## Integration
Once a candidate passes benchmarking:
1.  Save it as `prompts/correct-text-v{N}.md`.
2.  Update `src-tauri/src/managers/correction.rs` to point to the new file.
3.  Update logic if necessary (e.g., enforcing `temp=0.0`).
