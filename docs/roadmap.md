# Taskion Roadmap

## MVP Phase - Minimum Viable Product

### Goal

Define the minimum viable product (MVP) for Taskion to guide initial development and prevent scope creep.

### Must-have Features

- Read courses from Notion
- Read todos from Notion
- Create and update todos locally
- Mark todos as completed
- Bidirectional synchronization (Notion ↔ Local)

### Nice-to-have Features

- Manual sync trigger button
- Error state indicators
- Apple Watch support

### Out of Scope (for MVP)

- Multi-user support
- Cloud backend
- Collaboration features
- Advanced filtering and search
- Custom notifications

## Development Phases

### Phase 1: Foundation
- [ ] Set up backend (Rust with Actix-web)
- [ ] Implement local SQLite database schema
- [ ] Create data model (Course, Todo)

### Phase 2: Notion Integration
- [ ] Implement Notion API client
- [ ] Read courses from Notion
- [ ] Read todos from Notion
- [ ] Implement bidirectional synchronization

### Phase 3: Local Operations
- [ ] Implement local todo create/update/delete
- [ ] Mark todos as completed
- [ ] Implement sync state management

### Phase 4: Frontend
- [ ] Build client UI (display tasks)
- [ ] Implement local changes interface
- [ ] Add manual sync trigger

### Phase 5: Polish
- [ ] Error handling and indicators
- [ ] Testing and bug fixes
- [ ] Documentation

## Success Criteria

- [x] MVP scope documented
- [ ] Backend API functional
- [ ] Bidirectional sync working
- [ ] Basic UI operational
- [ ] End-to-end workflow tested

## Related Documents

- [Architecture Overview](./architecture.md) - System design and components
- [Data Model](./data-model.md) - Database schema and data structures
- [Synchronization Strategy](./synchronization.md) - Sync mechanism and conflict resolution
