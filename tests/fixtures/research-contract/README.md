# Research wire fixtures (research_protocol 1)

Desktop copy of the research HTTP contract. The Synaplan tree has no
`app:research:fixtures` command and no `_devextras/testing/research/fixtures/`
directory, so these files were written from the serialisers that ship in
synaplan `569fc1b6c`:

- `ResearchCapabilities::describe`
- `ResearchRunView::accepted` and `ResearchRunView::one`
- `ResearchEventStream::body`
- `ResearchController` input and follow-up bodies
- `ResearchOutcome::text`

A byte change here is a `research_protocol: 2` decision. The desktop client
talks to `POST /v1/research` with an API key, so `start-request.json` omits
`chat_id` (the API-key route rejects it). The session route still requires
`chat_id`; that field is not part of this desktop request.

`report.citations` is optional. The live state view sends `markdown` and
`version`. When `citations` is present the client uses it; otherwise it
builds chips from `[n]` markers and the sources list.
