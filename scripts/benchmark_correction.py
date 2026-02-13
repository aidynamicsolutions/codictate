#!/usr/bin/env python3
"""
Benchmark correction prompts with multiple runs and baseline comparison.
Supports both 'full-sentence' (old) and 'correction-only' (new) output modes.
Dependency-free (uses urllib).
"""

import argparse
import json
import statistics
import sys
import time
import urllib.request
import urllib.error
from pathlib import Path
from collections import defaultdict

SIDECAR_URL = "http://127.0.0.1:5000"
PROMPTS_DIR = Path(__file__).parent.parent / "prompts"

TEST_CASES = [
    # ── Homophones (Unseen Context) ──
    {"context": "There is know one here", "selected": "know one", "expected": "no one", "category": "homophone"},
    {"context": "I need a pear of shoes", "selected": "pear", "expected": "pair", "category": "homophone"},
    {"context": "Your welcome to join", "selected": "Your", "expected": "You're", "category": "homophone"},
    
    # ── Contextual Restoration (Unseen Slurring/Phrasing) ──
    {"context": "I tend to wake up early", "selected": "tend to", "expected": "tend to", "category": "no-error"},
    {"context": "I ten to wake up early", "selected": "ten to", "expected": "tend to", "category": "restoration"},
    
    # ── Grammatical Context ──
    {"context": "He should of known better", "selected": "should of", "expected": "should have", "category": "grammar"},

    # ── Negative Tests ──
    {"context": "I read the book yesterday", "selected": "read", "expected": "read", "category": "no-error"},
    
    # ── Contextual Mismatch (Target Case) ──
    {"context": "When I slur I see what happens", "selected": "I see", "expected": "let's see", "category": "target"},
]

# ── Helpers ─────────────────────────────────────────────────────────────────

def load_prompt(name: str) -> str:
    path = PROMPTS_DIR / f"{name}.md"
    if not path.exists():
        raise FileNotFoundError(f"Prompt not found: {path}")
    return path.read_text()

def interpolate_prompt(template: str, output: str, context: str, selection: str) -> str:
    def json_escape_content(s):
        return json.dumps(s)[1:-1]

    safe_context = json_escape_content(context)
    safe_selection = json_escape_content(selection)
    safe_output = json_escape_content(output)

    # Load resources lazy-loaded to avoid path issues if not needed
    hints_str = ""
    try:
        slips_path = Path(__file__).parent.parent / "src-tauri/resources/phonetic_slips.json"
        homophones_path = Path(__file__).parent.parent / "src-tauri/resources/homophones.json"
        
        if slips_path.exists() and homophones_path.exists():
            slips_map = json.loads(slips_path.read_text())
            homophones_map = json.loads(homophones_path.read_text())
            
            hints_list = []
            sel_lower = selection.lower().strip()
            
            # 1. Slips
            if sel_lower in slips_map:
                candidates = slips_map[sel_lower]
                candidates_str = "', '".join(candidates)
                hints_list.append(f"- [Slip] '{selection.strip()}' is likely supposed to be '{candidates_str}'")
                
            # 2. Homophones
            if sel_lower in homophones_map:
                candidates = homophones_map[sel_lower]
                candidates_str = "', '".join(candidates)
                hints_list.append(f"- [Hint] '{selection.strip()}' might be meant as '{candidates_str}'")
                
            hints_str = "\\n".join(hints_list)
    except Exception as e:
        print(f"Warning: Could not load hints: {e}")

    return (
        template
        .replace("${output}", safe_output)
        .replace("${context}", safe_context)
        .replace("${selection}", safe_selection)
        .replace("${hints}", hints_str) 
        .replace("${dictionary}", "")
    )

def call_sidecar(prompt: str, temperature: float, top_p: float, min_p: float) -> str:
    data = {
        "prompt": prompt,
        "max_tokens": -1,
        "temperature": temperature,
        "top_p": top_p,
        "min_p": min_p,
        "system_ram_gb": 16,
    }
    
    req = urllib.request.Request(
        f"{SIDECAR_URL}/generate",
        data=json.dumps(data).encode('utf-8'),
        headers={'Content-Type': 'application/json'}
    )
    
    try:
        with urllib.request.urlopen(req, timeout=60) as response:
            return json.load(response)["response"]
    except urllib.error.URLError as e:
        raise Exception(f"Sidecar request failed: {e}")

def normalize(text: str) -> str:
    return text.strip().strip(".,!?;:").lower()

def run_benchmark(
    prompt_name: str,
    runs: int, 
    mode: str,
    temperature: float,
    top_p: float, 
    min_p: float,
    verbose: bool
):
    print(f"\n🚀 Benchmarking '{prompt_name}' (Runs: {runs}, Mode: {mode})")
    print(f"   Params: temp={temperature}, top_p={top_p}, min_p={min_p}")
    
    try:
        template = load_prompt(prompt_name)
    except FileNotFoundError:
        print(f"Skipping {prompt_name}: File not found")
        return 0.0, 0.0
    
    case_stats = defaultdict(list)
    
    total_start = time.time()
    
    for i, case in enumerate(TEST_CASES):
        context = case["context"]
        selected = case["selected"]
        expected = case["expected"]
        category = case["category"]
        
        # Prepare inputs
        use_full_context = context and selected in context
        text_for_llm = context if (use_full_context and mode == "full") else selected
        if mode == "auto":
             text_for_llm = context 
             
        prompt = interpolate_prompt(
            template, 
            output=context, 
            context=context, 
            selection=selected
        )
        
        if verbose:
            print(f"\n🔸 Case {i+1}: {selected} -> {expected}")

        for r in range(runs):
            try:
                start_time = time.time()
                raw = call_sidecar(prompt, temperature, top_p, min_p)
                end_time = time.time()
                latency = end_time - start_time
                
                cleaned = raw.strip().strip('"')
                
                # Handling Output Mode - Simplified for benchmark
                # We assume the model outputs JSON or text.
                # If JSON, we try to extract "correction" or "c"
                result = cleaned
                try:
                    # Strip markdown blocks if present
                    if "```json" in cleaned:
                        json_str = cleaned.split("```json")[1].split("```")[0].strip()
                    elif "```" in cleaned:
                        json_str = cleaned.split("```")[1].split("```")[0].strip()
                    else:
                        json_str = cleaned
                        
                    data = json.loads(json_str)
                    if isinstance(data, dict):
                        result = data.get("correction", data.get("c", cleaned))
                except json.JSONDecodeError:
                    pass # Result remains as cleaned text

                result = str(result).strip()

                # Scoring
                is_exact = normalize(result) == normalize(expected)
                is_contained = normalize(expected) in normalize(result)
                
                passed = False
                if expected == selected: # No error expected
                    passed = is_contained # If expected text is there, good.
                else:
                    passed = is_exact or is_contained
                
                case_stats[i].append((passed, result, latency))
                
                if verbose:
                    icon = "✅" if passed else "❌"
                    print(f"   Run {r+1}: {icon} '{result}' ({latency:.2f}s)")

            except Exception as e:
                print(f"   Run {r+1}: 💥 Error: {e}")
                case_stats[i].append((False, "ERROR", 0.0))

    total_time = time.time() - total_start
    
    # ── Report ──
    print(f"\n📊 Results for '{prompt_name}' ({runs} runs/case)")
    print(f"{'='*80}")
    print(f"{'#':<4} {'Category':<15} {'Case':<20} {'Pass Rate':<10} {'Avg Latency':<12} {'Result (Sample)'}")
    print(f"{'-'*80}")
    
    total_passes = 0
    total_runs_count = 0
    total_latencies = []
    
    for i, case in enumerate(TEST_CASES):
        results = case_stats[i]
        passes = sum(1 for r in results if r[0])
        total_passes += passes
        total_runs_count += len(results)
        
        case_latencies = [r[2] for r in results if len(r) > 2]
        total_latencies.extend(case_latencies)
        avg_case_latency = statistics.mean(case_latencies) if case_latencies else 0.0
        
        rate = passes / len(results) * 100
        sample = results[0][1] if results else "N/A"
        if len(sample) > 20: sample = sample[:17] + "..."
        
        print(f"{i+1:<4} {case['category']:<15} {case['selected']:<20} {rate:3.0f}%      {avg_case_latency:5.2f}s      {sample}")

    overall_accuracy = total_passes / total_runs_count * 100 if total_runs_count else 0
    overall_avg_latency = statistics.mean(total_latencies) if total_latencies else 0.0
    
    print(f"{'='*80}")
    print(f"🏆 Overall Accuracy: {overall_accuracy:.1f}%  |  ⏱️  Avg Latency: {overall_avg_latency:.2f}s  (Total Time: {total_time:.1f}s)")
    
    return overall_accuracy, overall_avg_latency

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--prompts", "-p", nargs="+", default=["correct-text-v5"], help="Prompts to test")
    parser.add_argument("--runs", "-n", type=int, default=3, help="Runs per test case")
    parser.add_argument("--mode", "-m", choices=["auto", "full", "correction"], default="correction", help="Expected output format")
    parser.add_argument("--baseline", "-b", help="Baseline prompt to compare against")
    parser.add_argument("--verbose", "-v", action="store_true")
    # Sampling
    parser.add_argument("--temp", type=float, default=0.0) # Low temp for consistency in benchmark
    parser.add_argument("--top-p", type=float, default=1.0)
    parser.add_argument("--min-p", type=float, default=0.0)
    parser.add_argument("--port", type=int, default=5337, help="Sidecar port")
    
    args = parser.parse_args()
    
    global SIDECAR_URL
    SIDECAR_URL = f"http://127.0.0.1:{args.port}"
    
    scores = {}
    latencies = {}
    
    # Run Baseline if requested
    if args.baseline:
        score, latency = run_benchmark(args.baseline, args.runs, args.mode, args.temp, args.top_p, args.min_p, args.verbose)
        scores[args.baseline] = score
        latencies[args.baseline] = latency
        
    # Run Candidates
    for p in args.prompts:
        score, latency = run_benchmark(p, args.runs, args.mode, args.temp, args.top_p, args.min_p, args.verbose)
        scores[p] = score
        latencies[p] = latency
        
    # Comparison
    if len(scores) > 1:
        print("\n⚖️  Comparison")
        sorted_scores = sorted(scores.items(), key=lambda x: x[1], reverse=True)
        print(f"{'Prompt':<25} {'Accuracy':<10} {'Avg Latency'}")
        print(f"{'-'*50}")
        for name, score in sorted_scores:
             print(f"{name:<25} {score:.1f}%      {latencies[name]:.2f}s")

if __name__ == "__main__":
    main()
