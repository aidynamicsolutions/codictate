# Prompt Engineering for Small LLMs (Lessons Learned)

This checklist summarizes key strategies for optimizing prompts for small language models (e.g., Qwen 4B, Llama 3B), based on extensive iteration on the ASR Correction task.

## ✅ Format & Structure
- [ ] **Use Strict JSON:** Small models adhere much better to JSON schemas than XML or natural language instructions. It prevents rambling and enforces structure.
- [ ] **Avoid Conversational Preambles:** Don't use "You are a helpful assistant." Use "You are a JSON-speaking tool." or "System: Correction Engine."
- [ ] **Data > Instructions:** Provide clear input/output schemas. The model mimics the data format better than it follows abstract rules.

## 🧠 Reasoning & Chain of Thought (CoT)
- [ ] **Force "Thinking" First:** Include a `"thought"` or `"reasoning"` field in the JSON output *before* the answer.
- [ ] **Solve Ambiguity:** Small models struggle with homophones ("know one" vs "no one") without explicit reasoning. CoT forces them to analyze context.
- [ ] **Break High-Frequency Bias:** Use counter-intuitive examples (e.g., "I know one" -> "no one") to force the model to prioritize context over literal validity for common phrases.

## 🚫 Constraints & Safety
- [ ] **Avoid "Don't" Rules:** Negative constraints (e.g., "Don't hallucinate") are often ignored.
- [ ] **Use Positive Examples:** Show, don't tell. Provide few-shot examples of *preserving* text to prevent over-correction.
- [ ] **Don't Over-Constrain:** Explicit "System 2" checks (e.g., "Check if sentence starter") can make small models **too conservative**, causing them to miss valid corrections.
- [ ] **Target Specific Failures:** If the model hallucinates on "Thank", add a specific example: `{"input": "Thank", "output": "Thank"}`.

## 🔬 Testing & Iteration
- [ ] **Benchmark Unseen Data:** Prompts often overfit to provided examples. Always test on a holdout set.
- [ ] **Measure Latency:** "Smarter" prompts are slower. measure average tokens/sec and total time.
- [ ] **Check for Regression:** Improvements in one area (context) often break another (basic spelling). Re-run regression tests on every change.
