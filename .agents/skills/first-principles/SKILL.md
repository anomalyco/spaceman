---
name: first-principles
description: Break through a "can't" by reasoning from first principles. Use when a tool, library, model or API seems to block the goal ("X doesn't support Y", "no way to", "not possible"), when about to settle for a workaround, or when the user says "first principles", "out of the box" or "re-implement it".
---

# First principles

A "can't" is a hypothesis, not a fact. Before accepting a limitation, find the layer where it actually lives, then decide from evidence whether to work within it, go under it, or rebuild that layer.

Reference case (Hex, Oct 2026): "Parakeet has no custom vocabulary" looked final. Reading NVIDIA's phrase-boosting paper and transcribe.cpp's decoder showed greedy decode ends in a plain argmax on the host, so a gated logit boost for user terms was a few hundred lines with no retraining.

## 1. State the limitation precisely

Write one line: what we want, what blocks it, and who said it is blocked (docs, an error, an agent's assumption, folklore). Done when the blocker is a specific claim someone could check.

## 2. Research both ends of the timeline

Spawn 2–3 fast research subagents, one per angle:
- **Primary sources:** the actual source code, papers (recent and decades-old), specs, release notes.
- **Prior art:** how competitors and other projects solved it, and what users ask for in issues.
- **The mechanism:** trace the data path to the exact line where the limitation is enforced.

Done when you can point to the file:line, paper section or spec clause where the limit lives, or show that it doesn't exist.

## 3. Find the seam

Ask, in order:
1. Is the limit real, or only in the wrapper or API surface we use?
2. Is there a lower layer we control (decoder loop, raw buffers, protocol, build step) where the goal becomes easy?
3. Would a small purpose-built implementation beat the general one? Name what ours would do differently and the size of a first version. Agents make rewrites cheap; a from-scratch layer is a real option.

Done when you have 2–3 candidate approaches, including at least one that goes under or replaces the blocking layer.

## 4. Plan from evidence

Hand the research to one strong planning agent to synthesize a phased plan with a **measurable gate**: an eval, benchmark or test that decides go/no-go (use `autoresearch` for measure-change-measure loops). Start with a short spike that proves or kills the riskiest assumption.

Done when the plan names the gate, the spike and the evidence behind the recommendation.

## 5. Recommend

Tell the user: the limitation, where it really lives, the recommended approach and why the evidence supports it, and the gate. Build the spike when the change is in your lane; bring anything outward or costly to the user first.
