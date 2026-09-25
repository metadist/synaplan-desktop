// A browser-side stand-in for the Rust side of Synaplan Desktop, used ONLY by
// the recorded use-case demos (tests/demo). It implements the same
// `window.__TAURI_INTERNALS__` surface `@tauri-apps/api` talks to — `invoke`,
// `transformCallback`, the event plugin — so the real Vue app runs unchanged in
// a normal browser and every command answers from a scripted scenario.
//
// Nothing here ships. It is injected by Playwright before the page loads and
// reads its scenario from `window.__SYNAPLAN_DEMO__` (see scenarios/types.ts).
// No key, no network: the "assistant" replies come from the scenario script.
;(() => {
  const scenario = window.__SYNAPLAN_DEMO__
  if (!scenario) {
    throw new Error('demo: window.__SYNAPLAN_DEMO__ is missing')
  }

  const SKILL_CATALOG = {
    'csv-insights': 'Markdown profile of a CSV: rows, fill rates, numeric and category summaries.',
    'email-draft': 'Unsent .eml draft for Outlook, Apple Mail, or Thunderbird.',
    'web-report': 'Standalone HTML page from Markdown or plain text.',
    slides: 'Self-contained HTML presentation (arrow keys; no PowerPoint).',
    chart: 'Bar or line chart from a CSV, as SVG inside a standalone HTML page.',
    'data-table': 'Searchable, sortable HTML table from a CSV.',
    'calendar-event': 'An .ics invite for Outlook, Apple Calendar, or Google Calendar.',
    vcard: 'A .vcf contact card for any address book.',
    'json-csv': 'JSON array of objects ⇄ CSV.',
    invoice: 'Print-ready HTML invoice from a JSON spec.',
    'hello-files': 'Tiny example that writes hello.txt into the out-box.',
    pptx: 'PowerPoint deck: title slide, bullets, pictures, native charts and tables.',
    docx: 'Markdown → real .docx (headings, lists, tables, charts) and .docx → Markdown.',
    xlsx: 'JSON/CSV → real .xlsx (sheets, styled header, formulas, totals) and back.',
  }

  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
  const nowIso = () => new Date().toISOString()
  const sep = scenario.paths.sep || '/'
  const join = (...parts) => parts.join(sep)
  const baseName = (path) => path.split(/[\\/]/).pop() || path

  // ---- state ---------------------------------------------------------------
  const projects = [scenario.personal, scenario.project].map((p) => ({
    ...p,
    kind: p.kind || 'project',
    createdAt: p.createdAt || nowIso(),
    updatedAt: p.updatedAt || nowIso(),
    dictationLanguage: p.dictationLanguage || scenario.language,
    defaultAssistantId: null,
    assistantIds: [],
    enabledSkills: p.enabledSkills || [],
    models: p.models,
    knowledgeFolder: `DESKTOP:${p.id}`,
    webSearch: p.webSearch === true,
  }))
  let activeId = scenario.project.id
  const personalId = scenario.personal.id

  const chats = new Map() // projectId -> ChatThread[]
  const notes = new Map() // projectId -> Note[]
  const files = new Map() // projectId -> {file, uploadedAtMs}[]
  const outFiles = new Map() // projectId -> OutFile[]
  let executionConsent = scenario.executionConsent === true
  let uiPrefs = { language: scenario.language, sidebarCollapsed: false, historyCollapsed: false }
  let nextId = 100
  let turnIndex = 0
  let cancelled = false
  const actions = [] // what the person "opened" or "revealed", for assertions

  for (const seed of scenario.notes || []) {
    const list = notes.get(scenario.project.id) || []
    list.push({
      name: seed.name,
      title: seed.title,
      content: seed.content,
      updatedAt: seed.updatedAt || nowIso(),
      size: seed.content.length,
      path: join(scenario.project.notesDir, seed.name),
    })
    notes.set(scenario.project.id, list)
  }
  for (const seed of scenario.files || []) {
    const list = files.get(scenario.project.id) || []
    list.push({
      uploadedAtMs: Date.now() - 3_600_000,
      file: {
        id: nextId++,
        name: seed.name,
        size: seed.size,
        state: 'ready',
        detail: null,
        uploadedAt: new Date(Date.now() - 3_600_000).toISOString(),
        sourcePath: seed.path || null,
        sourceDir: seed.path ? seed.path.split(/[\\/]/).slice(0, -1).join(sep) : null,
        sourceAvailable: true,
      },
    })
    files.set(scenario.project.id, list)
  }

  const projectById = (id) => {
    const project = projects.find((p) => p.id === id)
    if (!project) {
      throw { code: 'project_not_found', message: `demo: no project ${id}` }
    }
    return project
  }
  const projectsState = () => ({ projects, activeId, personalId })

  // ---- callbacks + events (what @tauri-apps/api expects) -------------------
  const callbacks = new Map()
  const listeners = new Map() // event -> Array<{ id, handler }>
  let nextEventId = 1

  function transformCallback(callback, once) {
    const id = Math.floor(Math.random() * 2 ** 31)
    callbacks.set(id, (data) => {
      if (once) {
        callbacks.delete(id)
      }
      return callback && callback(data)
    })
    return id
  }

  function emit(event, payload) {
    for (const { id, handler } of listeners.get(event) || []) {
      const cb = callbacks.get(handler)
      if (cb) {
        cb({ event, id, payload })
      }
    }
  }

  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener(event, eventId) {
      const list = listeners.get(event)
      if (list) {
        listeners.set(
          event,
          list.filter((l) => l.id !== eventId),
        )
      }
    },
  }

  // ---- the scripted assistant turn ----------------------------------------
  async function streamText(text, wordMs) {
    const words = text.split(/(\s+)/)
    for (const word of words) {
      if (cancelled) {
        return
      }
      if (word === '') {
        continue
      }
      emit('agent://text', word)
      if (!/^\s+$/.test(word)) {
        await sleep(wordMs)
      }
    }
  }

  async function runTurn(projectId, agent) {
    const script = (scenario.turns || [])[turnIndex]
    turnIndex += 1
    cancelled = false
    const textEvent = agent ? 'agent://text' : 'chat://token'
    const doneEvent = agent ? 'agent://done' : 'chat://done'
    if (!script) {
      await sleep(400)
      emit(textEvent, 'The demo script has no reply for this message.')
      emit(doneEvent, null)
      return
    }
    await sleep(script.thinkMs ?? 900)
    for (const ev of script.events) {
      if (cancelled) {
        break
      }
      if (ev.kind === 'pause') {
        await sleep(ev.ms)
      } else if (ev.kind === 'step') {
        emit('agent://tool', {
          phase: 'start',
          name: ev.tool,
          summary: ev.start,
          ok: true,
          artifact: null,
        })
        await sleep(ev.ms)
        emit('agent://tool', {
          phase: 'end',
          name: ev.tool,
          summary: ev.end,
          ok: ev.ok !== false,
          artifact: ev.artifact || null,
        })
        if (ev.artifact) {
          const list = outFiles.get(projectId) || []
          list.unshift({
            name: baseName(ev.artifact),
            size: ev.size || 24_576,
            modifiedAt: nowIso(),
            path: ev.artifact,
          })
          outFiles.set(projectId, list)
        }
        await sleep(180)
      } else if (ev.kind === 'text') {
        if (agent) {
          await streamText(ev.text, ev.wordMs ?? scenario.wordMs ?? 28)
        } else {
          for (const word of ev.text.split(/(\s+)/)) {
            if (word !== '') {
              emit('chat://token', word)
              if (!/^\s+$/.test(word)) {
                await sleep(ev.wordMs ?? scenario.wordMs ?? 28)
              }
            }
          }
        }
      }
    }
    emit(doneEvent, null)
  }

  // ---- files: a "sent" file becomes ready a few seconds after upload --------
  function fileView(entry) {
    const age = Date.now() - entry.uploadedAtMs
    const state =
      entry.file.state === 'ready' || age > 4_000
        ? 'ready'
        : age > 1_500
          ? 'indexing'
          : entry.file.state
    return { ...entry.file, state }
  }

  // ---- commands ------------------------------------------------------------
  const commands = {
    get_status: () => ({
      paired: true,
      apiBaseUrl: scenario.apiBaseUrl,
      deviceId: 7,
      keyBackend: scenario.keyBackend || 'macos-keychain',
      keyIsPlaintext: false,
    }),
    default_device_name: () => scenario.deviceName || 'Demo computer',
    get_ui_prefs: () => uiPrefs,
    set_ui_prefs: ({ prefs }) => {
      uiPrefs = prefs
      return uiPrefs
    },
    get_poll_status: () => ({
      running: true,
      lastCheckinUnix: scenario.lastCheckinUnix || Math.floor(Date.now() / 1000) - 45,
      nextCallAt: Math.floor(Date.now() / 1000) + 60,
      jobsWaiting: 0,
      lastError: null,
      plaintextBlocked: false,
    }),
    get_autostart: () => true,
    set_autostart: ({ enabled }) => enabled,
    get_storage_info: () => ({
      projectsDir: scenario.paths.projectsDir,
      outboxDir: scenario.paths.outboxDir,
      skillsDir: scenario.paths.skillsDir,
      configDir: scenario.paths.configDir,
    }),
    get_debug_log: () => ({ enabled: false, path: join(scenario.paths.configDir, 'logs') }),
    get_filesystem_policy: () => ({
      read: [scenario.project.projectDir],
      outbox: scenario.paths.outboxDir,
      deny: [],
      maxFileBytes: 26_214_400,
    }),
    skills_dir: () => scenario.paths.skillsDir,
    run_doctor: () => scenario.doctor || [],

    list_projects: () => projectsState(),
    get_project: ({ id }) => projectById(id),
    get_active_project: () => projectById(activeId),
    set_active_project: ({ id }) => {
      projectById(id)
      activeId = id
      return projectsState()
    },
    apply_default_models: ({ projectId }) => {
      const project = projectById(projectId)
      const defaults = scenario.defaultModels || {}
      for (const slot of ['chat', 'voice', 'speak', 'vision', 'image', 'video', 'embed', 'docs']) {
        if (!project.models[slot] && defaults[slot]) {
          project.models[slot] = defaults[slot]
        }
      }
      return project
    },
    update_project: ({ id, patch }) => {
      const project = projectById(id)
      Object.assign(project, patch, { updatedAt: nowIso() })
      return project
    },
    create_project: ({ name }) => {
      const id = `demo-${nextId++}`
      const project = {
        ...scenario.project,
        id,
        slug: id,
        name,
        enabledSkills: [],
        knowledgeFolder: `DESKTOP:${id}`,
        createdAt: nowIso(),
        updatedAt: nowIso(),
      }
      projects.push(project)
      return project
    },
    delete_project: ({ id }) => {
      const idx = projects.findIndex((p) => p.id === id)
      if (idx >= 0) {
        projects.splice(idx, 1)
      }
      activeId = projects[0].id
      return projectsState()
    },

    list_skills: () =>
      Object.entries(SKILL_CATALOG).map(([name, description]) => ({
        name,
        description,
        dir: join(scenario.paths.skillsDir, name),
        bundled: true,
        enabled: (scenario.skills || Object.keys(SKILL_CATALOG)).includes(name),
        source: 'bundled',
        license: 'Apache-2.0',
        compatibilityWarning: false,
        allowUnattended: false,
        blocked: false,
        blockedReason: null,
        version: '1.0.0',
        url: null,
        sha: null,
        needsPython: name !== 'hello-files',
        needsNode: false,
        needsLibreoffice: false,
        pythonImports: [],
      })),
    set_skill_enabled: () => commands.list_skills(),
    set_skill_unattended: () => commands.list_skills(),
    get_execution_consent: () => executionConsent,
    set_execution_consent: () => {
      executionConsent = true
      return null
    },
    get_studio_tiles: () => scenario.studioTiles || [],
    set_studio_tiles: ({ tiles }) => tiles,
    list_assistants: () => scenario.assistants || [],

    list_chats: ({ projectId }) =>
      (chats.get(projectId) || [])
        .map((thread) => ({
          id: thread.id,
          projectId,
          title: thread.title,
          createdAt: thread.createdAt,
          updatedAt: thread.updatedAt,
          messageCount: thread.messages.length,
          assistantId: thread.assistantId,
        }))
        .sort((a, b) => (a.updatedAt < b.updatedAt ? 1 : -1)),
    new_chat: ({ projectId }) => ({
      id: `chat-${nextId++}`,
      projectId,
      title: '',
      createdAt: nowIso(),
      updatedAt: nowIso(),
      assistantId: null,
      messages: [],
    }),
    save_chat: ({ thread }) => {
      const list = chats.get(thread.projectId) || []
      const first = thread.messages.find((m) => m.role === 'user')
      const title = first ? first.content.slice(0, 48) : thread.title
      const saved = { ...thread, title, updatedAt: nowIso() }
      const idx = list.findIndex((t) => t.id === thread.id)
      if (idx >= 0) {
        list[idx] = saved
      } else {
        list.push(saved)
      }
      chats.set(thread.projectId, list)
      return null
    },
    load_chat: ({ projectId, chatId }) => {
      const thread = (chats.get(projectId) || []).find((t) => t.id === chatId)
      if (!thread) {
        throw { code: 'project_not_found', message: 'demo: no such chat' }
      }
      return thread
    },
    delete_chat: ({ projectId, chatId }) => {
      chats.set(
        projectId,
        (chats.get(projectId) || []).filter((t) => t.id !== chatId),
      )
      return null
    },

    list_notes: ({ projectId, query }) => {
      const needle = (query || '').toLowerCase()
      return (notes.get(projectId) || [])
        .filter((n) => !needle || n.title.toLowerCase().includes(needle))
        .map(({ name, title, updatedAt, size }) => ({ name, title, updatedAt, size }))
        .sort((a, b) => (a.updatedAt < b.updatedAt ? 1 : -1))
    },
    create_note: ({ projectId }) => {
      const project = projectById(projectId)
      const name = `note-${nextId++}.md`
      const note = {
        name,
        title: '',
        content: '',
        updatedAt: nowIso(),
        size: 0,
        path: join(project.notesDir, name),
      }
      const list = notes.get(projectId) || []
      list.push(note)
      notes.set(projectId, list)
      return { name, title: note.title, updatedAt: note.updatedAt, content: '', path: note.path }
    },
    read_note: ({ projectId, name }) => {
      const note = (notes.get(projectId) || []).find((n) => n.name === name)
      if (!note) {
        throw { code: 'path_missing', message: 'demo: no such note' }
      }
      return {
        name,
        title: note.title,
        updatedAt: note.updatedAt,
        content: note.content,
        path: note.path,
      }
    },
    write_note: ({ projectId, name, content }) => {
      const note = (notes.get(projectId) || []).find((n) => n.name === name)
      if (!note) {
        throw { code: 'path_missing', message: 'demo: no such note' }
      }
      const heading = /^#\s+(.+)$/m.exec(content)
      note.content = content
      note.title = heading ? heading[1].trim() : content.split('\n')[0].slice(0, 80)
      note.updatedAt = nowIso()
      note.size = content.length
      return { name, title: note.title, updatedAt: note.updatedAt, size: note.size }
    },
    delete_note: ({ projectId, name }) => {
      notes.set(
        projectId,
        (notes.get(projectId) || []).filter((n) => n.name !== name),
      )
      return null
    },

    list_project_files: ({ projectId }) =>
      (files.get(projectId) || [])
        .map(fileView)
        .sort((a, b) => (a.uploadedAt < b.uploadedAt ? 1 : -1)),
    upload_project_file: async ({ projectId, path }) => {
      await sleep(scenario.uploadMs ?? 700)
      const seed = (scenario.pickFiles || []).find((f) => f.path === path)
      const entry = {
        uploadedAtMs: Date.now(),
        file: {
          id: nextId++,
          name: baseName(path),
          size: seed ? seed.size : 48_000,
          state: 'sent',
          detail: null,
          uploadedAt: nowIso(),
          sourcePath: path,
          sourceDir: path.split(/[\\/]/).slice(0, -1).join(sep),
          sourceAvailable: true,
        },
      }
      const list = files.get(projectId) || []
      list.push(entry)
      files.set(projectId, list)
      return fileView(entry)
    },
    delete_project_file: ({ projectId, fileId }) => {
      files.set(
        projectId,
        (files.get(projectId) || []).filter((e) => e.file.id !== fileId),
      )
      return null
    },
    list_out_files: ({ projectId }) => outFiles.get(projectId) || [],

    classify_generation: () => null,
    send_agent_chat: async ({ projectId }) => {
      await runTurn(projectId, true)
      return null
    },
    send_chat: async ({ projectId }) => {
      await runTurn(projectId, false)
      return null
    },
    cancel_chat: () => {
      cancelled = true
      return null
    },
    save_text_artifact: ({ projectId, name }) => {
      const project = projectById(projectId)
      const path = join(project.outDir, `${name.replace(/[^\w-]+/g, '-')}.md`)
      return { path, name: baseName(path), kind: 'document', fileId: null }
    },

    open_path: ({ path }) => {
      actions.push({ kind: 'open', path })
      return null
    },
    reveal_path: ({ path }) => {
      actions.push({ kind: 'reveal', path })
      return null
    },
    open_url: ({ url }) => {
      actions.push({ kind: 'url', url })
      return null
    },
    get_model_catalog: () => ({
      catalog: { slots: {}, sources: {}, catalogMissing: true },
      rebound: false,
    }),

    'plugin:dialog|open': ({ options }) => {
      const picks = (scenario.pickFiles || []).map((f) => f.path)
      if (options && options.directory) {
        return scenario.pickFolder || null
      }
      if (options && options.multiple) {
        return picks
      }
      return picks[0] || null
    },
    'plugin:event|listen': ({ event, handler }) => {
      const id = nextEventId++
      const list = listeners.get(event) || []
      list.push({ id, handler })
      listeners.set(event, list)
      return id
    },
    'plugin:event|unlisten': ({ event, eventId }) => {
      window.__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener(event, eventId)
      return null
    },
    'plugin:event|emit': ({ event, payload }) => {
      emit(event, payload)
      return null
    },
  }

  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { label: 'main', windowLabel: 'main' },
      windows: [{ label: 'main' }],
      webviews: [{ label: 'main', windowLabel: 'main' }],
    },
    transformCallback,
    unregisterCallback(id) {
      callbacks.delete(id)
    },
    convertFileSrc(path) {
      return `demo-asset://${encodeURIComponent(path)}`
    },
    async invoke(cmd, args) {
      const handler = commands[cmd]
      if (!handler) {
        throw { code: 'unexpected', message: `demo: unhandled command ${cmd}` }
      }
      return handler(args || {})
    },
  }

  // Test hooks: the spec reads these to assert the walk really happened.
  window.__SYNAPLAN_DEMO_STATE__ = {
    actions,
    outFiles: () => outFiles.get(activeId) || [],
    files: () => (files.get(activeId) || []).map(fileView),
    notes: () => notes.get(activeId) || [],
    executionConsent: () => executionConsent,
  }
})()
