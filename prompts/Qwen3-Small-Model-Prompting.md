# Qwen3 Small Model Prompting Guide (4B, 4-bit)

Best practices for prompting small/quantized Qwen3 models in Handy.

---

## Core Principles

| Do | Don't |
|----|-------|
| Use numbered lists | Use role-playing preambles |
| Add inline examples per rule | Use 6-10 few-shot examples |
| Use pattern-based rules | Use abstract/semantic rules |
| Add explicit "don't" constraints | Assume the model will preserve content |
| Keep prompts < 500 tokens | Use extended context features |

---

## Prompt Structure Template

```
[Direct task instruction]:
1. [Concrete rule] (example → result)
2. [Concrete rule] (example → result)
3. [Concrete rule] (example → result)

[Explicit negative constraint]

[Output instruction]

[Input label]:
${input}
```

---

## Rules for Avoiding Hallucination

### ❌ Abstract Rules (Avoid)
```
Remove technical glitches
Clean up noise
Fix formatting issues
```

### ✅ Pattern-Based Rules (Use)
```
Remove repeated single letters/words (c c c → c, the the → the)
Remove filler words (um, uh, like)
Convert number words to digits (twenty → 20)
```

---

## Sampling Parameters

| Parameter | Value | Notes |
|-----------|-------|-------|
| Temperature | 0.7 | Don't go lower; causes repetition |
| TopP | 0.8 | Nucleus sampling threshold |
| MinP | 0.05 | Filters low-probability tokens; reduces hallucination |
| Repetition Penalty | 1.15 | Prevents repetition loops |

> **Note**: `top_k` is NOT used — `min_p` is more effective for small models because it adapts dynamically to model confidence.

⚠️ **Avoid greedy decoding** (temp=0) — causes endless repetition loops.

---

## Mode Selection

| Task Type | Mode | Notes |
|-----------|------|-------|
| Text cleaning/editing | Non-thinking | Skip `<think>` overhead |
| Simple formatting | Non-thinking | Faster, more predictable |
| Math/logic problems | Thinking | Use sparingly; adds latency |

### How Handy Disables Thinking Mode

Handy uses the **programmatic approach** via the chat template API:

```python
# In server.py
tokenizer.apply_chat_template(
    messages,
    enable_thinking=False,  # Disables verbose reasoning
    ...
)
```

This is more reliable than the prompt-based `/no_think` directive because:
- It's enforced at the tokenizer level
- No risk of the model ignoring the directive
- Cleaner prompts without extra instructions

> **Alternative**: Add `/no_think` to the prompt text if programmatic control isn't available.

---

## Working Example: Transcription Cleaning

```
Clean this transcript:
1. Fix spelling, capitalization, and punctuation errors
2. Convert number words to digits (twenty-five → 25, ten percent → 10%)
3. Replace spoken punctuation with symbols (period → ., comma → ,)
4. Remove filler words (um, uh, like as filler)
5. Remove repeated single letters/words in sequence (c c c → c, the the → the)
6. Keep the language in the original version

Preserve exact meaning and word order. Do not paraphrase or reorder content.

Return only the cleaned transcript.

Transcript:
${output}
```

---

## Common Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| Content removed incorrectly | Abstract rule interpreted too broadly | Use pattern-based rule with inline example |
| Output paraphrased | Missing negative constraint | Add "Do not paraphrase or reorder" |
| Metadata treated as content | Role-playing confused the model | Remove preambles; use direct instructions |
| Repetition loops | Greedy decoding or low temperature | Use temp=0.7, avoid temp=0 |

---

## User Role vs System Role

Qwen3 uses the **ChatML format** with special tokens to structure messages:

```
<|im_start|>system
[system instructions here]<|im_end|>
<|im_start|>user
[user message here]<|im_end|>
<|im_start|>assistant
[model response]<|im_end|>
```

### Why Use User Role for Small Models?

| System Role | User Role |
|-------------|-----------|
| Adds cognitive overhead | Direct and simple |
| Model may ignore or misinterpret | More reliably followed |
| Adds extra tokens | Fewer tokens = more context for input |

### ❌ Avoid: System Role with Preamble

```python
messages = [
    {"role": "system", "content": "You are an expert transcription editor..."},
    {"role": "user", "content": "Voice to text recognition speech app."}
]
```
**Problem**: Small models may confuse the system instructions with the content, or ignore them entirely.

### ✅ Use: User Role with Direct Task

```python
messages = [
    {"role": "user", "content": """Clean this transcript:
1. Fix spelling and punctuation
2. Remove filler words (um, uh)

Return only the cleaned text.

Transcript:
Voice to text recognition speech app."""}
]
```
**Better**: Everything in one user message. Task is clear, no role-playing needed.

### How Handy Formats Messages

```python
# In server.py
messages = [{"role": "user", "content": prompt}]
formatted = tokenizer.apply_chat_template(
    messages,
    enable_thinking=False,
    add_generation_prompt=True
)
```

This produces:
```
<|im_start|>user
Clean this transcript:
1. Fix spelling...
...<|im_end|>
<|im_start|>assistant
```

---

## Quick Checklist

Before deploying a prompt for 4B models:

- [ ] No role-playing preamble ("You are an expert...")
- [ ] All rules have inline examples
- [ ] No abstract/semantic rules
- [ ] Explicit "Do not..." constraint included
- [ ] Prompt under 500 tokens
- [ ] Using user role, not system role