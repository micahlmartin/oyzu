# Oyzu investor presentation

Editable, offline HTML deck: open [index.html](index.html). Arrow keys navigate; N shows presenter notes. Keep the logos and profile photo alongside the HTML.

- [Investor presentation: 35 slides, including appendix](oyzu-investor-deck.pdf)

The deck explains the opinionated, policy-governed delivery model, enterprise rollout, AI objection, evidence and compliance, competitive positioning, founding team and $1M funding plan. Current implementation evidence is distinguished from funded capabilities; illustrative economics are not measured customer results.

## Edit and rebuild

Python 3 generates the HTML with no third-party Python dependencies:

```sh
python docs/investor-deck/build_visual_deck.py
```

`build_visual_deck.py` assembles the base deck and applies the named editorial modules in order. CSS modules supply their layouts. Edit the final module owning a slide rather than an earlier version it overrides. The latest layers are `ecosystem_story.py`, `rollout_story.py`, `final_clarity.py` and `opening_pains.py`. `opening_pains.py` owns the three opening pain slides and their sequence; it owns the opening narrative. `source_cleanup.py` then keeps visible footers for citations, moves explanatory context to presenter notes, and labels material product and scenario distinctions in the slide content.

To export PDFs and check desktop/print bounds, image loading, navigation and mobile width, install Playwright and Chromium in your local tooling environment, then run:

```sh
node docs/investor-deck/render-check.cjs
```

The renderer creates screenshots and refreshes the full PDF. Rendering checks do not validate product behavior or market claims. Re-export PDFs whenever slide content changes.

Sources and claim boundaries are linked in slide footers and presenter notes. Brand attribution is recorded in [tool-logos/ATTRIBUTION.txt](tool-logos/ATTRIBUTION.txt). The founder photo comes from the public GitHub profile at https://github.com/micahlmartin.png. Logos indicate ecosystem examples, not endorsements or implemented partnerships.

`investor_polish.py` and `investor_polish.css` provide the final investor-facing editorial pass and Appendix divider. The single PDF contains 20 pitch slides, one divider, and eight appendix slides.

`competitor_comparisons.py` owns the five scenario-based appendix comparisons with Harness, GitLab, Bazel, Pants and Buck2. Official source links accompany each comparison. Implementation status remains on the working foundation slide.
