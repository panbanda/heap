# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1](https://github.com/panbanda/heap/compare/heap-v0.1.0...heap-v0.1.1) (2026-01-12)


### Features

* add macOS app bundle support ([954bdeb](https://github.com/panbanda/heap/commit/954bdebd5e800046f8f342e9567e9ebdf3cb4d29))
* **embedding:** implement Candle-based embedding engine ([fce0ba5](https://github.com/panbanda/heap/commit/fce0ba50021eac1b2fc21790f976883a2056d876))
* **providers:** implement Gmail API provider with OAuth ([9bb4802](https://github.com/panbanda/heap/commit/9bb48026079c35b291ba11826753f9d25b4d9ec4))
* **providers:** implement IMAP/SMTP email provider ([38e2b49](https://github.com/panbanda/heap/commit/38e2b4941cc90f2fda54e76b1b3f811fc7533080))
* **services:** add ContactService for contact management ([bbe1b55](https://github.com/panbanda/heap/commit/bbe1b550e09ee2297089a920cba19f1dba97cb3d))
* **services:** add LabelService for label/folder management ([e2c2f95](https://github.com/panbanda/heap/commit/e2c2f95579e453ef74ca6769378da6d0fc50348d))
* **services:** add remaining service layer implementations ([3627d5f](https://github.com/panbanda/heap/commit/3627d5fe77ca837d1aaef52cfd2f8e9c9e39e715))
* **services:** add SnoozeService for email snooze functionality ([358cdc1](https://github.com/panbanda/heap/commit/358cdc1de242bb09c2b323c21a0682f2ffccbe2b))
* **services:** add TelemetryService for local usage statistics ([a38272d](https://github.com/panbanda/heap/commit/a38272dbcd3c15f27a64153e2005500cec0fbbb9))
* **ui:** add search, notifications, smart views, stats, settings, undo ([eb84e7f](https://github.com/panbanda/heap/commit/eb84e7f4d3725923f0e1324fb946fd07e8f4b2d7))
* **ui:** add UI components, integration tests, and accessibility ([1fc5a74](https://github.com/panbanda/heap/commit/1fc5a74225aba2fbc92c0ccca7cbc3a1bbfc751d))


### Bug Fixes

* labels tests FK constraint and update release to release-please ([317a5b0](https://github.com/panbanda/heap/commit/317a5b06a75852d4af0fb571daaf33b017c29339))
* resolve clippy and rustdoc warnings ([3206b9d](https://github.com/panbanda/heap/commit/3206b9d315df2fbed7cf2d09136e2e2b50bd8658))


### Refactoring

* rename project from margin to The Heap ([192bac3](https://github.com/panbanda/heap/commit/192bac3363d5c35c336e954492a3fd02aba51e90))


### CI/CD

* add GitHub Actions CI and release workflows ([3f45ef4](https://github.com/panbanda/heap/commit/3f45ef435b3530e0d85a3bdea382a32c1a402416))
* add missing X11/XCB system dependencies ([#3](https://github.com/panbanda/heap/issues/3)) ([dddfaed](https://github.com/panbanda/heap/commit/dddfaed9164ce4a8839f13aea5769f8a730caf84))

## [Unreleased]

### Added
- Initial project structure following SPARC methodology
- Domain layer types (Email, Thread, Account, Label, Contact, Screener)
- Storage layer with SQLite schema and queries
- Email provider trait with Gmail and IMAP stub implementations
- AI provider trait with OpenAI, Anthropic, and Ollama implementations
- Service layer (EmailService, AiService, SyncService)
- Configuration system with appearance, AI, notification, sync, and privacy settings
- Embedding engine for semantic search
- Basic gpui application scaffold
- GitHub Actions CI workflow with 95% coverage requirement
- Lefthook pre-commit hooks for formatting and linting
