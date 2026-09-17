//! UC-08 notes — the local half of the product. Runs against a temporary
//! `ProjectStore` exactly like the Tauri notes commands: create, write,
//! read back, list, search, append a dictated sentence, delete, refuse bad
//! names. No network; a red row here is a data-loss bug.

use synaplan_core::projects::notes::is_valid_note_name;
use synaplan_core::projects::{PersonalSeed, ProjectStore};

use crate::report::{Case, CaseResult};
use crate::Ctx;

pub async fn uc08_notes(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-08", "Write, find and delete notes (local)");
    let root = ctx.work_dir.join("uc08");
    let store = ProjectStore::with_roots(
        root.join("config").join("projects"),
        root.join("Synaplan").join("projects"),
    );
    if let Err(e) = store.ensure_personal(&PersonalSeed::default()) {
        case.check("Personal project exists", false, e.to_string());
        return case.finish("store unusable");
    }
    let project = match store.create_project("Use-case notes", "en", None) {
        Ok(p) => p,
        Err(e) => {
            case.check("project created", false, e.to_string());
            return case.finish("store unusable");
        }
    };
    case.check("project created", true, format!("slug {}", project.slug));

    let note = match store.create_note(&project.id) {
        Ok(n) => n,
        Err(e) => {
            case.check("note created", false, e.to_string());
            return case.finish("cannot create notes");
        }
    };
    case.check(
        "note created with a timestamp name",
        note.name.ends_with(".md") && note.content.is_empty(),
        note.name.clone(),
    );

    let body = "# Aurora kick-off\n\n- Bremen ships end of month\n- Malin sends the forecast\n\n**Decision:** review every two weeks.\n";
    let written = store.write_note(&project.id, &note.name, body);
    case.check(
        "markdown written",
        written
            .as_ref()
            .map(|s| s.size == body.len() as u64)
            .unwrap_or(false),
        written
            .as_ref()
            .map(|s| format!("title \"{}\", {} bytes", s.title, s.size))
            .unwrap_or_else(|e| e.to_string()),
    );
    let back = store.read_note(&project.id, &note.name);
    case.check(
        "read back byte-identical",
        back.as_ref().map(|n| n.content == body).unwrap_or(false),
        back.as_ref()
            .map(|n| format!("{} chars, title \"{}\"", n.content.chars().count(), n.title))
            .unwrap_or_else(|e| e.to_string()),
    );

    // A dictated sentence lands at the end, like the caret insert.
    let dictated = "Please remind me to book the room and to invite the supplier from Kiel.";
    let appended = format!("{body}\n{dictated}\n");
    let _ = store.write_note(&project.id, &note.name, &appended);
    let after = store
        .read_note(&project.id, &note.name)
        .map(|n| n.content)
        .unwrap_or_default();
    case.check(
        "dictated sentence appended without losing text",
        after.starts_with(body) && after.contains(dictated),
        format!("{} chars", after.chars().count()),
    );

    let listed = store.list_notes(&project.id).unwrap_or_default();
    case.check(
        "note appears in the list with its title",
        listed
            .iter()
            .any(|n| n.name == note.name && n.title == "Aurora kick-off"),
        format!("{} note(s) listed", listed.len()),
    );
    let found = store
        .search_notes(&project.id, "supplier from kiel")
        .unwrap_or_default();
    case.check(
        "full-text search finds the dictated words",
        found.iter().any(|n| n.name == note.name),
        format!("{} hit(s)", found.len()),
    );
    let none = store
        .search_notes(&project.id, "zzzz-not-there")
        .unwrap_or_default();
    case.check(
        "search for absent text is empty",
        none.is_empty(),
        format!("{} hit(s)", none.len()),
    );

    for bad in [
        "../escape.md",
        "notes/inner.md",
        "",
        "no-extension",
        "a\\b.md",
    ] {
        case.check(
            format!("name {bad:?} refused"),
            !is_valid_note_name(bad) && store.read_note(&project.id, bad).is_err(),
            "",
        );
    }

    let deleted = store.delete_note(&project.id, &note.name);
    let gone = store.read_note(&project.id, &note.name).is_err();
    case.check(
        "delete removes the file",
        deleted.is_ok() && gone,
        deleted
            .err()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "gone".into()),
    );
    let on_disk = store.notes_dir(&project);
    case.check(
        "notes live under the project folder on disk",
        on_disk.starts_with(store.projects_dir()),
        on_disk.display().to_string(),
    );
    case.finish("create → write → read → search → append → delete")
}
