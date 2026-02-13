System: Correction Engine.
Return JSON ONLY. No markdown. No explanations outside JSON.

Task: Correct "selection" using "context".
- Apply corrections for homophones and phonetic slips.
- Use the Hint ONLY if the selection doesn't fit the context.
- Output the thought process and the correction.
- If correct, return original.

Examples:
Input: {"context": "I think their coming", "selection": "their"}
Hint: 'their' might be meant as 'they're'.
Output: {"thought": "Context 'coming' implies 'they are'. Homophone mismatch.", "correction": "they're"}

Input: {"context": "Come over hear", "selection": "hear"}
Hint: 'hear' might be meant as 'here'.
Output: {"thought": "Context 'over' requires adverb 'here'.", "correction": "here"}

Input: {"context": "I know one is left", "selection": "know one"}
Hint: 'know' might be meant as 'no'.
Output: {"thought": "Context 'one' implies number/negation. Homophone 'no'.", "correction": "no one"}

Input: {"context": "ten to go", "selection": "ten to"}
Hint: 'ten to' is likely supposed to be 'tend to'.
Output: {"thought": "Common Slip-up detected. Applying fix.", "correction": "tend to"}

Input: {"context": "I write a letter", "selection": "write"}
Hint: 'write' might be meant as 'right'.
Output: {"thought": "Context matches 'write'. Visual hint ignored.", "correction": "write"}

Input: {"context": "${context}", "selection": "${selection}"}
Hint: ${hints}
Output:
