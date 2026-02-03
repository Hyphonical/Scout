
# 🎯 Optimizing Your Search Queries

Scout uses **SigLIP2**, a state-of-the-art vision-language model. Unlike traditional keyword search, SigLIP2 understands visual descriptions. To get the best results, think like a captioner, not a tagger.

## The Golden Rule: Be Descriptive

SigLIP2 performs poorly with single words like "Dog" because it doesn't know if you want a cartoon dog, a photo of a dog, or a dog-shaped cloud.

| Query Type | Weak Query (Avoid) | Strong Query (Recommended) |
| --- | --- | --- |
| **Broad Subject**      | `Dog`        | `A photo of a golden retriever playing in grass`     |
| **Specific Character** | `Snow White` | `A woman in a blue and yellow dress with black hair` |
| **Atmosphere**         | `Dark`       | `A moody, low-light interior with candle flames`     |
| **Perspective**        | `Mountain`   | `An aerial drone shot of snowy mountain peaks`       |
| **Art Style**          | `Drawing`    | `A charcoal sketch of a person's face on paper`      |

---

## Tips & Tricks

### 1. The "Photo-Caption" Template

Wrap your subject in a natural sentence. This "activates" the model's visual reasoning more effectively than a list of keywords.

* **Template:** `A [style] of [subject] [doing something] in [environment].`
* **Example:** `A cinematic wide shot of a car driving through a desert at sunset.`

### 2. Use Visual Adjectives

SigLIP2 is sensitive to textures, colors, and lighting. Use words that describe the **look** of the image:

* *Materials:* "Metallic," "wooden," "glass," "fluffy."
* *Lighting:* "Backlit," "neon," "golden hour," "high-contrast."

### 3. Solving the "Character" Problem

Models like SigLIP2 often struggle with proper names (e.g., "Mickey Mouse" or "Master Chief") unless they were heavily present in the training data. If a name fails, describe their **iconic features**:

* **Instead of:** `Spider-Man`
* **Try:** `A person in a red and blue superhero suit with a web pattern.`

### 4. Leverage Negative Prompts (`--not`)

If you are searching for real photos but keep getting 3D renders, use the `--not` flag to "push" the search away from those vectors.

```bash
scout search "forest" --not "3d render, cartoon, drawing"

```

### 5. Multilingual Capability

SigLIP2 is natively multilingual. If you are looking for specific cultural landmarks or items, try searching in the native language of that subject—it often yields more "authentic" visual matches.

### 6. Iterative Refinement

If your first search doesn't hit the mark, **adjust your phrasing, not just keywords**. Small changes can make a big difference:

* **Too broad:** `A cat` → **Better:** `A tabby cat lying on a wooden floor`
* **Wrong style:** `Person` → **Better:** `A professional headshot of a person looking at the camera`
* **Missing context:** `Street` → **Better:** `A busy city street at night with car lights and pedestrians`

Think of it as conversation with the model, be more specific, not just in *what* you're looking for, but in *how* it appears.

### 7. Action and Position Matter

Verbs and spatial descriptions significantly improve results:

* **Static:** `Dog` → **Dynamic:** `A dog jumping over a fence`
* **Vague position:** `Person with laptop` → **Specific:** `A person sitting at a desk working on a laptop`
* **Implied motion:** `Running` vs. `standing` vs. "mid-jump" all produce visually different results

### 8. Handling Ambiguity with Multiple Searches

If you're unsure about the exact look of what you're searching for, run multiple queries:

```bash
# First attempt: descriptive
scout search "beach scene with sunset" -d ~/Photos

# Second attempt: style-focused
scout search "golden hour beach photography" -d ~/Photos

# Third attempt: mood-focused
scout search "serene coastal landscape with warm lighting" -d ~/Photos
```

Compare results across queries to find what you're after.

### 9. Combining with `--not` for Refinement

Use negative prompts to eliminate common false positives:

```bash
# Looking for real photos of forests
scout search "dense forest with tall trees" --not "painting, drawing, digital art, 3d render"

# Finding action shots, not stills
scout search "person surfing on water" --not "statue, mannequin, illustration"
```

This is especially useful when a style or subject keeps appearing in unwanted results.

### 10. Search Performance Tips

* **Longer descriptions are usually better** than shorter ones—give the model more context
* **Specific numbers help:** `a group of 5 people` is more distinctive than `multiple people`
* **Compound descriptions work:** `A wooden desk with a laptop and coffee mug` is better than searching for each element separately
* **Punctuation doesn't matter**, but clarity does, natural sentences work best
* **Don't over-explain:** You don't need a paragraph, 2-3 clear, descriptive sentences are ideal