# UX and five-locale comprehension

**Status:** Draft 2026-09-10. Copy is reviewed **before** the Vue is
built (`PC4` depends on this table).
**Depends on:** checklist rows 3, 8, 20, 26–27; [`12_ux_and_i18n.md`](../20260829-desktop-agent-client/12_ux_and_i18n.md)
still applies (forbidden words, platform-native paths, five questions).
**Unlocks:** `PC4` shell, every later view.
**Repos:** `synaplan-desktop` (`src/i18n/{en,de,es,fr,tr}.json`).
**Done:** a non-technical user can switch projects and explain “this
project's models” in their language.

---

## 0. Why this file exists

The 2026-08-29 UI is a **computer** app: Chat, Skills, This computer,
Check this computer. This epic is a **project** app. If we keep the
old rail and tuck projects into a dropdown, people will keep chatting
in the wrong world.

This is also the first desktop surface that talks about **where data
is processed**. Jargon will make them click through. The Models panel
must pass the comprehension gate in §6.

---

## 1. The five questions (updated)

The 2026-08-29 five questions still hold for pairing and skills. Every
**project** screen must also answer:

| # | Question | Where |
| - | -------- | ----- |
| 1 | **Which project am I in?** | Switcher (always visible) |
| 2 | **Which models process this project's data?** | Models + a one-line summary on Chat |
| 3 | **What stays on this computer?** | Notes empty state; delete-project copy |
| 4 | **What is sent to Synaplan?** | Files empty state; share-note confirm |
| 5 | **How do I stop a model / Assistant / Skill?** | Models unset; Agents unbind; skill toggle; revoke still on the web |

If a design needs a sixth primary rail item beyond Chat / Notes /
Files / Agents / Models, cut scope.

---

## 2. Canonical terminology — all five locales

Proposed translations for native-speaker review (row `L1` in the
breakdown). Do not treat DE/ES/FR/TR as final until that review.

| Concept | EN | DE | ES | FR | TR |
| ------- | -- | -- | -- | -- | -- |
| Local unit | **Project** | Projekt | Proyecto | Projet | Proje |
| Built-in first project | **Personal** | Persönlich | Personal | Personnel | Kişisel |
| Server recipe | **Assistant** | Assistent | Asistente | Assistant | Asistan |
| Local `SKILL.md` | **Skill** | Skill | Skill | Skill | Skill |
| Model matrix | **This project's models** | Modelle dieses Projekts | Modelos de este proyecto | Modèles de ce projet | Bu projenin modelleri |
| World chip | **Stay in this world** | In dieser Welt bleiben | Seguir en este mundo | Rester dans ce monde | Bu dünyada kal |
| Sovereignty sentence (short) | Data is processed only by the models you picked for this project. | Daten werden nur von den Modellen verarbeitet, die du für dieses Projekt gewählt hast. | Los datos solo los procesan los modelos que elegiste para este proyecto. | Les données ne sont traitées que par les modèles que vous avez choisis pour ce projet. | Veriler yalnızca bu proje için seçtiğiniz modellerle işlenir. |
| Notes | **Notes** | Notizen | Notas | Notes | Notlar |
| Dictation | **Dictation** | Diktat | Dictado | Dictée | Dikte |
| Dictation language | **Dictation language** | Diktatsprache | Idioma del dictado | Langue de dictée | Dikte dili |
| Knowledge folder | **Knowledge folder** | Wissensordner | Carpeta de conocimiento | Dossier de connaissances | Bilgi klasörü |
| Index files (EMBED) | **Index files** | Dateien indexieren | Indexar archivos | Indexer les fichiers | Dosyaları dizinle |
| Chat slot | **Chat** | Chat | Chat | Chat | Sohbet |
| Voice slot | **Dictation** | Diktat | Dictado | Dictée | Dikte |
| Speak slot | **Read aloud** | Vorlesen | Lectura en voz alta | Lecture à voix haute | Sesli oku |
| Vision slot | **Images (understand)** | Bilder (verstehen) | Imágenes (entender) | Images (comprendre) | Görseller (anlama) |
| Image slot | **Images (create)** | Bilder (erstellen) | Imágenes (crear) | Images (créer) | Görseller (oluşturma) |
| Video slot | **Video** | Video | Vídeo | Vidéo | Video |
| Docs slot | **Documents** | Dokumente | Documentos | Documents | Belgeler |
| Agents view | **Agents** | Agenten | Agentes | Agents | Ajanlar |
| In board | **In** | Eingang | Entrada | Entrée | Gelen |
| Out board | **Out** | Ausgang | Salida | Sortie | Giden |
| Overflow computer | **This computer** | Dieser Computer | Este equipo | Cet ordinateur | Bu bilgisayar |
| Overflow doctor | **Check this computer** | Computer prüfen | Comprobar este equipo | Vérifier cet ordinateur | Bu bilgisayarı denetle |

**Skill** stays untranslated (loanword). Same reason as 2026-08-29.

**Assistant** is the web product word. Do not say Agent in the UI
except the **Agents** view title (the board that holds both
Assistants and Skills). If that collides in a locale, prefer
“Assistants & Skills” over “Agents” — decide in `L1`, one term
everywhere.

### 2.1 Words that must never appear in primary UI copy

From 2026-08-29, still forbidden:

`Claude`, `Anthropic`, `MCP`, `shell`, `Bash`, `tool_use`, `sk_`,
`lease`, `Tauri`, `pairing token`, `brogent`.

**New forbidden** (this epic):

`DEFAULTMODEL`, `BTAG`, `BMODELS`, `capability enum`, `capability`,
`protocol`, `group_key`, `VECTORIZE`, `SOUND2TEXT`, `PIC2TEXT`,
`TEXT2PIC`, `TEXT2VID`, `TEXT2SOUND`, `ANALYZE`, `BID`,
`service:providerId:tag` (engineers' key — show `providerId`).

Secondary/docs/admin may use precise terms. The Models panel may not.

### 2.2 Terms that already exist — do not invent synonyms

| Existing | Keep |
| -------- | ---- |
| Synaplan Desktop | App name, all locales |
| Pair this computer / Disconnect | Unchanged |
| Check this computer | Unchanged |
| Skill / Enable / Install skill | Unchanged |
| Folder this app may use | Unchanged |
| Knowledge folder | **New** — do not say Sources, group, or RAG folder in the UI |

---

## 3. Shell (`PC4`)

Replace the four-item global nav in `AppSidebar.vue` / `ui.ts`.

```text
┌ View = 'chat' | 'notes' | 'files' | 'agents' | 'models'
│         + overflow 'computer' | 'doctor' | 'skills-install'
```

**Left rail**

1. Project switcher (name + chevron). Menu: list, create, rename,
   delete.
2. Chat
3. Notes
4. Files
5. Agents
6. Models

**Footer / overflow** (machine-level)

- This computer
- Check this computer
- Install skills (computer-level) — may live inside This computer
  instead of a rail item
- Documentation
- Connection / last check-in
- Sign out

Pairing remains the only screen when unpaired (`App.vue`).

Empty first-run (migration already created Personal): land on Chat
inside Personal. Do not force the create dialog.

Create-project dialog: name + dictation language. Models can wait.

Delete: `useDialog` danger confirm. Copy from
[`01_product_model.md`](./01_product_model.md) §5 (notes vs Synaplan
files).

### 3.1 Chat

Per-project thread list + transcript + composer.

Composer: project CHAT model is **not** a free global picker of every
`/v1/models` id. It shows the project Chat model (link to Models).
v1: no per-turn override (sovereign §10).

Mic uses project VOICE + language.

Default Assistant chip if bound; world warning if mismatch.

### 3.2 Notes / Files / Agents / Models

See [`06`](./06_notes_and_dictation.md), [`07`](./07_files_and_io.md),
[`08`](./08_assistants_and_skills.md), [`02`](./02_sovereign_models.md).

### 3.3 Design

Evolve the current tokenized CSS (light + dark). No one-off palette.
Buttons: house colour **and** shape (`px-4 py-2.5 rounded-lg` or the
desktop equivalent already in this repo — **match nearby buttons**).
Fields: full house chain; never a borderless preflight box.

Platform-native paths. Placeholders `{name}`, `{path}`, `{project}`.

---

## 4. Honesty rules

- Do not say files “stay on this computer” if they were uploaded.
- Do not say models “run on this computer” — they run on the paired
  workspace / its providers. The sovereignty sentence already says so.
- Do not say “encrypted” or “air-gapped”.
- Do not promise SPEAK or VIDEO works if the product does not use
  them yet — picker visible, helper text “Not used yet” / “Optional”.
- Assistants off: “Assistants are turned off on this workspace.”
  Not a dead dropdown of 500 errors.
- Catalog fetch fail: keep last picks; banner; no invented models.

---

## 5. Locale process

1. **`L1`** — native-speaker pass on §2 before `PC4` copy freezes.
2. Every string PR updates **all five** files: `en`, `de`, `es`,
   `fr`, `tr`.
3. Existing locale-parity test must stay green. New keys are added
   to all five in the same change — no ledger exceptions for new
   work.
4. Placeholder names must match English.

---

## 6. Comprehension gate (manual, before `PC6` is “done”)

Give a non-engineer the Models panel + Files empty state in DE or
ES. They must answer:

1. What is a project?
2. Which models will read a file I add here?
3. Do my notes leave this computer if I do not share them?
4. If an Assistant “uses other models”, what happens?

If they cannot, fix copy, not the user.

---

## 7. Non-goals

- Redesigning pairing.
- Embedding the web Assistants builder.
- A sixth primary rail item.
- English-only placeholders “for now”.
