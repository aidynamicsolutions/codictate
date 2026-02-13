
import json
import sys
import os
import argparse
import time
import urllib.request
import urllib.error

SIDECAR_URL = "http://127.0.0.1:5337"
HOMOPHONES_PATH = "src-tauri/resources/homophones.json"
DEFAULT_PROMPT_PATH = "prompts/correct-text-v4.md"
MODEL_PATH = "mlx-community/Qwen3-4B-4bit"

# Test Cases
TEST_CASES = [
    # Regression: "They're" (Critical)
    {"context": "I think their coming tomorrow.", "selection": "their", "expected": "they're", "type": "Regression"},
    {"context": "We think were going later.", "selection": "were", "expected": "we're", "type": "Regression"},
    {"context": "Your welcome.", "selection": "Your", "expected": "You're", "type": "Regression"},
    {"context": "Put it over their.", "selection": "their", "expected": "there", "type": "Regression"},

    # Generalization: Unseen Homophones
    {"context": "Turn write at the light.", "selection": "write", "expected": "right", "type": "Unseen"},
    {"context": "Give me a peace of cake.", "selection": "peace", "expected": "piece", "type": "Unseen"},
    {"context": "I need to add it up.", "selection": "ad", "expected": "add", "type": "Unseen"},
    {"context": "Take a brake from work.", "selection": "brake", "expected": "break", "type": "Unseen"},
    {"context": "Baking with flower.", "selection": "flower", "expected": "flour", "type": "Unseen"},
    {"context": "Come over hear.", "selection": "hear", "expected": "here", "type": "Unseen"},
    {"context": "By the way.", "selection": "buy", "expected": "by", "type": "Unseen"},
    {"context": "The wind blue hard.", "selection": "blue", "expected": "blew", "type": "Unseen"},
    
    # Control: Do not correct
    {"context": "It was a black hole.", "selection": "hole", "expected": "hole", "type": "Control"},
    {"context": "World peace is good.", "selection": "peace", "expected": "peace", "type": "Control"},
]

def load_homophones():
    if os.path.exists(HOMOPHONES_PATH):
        path = HOMOPHONES_PATH
    else:
        path = os.path.join(os.path.dirname(os.path.dirname(__file__)), HOMOPHONES_PATH)
    with open(path, "r") as f:
        return json.load(f)

def generate_hints(selection, homophones_map):
    selection_lower = selection.lower().strip()
    if selection_lower in homophones_map:
        candidates = homophones_map[selection_lower]
        candidates_str = "', '".join(candidates)
        return f"- (Visual Hint) '{selection.strip()}' sounds identical to '{candidates_str}'"
    return ""

def interpolate_prompt(template, context, selection, hints):
    # Determine the placeholders used in the template
    # v4 uses ${hints}
    # Some older prompts might use ${dictionary}
    # We'll just replace both to be safe
    
    def json_escape_content(s):
        # Escape string for JSON inclusion, strip outer quotes
        return json.dumps(s)[1:-1]

    safe_context = json_escape_content(context)
    safe_selection = json_escape_content(selection)
    
    prompt = template.replace("${context}", safe_context)
    prompt = prompt.replace("${selection}", safe_selection)
    prompt = prompt.replace("${hints}", hints)
    prompt = prompt.replace("${dictionary}", hints)
    
    return prompt

def post_request(url, data):
    req = urllib.request.Request(
        url,
        data=json.dumps(data).encode('utf-8'),
        headers={'Content-Type': 'application/json'}
    )
    with urllib.request.urlopen(req, timeout=30) as response:
        return json.load(response)

def ensure_model_loaded():
    try:
        # Check status
        status_url = f"{SIDECAR_URL}/status"
        with urllib.request.urlopen(status_url, timeout=5) as response:
            status = json.load(response)
            
        if status.get("model_loaded"):
            print(f"Model already loaded: {status.get('model_path')}")
            return True
            
        # Load model
        print(f"Loading model: {MODEL_PATH} ...")
        load_url = f"{SIDECAR_URL}/load"
        post_request(load_url, {"model_path": MODEL_PATH})
        print("Model loaded successfully.")
        return True
        
    except Exception as e:
        print(f"[ERROR] Failed to check/load model: {e}")
        return False

def run_tests(prompt_path=None, prompt_content=None, verbose=False):
    if not ensure_model_loaded():
        return 0, len(TEST_CASES)

    if prompt_content:
        template = prompt_content
    else:
        path = prompt_path or DEFAULT_PROMPT_PATH
        if not os.path.exists(path):
             path = os.path.join(os.path.dirname(os.path.dirname(__file__)), path)
        with open(path, "r") as f:
            template = f.read()

    homophones_map = load_homophones()
    
    passes = 0
    total = len(TEST_CASES)
    
    print(f"Running {total} tests...")
    
    start_time = time.time()
    
    for case in TEST_CASES:
        hints = generate_hints(case["selection"], homophones_map)
        prompt = interpolate_prompt(template, case["context"], case["selection"], hints)
        
        try:
            data = post_request(
                f"{SIDECAR_URL}/generate",
                {
                    "prompt": prompt,
                    "max_tokens": -1, # Auto-calculate
                    "temperature": 0.0,
                    "top_p": 1.0, 
                    "min_p": 0.0,
                }
            )
            raw_response = data["response"]
            
            # Helper to extract JSON if embedded in markdown
            text = raw_response.strip()
            if "```json" in text:
                text = text.split("```json")[1].split("```")[0].strip()
            elif "```" in text:
                text = text.split("```")[1].split("```")[0].strip()
            
            correction = ""
            thought = ""
            
            # Heuristic JSON parsing
            try:
                # Try standard JSON
                parsed = json.loads(text)
                if isinstance(parsed, dict):
                    correction = parsed.get("correction", "")
                    thought = parsed.get("thought", "")
                    if not correction and "correction" not in parsed:
                        # Maybe keys are shortened? or just 'response'?
                        pass
                else:
                    # Maybe just a string?
                    pass
            except json.JSONDecodeError:
                # If JSON fails, maybe the model output raw string or something else
                # Check if we are testing short keys v/s normal keys
                pass

            # Correction might be missing if key names changed in experiment
            # But the script should assume standard output unless specified otherwise.
            # However, for shortening keys experiment, I might need to adapt the script.
            # I'll add logic to check for "c" if "correction" is missing, etc.
            if not correction:
                if isinstance(parsed, dict):
                     correction = parsed.get("c", "") # Short key support
                     thought = parsed.get("t", "")

            # If still invalid, perform manual extraction if simple format
            if not correction and not isinstance(parsed, dict):
                 # Fallback/Error
                 pass

            correction = str(correction).strip()
            
            is_pass = correction.lower() == case["expected"].lower()
            
            if is_pass:
                passes += 1
                status = "PASS"
            else:
                status = "FAIL"
            
            if verbose or not is_pass:
                print(f"[{status}] {case['type']} | '{case['context']}' -> '{correction}' (Exp: '{case['expected']}')")
                if not is_pass:
                    print(f"  Thought: {thought}")
                    print(f"  Hints: {hints}")

        except Exception as e:
            print(f"[ERROR] Request Failed | '{case['context']}' : {e}")

    duration = time.time() - start_time
    print("-" * 40)
    print(f"Results: {passes}/{total} Passed ({passes/total*100:.1f}%)")
    print(f"Time: {duration:.2f}s")
    
    return passes

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--prompt", help="Path to prompt file")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()
    
    run_tests(args.prompt, verbose=args.verbose)
