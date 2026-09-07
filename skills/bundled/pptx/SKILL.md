---
name: pptx
description: Create a PowerPoint (.pptx) deck from an outline and save it in the out-box. Use when the user wants a real PowerPoint file, not an HTML slideshow.
license: Apache-2.0
compatibility:
  python: true
  pythonImports: ["pptx"]
---

# pptx

Create a `.pptx` file on this computer with **Python** and the `python-pptx`
package. Synaplan Desktop does not install packages for you. If `import pptx`
fails, tell the user to install `python-pptx` themselves, then try again.

This skill is original to Synaplan Desktop (Apache-2.0). It is not Anthropic's
document skill.

## When to use

- The user asks for a PowerPoint deck, `.pptx`, or slides that open in
  PowerPoint / Keynote / LibreOffice Impress.
- For a browser slideshow with no extra packages, use the **slides** skill
  instead.

## How to run

```
python3 {this-folder}/run.py OUT.pptx TITLE SLIDE1_TITLE SLIDE1_BODY [SLIDE2_TITLE SLIDE2_BODY ...]
```

- `OUT.pptx` must be inside the out-box.
- Pair remaining arguments as title/body for each slide after the deck title.
- Keep slide bodies short (one or two sentences).

After the script exits 0, tell the user the full path of the file.
